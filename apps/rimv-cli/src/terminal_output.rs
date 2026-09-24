use crate::transcript_renderer::TranscriptRenderer;
use engine_protocol::TranscriptUpdate;
use std::{
    io::{self, Write}, sync::{Arc, Mutex}, time::Duration,
};

pub(crate) type SharedTerminal<W> = Arc<Mutex<TranscriptRenderer<W>>>;
pub(crate) const REFRESH_INTERVAL: Duration = Duration::from_millis(150);

/// Coalesce snapshots between UI ticks. Finals bypass the refresh interval.
#[derive(Default)]
pub(crate) struct PendingPartials(Vec<TranscriptUpdate>);

impl PendingPartials {
    pub(crate) fn accept<W: Write>(
        &mut self,
        terminal: &SharedTerminal<W>,
        update: TranscriptUpdate,
    ) -> io::Result<()> {
        self.0
            .retain(|old| old.source != update.source || old.utterance_id != update.utterance_id);
        if update.is_final {
            terminal
                .lock()
                .map_err(|_| io::Error::other("terminal lock poisoned"))?
                .update(update)
        } else {
            self.0.push(update);
            Ok(())
        }
    }

    pub(crate) fn refresh<W: Write>(&mut self, terminal: &SharedTerminal<W>) -> io::Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }
        let mut renderer = terminal
            .lock()
            .map_err(|_| io::Error::other("terminal lock poisoned"))?;
        for update in self.0.drain(..) {
            renderer.update(update)?;
        }
        Ok(())
    }
}

/// tracing formats a record through several write calls. Buffer the record so
/// its complete diagnostic and the partial restoration hold one renderer lock.
pub(crate) struct DiagnosticWriter<W: Write, D: Write> {
    terminal: Option<SharedTerminal<W>>,
    diagnostic: D,
    bytes: Vec<u8>,
}

impl<W: Write, D: Write> DiagnosticWriter<W, D> {
    pub(crate) fn new(terminal: Option<SharedTerminal<W>>, diagnostic: D) -> Self {
        Self {
            terminal,
            diagnostic,
            bytes: Vec::new(),
        }
    }
}

impl<W: Write, D: Write> Write for DiagnosticWriter<W, D> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.bytes.is_empty() {
            return Ok(());
        }
        let bytes = std::mem::take(&mut self.bytes);
        let write = || {
            self.diagnostic.write_all(&bytes)?;
            self.diagnostic.flush()
        };
        if let Some(terminal) = &self.terminal {
            terminal
                .lock()
                .map_err(|_| io::Error::other("terminal lock poisoned"))?
                .diagnostic(write)
        } else {
            let mut write = write;
            write()
        }
    }
}

impl<W: Write, D: Write> Drop for DiagnosticWriter<W, D> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_protocol::AudioSource;

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Buffer {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    fn update(id: &str, text: &str, is_final: bool) -> TranscriptUpdate {
        TranscriptUpdate {
            source: AudioSource::Microphone,
            utterance_id: id.into(),
            start_ms: 0,
            end_ms: 1000,
            stable_text: if is_final { text.into() } else { String::new() },
            unstable_text: if is_final { String::new() } else { text.into() },
            is_final,
            language: None,
            confidence: None,
        }
    }

    #[test]
    fn refresh_coalesces_only_pending_snapshots_and_silence_writes_nothing() {
        let buffer = Buffer::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(buffer.clone(), true)));
        let mut pending = PendingPartials::default();
        pending
            .accept(&terminal, update("u1", "hello", false))
            .unwrap();
        pending
            .accept(&terminal, update("u1", "hello world", false))
            .unwrap();
        assert!(buffer.text().is_empty());
        pending.refresh(&terminal).unwrap();
        let displayed = buffer.text();
        assert!(displayed.contains("hello world"));
        assert_eq!(displayed.matches("hello").count(), 1);
        for _ in 0..20 {
            pending.refresh(&terminal).unwrap();
        }
        assert_eq!(buffer.text(), displayed);
    }

    #[test]
    fn available_asr_partial_is_visible_on_the_first_ui_refresh() {
        let buffer = Buffer::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(buffer.clone(), true)));
        let mut pending = PendingPartials::default();
        // This timestamp represents the instant the decoder has an ASR result
        // and hands its event to the CLI. The next UI refresh must render it;
        // it must not wait for a final or a stable-prefix change.
        let result_available = Instant::now();
        pending
            .accept(&terminal, update("u1", "newly available words", false))
            .unwrap();
        pending.refresh(&terminal).unwrap();
        let visible_at = Instant::now();

        assert!(buffer.text().contains("newly available words"));
        assert!(
            visible_at.duration_since(result_available) <= REFRESH_INTERVAL,
            "ASR result waited {:?} before terminal display",
            visible_at.duration_since(result_available)
        );
    }

    #[test]
    fn finals_bypass_tick_and_cancel_their_pending_partial() {
        let buffer = Buffer::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(buffer.clone(), false)));
        let mut pending = PendingPartials::default();
        pending
            .accept(&terminal, update("u1", "draft", false))
            .unwrap();
        pending
            .accept(&terminal, update("u1", "confirmed", true))
            .unwrap();
        pending
            .accept(&terminal, update("u1", "confirmed", true))
            .unwrap();
        pending.refresh(&terminal).unwrap();
        assert_eq!(buffer.text().matches("confirmed").count(), 1);
        assert!(!buffer.text().contains("draft"));
    }

    #[test]
    fn tracing_record_is_buffered_and_serialized_with_partial_updates() {
        let buffer = Buffer::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(buffer.clone(), true)));
        terminal
            .lock()
            .unwrap()
            .update(update("u1", "hello", false))
            .unwrap();
        let before = buffer.text();
        let mut log = DiagnosticWriter::new(Some(terminal.clone()), buffer.clone());
        write!(log, "diagnostic ").unwrap();
        assert_eq!(buffer.text(), before);
        writeln!(log, "message").unwrap();
        drop(log);
        terminal
            .lock()
            .unwrap()
            .update(update("u1", "hello world", false))
            .unwrap();
        terminal
            .lock()
            .unwrap()
            .update(update("u1", "hello world", true))
            .unwrap();
        let mut screen = vt100::Parser::new(24, 80, 100);
        screen.process(buffer.text().as_bytes());
        let text = screen.screen().contents();
        assert_eq!(text.matches("diagnostic message").count(), 1);
        assert_eq!(text.matches("hello world").count(), 1, "{text:?}");
        assert!(!text.contains("partial"));
    }

    #[test]
    fn concurrent_tracing_records_do_not_corrupt_the_active_region() {
        let buffer = Buffer::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(buffer.clone(), true)));
        let log_terminal = terminal.clone();
        let log_buffer = buffer.clone();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || {
                DiagnosticWriter::new(Some(log_terminal.clone()), log_buffer.clone())
            })
            .finish();
        let dispatch = tracing::Dispatch::new(subscriber);
        terminal
            .lock()
            .unwrap()
            .update(update("u1", "hello", false))
            .unwrap();
        std::thread::scope(|scope| {
            for index in 0..8 {
                let dispatch = dispatch.clone();
                scope.spawn(move || {
                    tracing::dispatcher::with_default(&dispatch, || {
                        tracing::warn!("diagnostic-{index}")
                    });
                });
            }
            for text in ["hello world", "hello world again", "hello world corrected"] {
                terminal
                    .lock()
                    .unwrap()
                    .update(update("u1", text, false))
                    .unwrap();
            }
        });
        terminal
            .lock()
            .unwrap()
            .update(update("u1", "hello world corrected", true))
            .unwrap();
        let mut parser = vt100::Parser::new(40, 100, 100);
        parser.process(buffer.text().as_bytes());
        let screen = parser.screen().contents();
        for index in 0..8 {
            assert_eq!(screen.matches(&format!("diagnostic-{index}")).count(), 1);
        }
        assert_eq!(
            screen.matches("hello world corrected").count(),
            1,
            "{screen:?}"
        );
        assert!(!screen.contains("partial"));
    }
}
