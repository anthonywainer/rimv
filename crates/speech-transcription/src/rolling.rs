use crate::ASR_SAMPLE_RATE;
use std::collections::VecDeque;

/// A fixed rolling window with a forward step. Input is never allowed to grow
/// without bound: after emitting one window the preceding step is discarded.
pub struct RollingWindow {
    samples: VecDeque<f32>,
    start_ms: u64,
    initialized: bool,
    window_samples: usize,
    step_samples: usize,
}
impl RollingWindow {
    pub fn new(window_ms: u64, step_ms: u64) -> Self {
        assert!(window_ms >= step_ms && step_ms > 0);
        Self {
            samples: VecDeque::new(),
            start_ms: 0,
            initialized: false,
            window_samples: (window_ms * ASR_SAMPLE_RATE as u64 / 1000) as usize,
            step_samples: (step_ms * ASR_SAMPLE_RATE as u64 / 1000) as usize,
        }
    }
    pub fn push(&mut self, start_ms: u64, samples: &[f32]) -> Vec<(u64, Vec<f32>)> {
        if !self.initialized {
            self.start_ms = start_ms;
            self.initialized = true;
        }
        self.samples.extend(samples.iter().copied());
        let mut windows = Vec::new();
        while self.samples.len() >= self.window_samples {
            windows.push((
                self.start_ms,
                self.samples
                    .iter()
                    .take(self.window_samples)
                    .copied()
                    .collect(),
            ));
            self.samples.drain(..self.step_samples);
            self.start_ms = self
                .start_ms
                .saturating_add(self.step_samples as u64 * 1000 / ASR_SAMPLE_RATE as u64);
        }
        windows
    }
}

/// Removes a repeated word-prefix caused by overlap. It only compares complete
/// case-insensitive whitespace words, never edits a model's internal text.
pub fn deduplicate_overlap(previous: &str, current: &str) -> String {
    let prior: Vec<_> = previous.split_whitespace().collect();
    let now: Vec<_> = current.split_whitespace().collect();
    let longest = prior.len().min(now.len());
    let overlap = (1..=longest)
        .rev()
        .find(|&n| {
            prior[prior.len() - n..]
                .iter()
                .map(|w| w.to_lowercase())
                .eq(now[..n].iter().map(|w| w.to_lowercase()))
        })
        .unwrap_or(0);
    now[overlap..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_have_global_offsets() {
        let mut rolling = RollingWindow::new(4000, 2000);
        let first = rolling.push(0, &vec![0.0; 64_000]);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, 0);
        let second = rolling.push(4000, &vec![0.0; 32_000]);
        assert_eq!(second[0].0, 2000);
    }
    #[test]
    fn removes_only_word_overlap() {
        assert_eq!(
            deduplicate_overlap("hello rimv", "rimv speaks now"),
            "speaks now"
        );
        assert_eq!(deduplicate_overlap("one two", "three four"), "three four");
    }
}
