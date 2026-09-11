use engine_protocol::{AudioSource, TranscriptUpdate};
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartialKey {
    source: AudioSource,
    utterance_id: String,
}

#[derive(Debug, Clone)]
struct ActivePartial {
    key: PartialKey,
    text: String,
}

/// Renders mutable transcript state without changing the transcript event or
/// persistence paths. Interactive output owns one terminal line; redirected
/// output emits finalized transcripts only.
pub(crate) struct TranscriptRenderer<W: Write> {
    writer: W,
    interactive_partials: bool,
    active: Vec<ActivePartial>,
    line_visible: bool,
}

impl<W: Write> TranscriptRenderer<W> {
    pub(crate) fn new(writer: W, interactive_partials: bool) -> Self {
        Self {
            writer,
            interactive_partials,
            active: Vec::new(),
            line_visible: false,
        }
    }

    pub(crate) fn update(&mut self, update: TranscriptUpdate) -> io::Result<()> {
        let key = PartialKey {
            source: update.source,
            utterance_id: update.utterance_id,
        };
        if update.is_final {
            self.active.retain(|partial| partial.key != key);
            self.clear_line()?;
            writeln!(
                self.writer,
                "[{} {:>7}-{:>7} ms] {}",
                source_label(update.source),
                update.start_ms,
                update.end_ms,
                update.stable_text
            )?;
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

    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        self.clear_line()
    }

    pub(crate) fn resume(&mut self) -> io::Result<()> {
        self.redraw()
    }

    pub(crate) fn finish(&mut self) -> io::Result<()> {
        self.active.clear();
        self.clear_line()?;
        self.writer.flush()
    }

    fn clear_line(&mut self) -> io::Result<()> {
        if self.interactive_partials && self.line_visible {
            write!(self.writer, "\r\x1b[2K")?;
            self.line_visible = false;
        }
        Ok(())
    }

    fn redraw(&mut self) -> io::Result<()> {
        if !self.interactive_partials || self.active.is_empty() {
            return self.writer.flush();
        }
        self.clear_line()?;
        write!(self.writer, "\rpartial ")?;
        for (index, partial) in self.active.iter().enumerate() {
            if index != 0 {
                write!(self.writer, " | ")?;
            }
            write!(
                self.writer,
                "[{}] {}",
                source_label(partial.key.source),
                partial.text
            )?;
        }
        self.line_visible = true;
        self.writer.flush()
    }
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

    #[test]
    fn cumulative_partials_redraw_one_interactive_line() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), true);
        renderer
            .update(update(AudioSource::System, "u1", "", "text1", false))
            .unwrap();
        renderer
            .update(update(AudioSource::System, "u1", "text1", "text2", false))
            .unwrap();
        renderer
            .update(update(
                AudioSource::System,
                "u1",
                "text1 text2",
                "text",
                false,
            ))
            .unwrap();
        let output = String::from_utf8(renderer.writer).unwrap();
        assert!(!output.contains('\n'));
        assert!(output.ends_with("\rpartial [system] text1 text2 text"));
    }

    #[test]
    fn final_clears_partial_and_prints_once() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), true);
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "hello", false))
            .unwrap();
        renderer
            .update(update(AudioSource::Microphone, "u1", "hello", "", true))
            .unwrap();
        let output = String::from_utf8(renderer.writer).unwrap();
        assert!(output.ends_with("\r\x1b[2K[mic      10-     20 ms] hello\n"));
        assert_eq!(output.matches(" ms] hello\n").count(), 1);
    }

    #[test]
    fn legitimate_repeated_words_are_preserved() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), true);
        renderer
            .update(update(AudioSource::Microphone, "u1", "go", "go", false))
            .unwrap();
        let output = String::from_utf8(renderer.writer).unwrap();
        assert!(output.ends_with("[mic] go go"));
    }

    #[test]
    fn source_and_utterance_state_remain_independent() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), true);
        renderer
            .update(update(AudioSource::Microphone, "mic-1", "", "one", false))
            .unwrap();
        renderer
            .update(update(AudioSource::System, "sys-1", "", "two", false))
            .unwrap();
        renderer
            .update(update(AudioSource::Microphone, "mic-1", "one", "", true))
            .unwrap();
        assert_eq!(renderer.active.len(), 1);
        assert_eq!(renderer.active[0].key.source, AudioSource::System);
        let output = String::from_utf8(renderer.writer).unwrap();
        assert!(output.ends_with("\rpartial [system] two"));
    }

    #[test]
    fn shutdown_clears_interactive_state() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), true);
        renderer
            .update(update(AudioSource::Microphone, "u1", "", "hello", false))
            .unwrap();
        renderer.finish().unwrap();
        assert!(renderer.active.is_empty());
        let output = String::from_utf8(renderer.writer).unwrap();
        assert!(output.ends_with("\r\x1b[2K"));
    }

    #[test]
    fn non_tty_output_contains_finals_without_control_sequences() {
        let mut renderer = TranscriptRenderer::new(Vec::new(), false);
        renderer
            .update(update(AudioSource::System, "u1", "", "partial", false))
            .unwrap();
        renderer
            .update(update(AudioSource::System, "u1", "done", "", true))
            .unwrap();
        renderer.finish().unwrap();
        let output = String::from_utf8(renderer.writer).unwrap();
        assert_eq!(output, "[system      10-     20 ms] done\n");
        assert!(!output.contains('\r'));
        assert!(!output.contains('\x1b'));
    }
}
