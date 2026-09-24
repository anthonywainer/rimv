use engine_protocol::{AudioSource, TranscriptUpdate};
use std::{
    collections::HashSet,
    io::{self, Write},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PartialKey {
    source: AudioSource,
    utterance_id: String,
}

#[derive(Debug, Clone)]
struct ActivePartial {
    key: PartialKey,
    text: String,
}

/// Renders finalized transcripts as immutable lines and keeps mutable partials
/// in the terminal rows immediately below them.
pub(crate) struct TranscriptRenderer<W: Write> {
    writer: W,
    interactive_partials: bool,
    terminal_columns: Option<usize>,
    active: Vec<ActivePartial>,
    finalized: HashSet<PartialKey>,
    displayed: String,
    suspended: bool,
}

impl<W: Write> TranscriptRenderer<W> {
    pub(crate) fn new(writer: W, interactive_partials: bool) -> Self {
        Self::with_terminal_columns(writer, interactive_partials, terminal_columns())
    }

    pub(crate) fn with_terminal_columns(
        writer: W,
        interactive_partials: bool,
        terminal_columns: Option<usize>,
    ) -> Self {
        Self {
            writer,
            interactive_partials,
            terminal_columns,
            active: Vec::new(),
            finalized: HashSet::new(),
            displayed: String::new(),
            suspended: false,
        }
    }

    pub(crate) fn update(&mut self, update: TranscriptUpdate) -> io::Result<()> {
        let key = PartialKey {
            source: update.source,
            utterance_id: update.utterance_id,
        };
        if self.finalized.contains(&key) {
            return Ok(());
        }

        if update.is_final {
            self.active.retain(|partial| partial.key != key);
            self.clear_display()?;
            writeln!(
                self.writer,
                "[{} {:>7}-{:>7} ms] {}",
                source_label(update.source),
                update.start_ms,
                update.end_ms,
                update.stable_text
            )?;
            self.finalized.insert(key);
            if self.interactive_partials && !self.active.is_empty() && !self.suspended {
                write!(self.writer, "\r")?;
            }
            return self.redraw();
        }

        if !self.interactive_partials {
            return Ok(());
        }
        let text = hypothesis_text(&update.stable_text, &update.unstable_text);
        if let Some(partial) = self.active.iter_mut().find(|partial| partial.key == key) {
            if partial.text == text {
                return Ok(());
            }
            partial.text = text;
        } else {
            self.active.push(ActivePartial { key, text });
        }
        self.redraw()
    }

    /// Clears and flushes the partial before a complete diagnostic record is
    /// written to the shared terminal, then restores the partial afterward.
    pub(crate) fn diagnostic(&mut self, write: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        let was_suspended = self.suspended;
        if was_suspended {
            self.writer.flush()?;
        } else {
            self.clear_display()?;
            self.writer.flush()?;
            self.suspended = true;
        }

        let diagnostic_result = write();
        let redraw_result = if was_suspended {
            Ok(())
        } else {
            // A raw terminal LF does not imply CR. Restore column zero before
            // putting the partial below the diagnostic record.
            let carriage_result = write!(self.writer, "\r");
            self.suspended = false;
            carriage_result.and_then(|()| self.redraw())
        };
        diagnostic_result.and(redraw_result)
    }

    /// Print a CLI status message while keeping the provisional region below it.
    pub(crate) fn message(&mut self, message: &str) -> io::Result<()> {
        let was_suspended = self.suspended;
        if !was_suspended {
            self.clear_display()?;
            self.suspended = true;
        }
        writeln!(self.writer, "{message}")?;
        if was_suspended {
            self.writer.flush()
        } else {
            write!(self.writer, "\r")?;
            self.suspended = false;
            self.redraw()
        }
    }

    pub(crate) fn finish(&mut self) -> io::Result<()> {
        self.active.clear();
        self.clear_display()?;
        self.writer.flush()
    }

    fn clear_display(&mut self) -> io::Result<()> {
        if self.interactive_partials && !self.displayed.is_empty() {
            self.move_to_prefix(0)?;
            // Erase every physical row explicitly. Some terminal emulators do
            // not reliably apply ED (CSI J) after a pending auto-wrap.
            let rows = self
                .terminal_columns
                .map(|columns| terminal_cursor(&self.displayed, columns, false).row)
                .unwrap_or(0);
            for row in 0..=rows {
                write!(self.writer, "\x1b[2K")?;
                if row != rows {
                    write!(self.writer, "\x1b[1B\r")?;
                }
            }
            if rows != 0 {
                write!(self.writer, "\x1b[{rows}A\r")?;
            }
            self.displayed.clear();
        }
        Ok(())
    }

    fn redraw(&mut self) -> io::Result<()> {
        if !self.interactive_partials || self.suspended {
            return self.writer.flush();
        }

        let desired = self.partial_line();
        if desired == self.displayed {
            return Ok(());
        }

        let prefix_bytes = common_grapheme_prefix(&self.displayed, &desired);
        if prefix_bytes == self.displayed.len() {
            write!(self.writer, "{}", &desired[prefix_bytes..])?;
        } else if self.terminal_columns.is_some() {
            self.move_to_prefix(prefix_bytes)?;
            write!(self.writer, "\x1b[0J{}", &desired[prefix_bytes..])?;
        } else {
            // With unknown dimensions a wrapped suffix cannot be located.
            // Repainting from column zero still leaves one correct display.
            write!(self.writer, "\r\x1b[0J{desired}")?;
        }
        self.displayed = desired;
        self.writer.flush()
    }

    fn partial_line(&self) -> String {
        if self.active.is_empty() {
            return String::new();
        }
        let mut line = String::from("partial ");
        for (index, partial) in self.active.iter().enumerate() {
            if index != 0 {
                line.push_str(" | ");
            }
            line.push('[');
            line.push_str(source_label(partial.key.source));
            line.push_str("] ");
            line.push_str(&partial.text);
        }
        line
    }

    fn move_to_prefix(&mut self, prefix_bytes: usize) -> io::Result<()> {
        let Some(columns) = self.terminal_columns else {
            return Ok(());
        };
        let current = terminal_cursor(&self.displayed, columns, false);
        let target = terminal_cursor(
            &self.displayed[..prefix_bytes],
            columns,
            prefix_bytes < self.displayed.len(),
        );
        debug_assert!(target.row <= current.row);
        let rows_up = current.row.saturating_sub(target.row);
        if rows_up != 0 {
            write!(self.writer, "\x1b[{rows_up}A")?;
        }
        write!(self.writer, "\r")?;
        if target.column != 0 {
            write!(self.writer, "\x1b[{}C", target.column)?;
        }
        Ok(())
    }
}

fn terminal_columns() -> Option<usize> {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(columns), _)| columns as usize)
        .filter(|&columns| columns > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cursor {
    row: usize,
    column: usize,
}

fn terminal_cursor(text: &str, columns: usize, before_following_text: bool) -> Cursor {
    let columns = columns.max(1);
    let mut row: usize = 0;
    let mut column: usize = 0;
    let mut pending_wrap = false;
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme);
        if width == 0 {
            continue;
        }
        if pending_wrap {
            row += 1;
            column = 0;
            pending_wrap = false;
        }
        if column != 0 && column.saturating_add(width) > columns {
            row += 1;
            column = 0;
        }
        column = column.saturating_add(width).min(columns);
        if column == columns {
            column = columns - 1;
            pending_wrap = true;
        }
    }
    if pending_wrap && before_following_text {
        Cursor {
            row: row + 1,
            column: 0,
        }
    } else {
        Cursor { row, column }
    }
}

fn common_grapheme_prefix(left: &str, right: &str) -> usize {
    left.graphemes(true)
        .zip(right.graphemes(true))
        .take_while(|(left, right)| left == right)
        .map(|(grapheme, _)| grapheme.len())
        .sum()
}

fn hypothesis_text(stable: &str, unstable: &str) -> String {
    if stable.is_empty() {
        return unstable.to_owned();
    }
    if unstable.is_empty() {
        return stable.to_owned();
    }
    if stable.ends_with(char::is_whitespace) || unstable.starts_with(char::is_whitespace) {
        format!("{stable}{unstable}")
    } else {
        format!("{stable} {unstable}")
    }
}

pub(crate) fn source_label(source: AudioSource) -> &'static str {
    match source {
        AudioSource::Microphone => "mic",
        AudioSource::System => "system",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn update(
        source: AudioSource,
        id: &str,
        stable: &str,
        unstable: &str,
        is_final: bool,
    ) -> TranscriptUpdate {
        TranscriptUpdate {
            source,
            utterance_id: id.into(),
            start_ms: 10,
            end_ms: 20,
            stable_text: stable.into(),
            unstable_text: unstable.into(),
            is_final,
            language: None,
            confidence: None,
        }
    }

    fn screen(bytes: &[u8], rows: u16, columns: u16) -> String {
        let mut parser = vt100::Parser::new(rows, columns, 100);
        parser.process(bytes);
        parser.screen().contents()
    }

    #[derive(Clone, Default)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl SharedWriter {
        fn bytes(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    #[test]
    fn appending_partial_sends_only_the_new_suffix() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(80));
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "hello", false))
            .unwrap();
        let before = renderer.writer.len();
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "hello world",
                false,
            ))
            .unwrap();

        assert_eq!(&renderer.writer[before..], b" world");
        assert_eq!(
            screen(&renderer.writer, 24, 80),
            "partial [mic] hello world"
        );
    }

    #[test]
    fn correction_and_shrink_replace_only_the_changed_suffix() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(80));
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "hello world",
                false,
            ))
            .unwrap();
        let correction_start = renderer.writer.len();
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "hello there",
                false,
            ))
            .unwrap();
        let shrink_start = renderer.writer.len();
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "hello", false))
            .unwrap();

        let correction = &renderer.writer[correction_start..shrink_start];
        let shrink = &renderer.writer[shrink_start..];
        assert!(correction.ends_with(b"\x1b[0Jthere"));
        assert!(!correction.windows(5).any(|bytes| bytes == b"hello"));
        assert!(shrink.ends_with(b"\x1b[0J"));
        assert_eq!(screen(&renderer.writer, 24, 80), "partial [mic] hello");
    }

    #[test]
    fn append_at_exact_wrap_edge_preserves_one_partial_display() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(20));
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "123456", false))
            .unwrap();
        let before = renderer.writer.len();
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "1234567", false))
            .unwrap();

        assert_eq!(&renderer.writer[before..], b"7");
        // vt100 retains an exact-width row in its pending-wrap state, so the
        // visible contents do not contain a synthetic newline here.
        assert_eq!(screen(&renderer.writer, 6, 20), "partial [mic] 1234567");
    }

    #[test]
    fn unicode_graphemes_use_terminal_cell_widths() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(20));
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "cafe\u{301} 🙂x",
                false,
            ))
            .unwrap();
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "cafe\u{301} 🙂y",
                false,
            ))
            .unwrap();

        assert_eq!(
            screen(&renderer.writer, 8, 20),
            "partial [mic] cafe\u{301} \n🙂y"
        );
        assert_eq!(terminal_cursor("1234567890123456789🙂x", 20, false).row, 1);
    }

    #[test]
    fn final_is_idempotent_and_late_partial_is_ignored() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(80));
        let final_update = update(AudioSource::Microphone, "u1", "confirmed", "", true);
        renderer.update(final_update.clone()).unwrap();
        renderer.update(final_update).unwrap();
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "late partial",
                false,
            ))
            .unwrap();

        let bytes = String::from_utf8(renderer.writer).unwrap();
        assert_eq!(bytes.matches(" ms] confirmed\n").count(), 1);
        assert!(!bytes.contains("late partial"));
    }

    #[test]
    fn consecutive_finals_survive_silence_and_wrapped_partials() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(24));
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "first segment",
                "",
                true,
            ))
            .unwrap();
        let after_silence = renderer.writer.clone();
        // Silence produces no renderer event and therefore no terminal bytes.
        assert_eq!(renderer.writer, after_silence);
        renderer
            .update(update(
                AudioSource::Microphone,
                "u2",
                "",
                "a second segment partial that wraps",
                false,
            ))
            .unwrap();
        renderer
            .update(update(
                AudioSource::Microphone,
                "u2",
                "second segment",
                "",
                true,
            ))
            .unwrap();

        let terminal = screen(&renderer.writer, 12, 24);
        assert!(terminal.contains("first segment"));
        assert!(terminal.contains("second segment"));
        assert!(!terminal.contains("partial"));
    }

    #[test]
    fn diagnostic_clears_flushes_and_restores_partial() {
        let writer = SharedWriter::default();
        let diagnostic_writer = writer.clone();
        let mut renderer =
            TranscriptRenderer::with_terminal_columns(writer.clone(), true, Some(30));
        renderer
            .update(update(
                AudioSource::Microphone,
                "u1",
                "",
                "active partial",
                false,
            ))
            .unwrap();
        renderer
            .diagnostic(move || {
                let mut writer = diagnostic_writer;
                writeln!(writer, "error: diagnostic")
            })
            .unwrap();

        let bytes = writer.bytes();
        let terminal = screen(&bytes, 12, 30);
        assert!(terminal.contains("error: diagnostic"));
        assert!(terminal.ends_with("partial [mic] active partial"));
        assert_eq!(
            String::from_utf8(bytes)
                .unwrap()
                .matches("error: diagnostic\n")
                .count(),
            1
        );
    }

    #[test]
    fn non_tty_output_contains_only_unique_finals() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), false);
        renderer
            .update(update(AudioSource::System, "u1", "", "partial", false))
            .unwrap();
        let final_update = update(AudioSource::System, "u1", "done", "", true);
        renderer.update(final_update.clone()).unwrap();
        renderer.update(final_update).unwrap();
        renderer.finish().unwrap();

        let output = String::from_utf8(renderer.writer).unwrap();
        assert_eq!(output, "[system      10-     20 ms] done\n");
        assert!(!output.contains('\r'));
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn stable_and_unstable_text_preserve_legitimate_repetition() {
        let mut renderer = TranscriptRenderer::with_terminal_columns(Vec::new(), true, Some(80));
        renderer
            .update(update(AudioSource::Microphone, "u1", "go", "go", false))
            .unwrap();
        assert_eq!(screen(&renderer.writer, 4, 80), "partial [mic] go go");
    }
}
