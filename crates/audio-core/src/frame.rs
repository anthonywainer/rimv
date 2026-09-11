use crate::{AudioError, AudioFormat, AudioSourceKind, Result};
use std::time::Duration;
/// Interleaved f32 PCM, approximately -1..1. Timestamp is the first sample's
/// position relative to this capture session, not a shared device clock.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    source: AudioSourceKind,
    timestamp: Duration,
    format: AudioFormat,
    samples: Vec<f32>,
}
impl AudioFrame {
    pub fn new(
        source: AudioSourceKind,
        timestamp: Duration,
        format: AudioFormat,
        samples: Vec<f32>,
    ) -> Result<Self> {
        if samples.is_empty() || !samples.len().is_multiple_of(format.channels() as usize) {
            return Err(AudioError::InvalidFrame(
                "samples must contain complete, nonempty interleaved frames",
            ));
        }
        if samples.iter().any(|s| !s.is_finite()) {
            return Err(AudioError::InvalidFrame("samples must be finite"));
        }
        Ok(Self {
            source,
            timestamp,
            format,
            samples,
        })
    }
    pub fn source(&self) -> AudioSourceKind {
        self.source
    }
    pub fn timestamp(&self) -> Duration {
        self.timestamp
    }
    pub fn format(&self) -> AudioFormat {
        self.format
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    pub fn sample_frames(&self) -> usize {
        self.samples.len() / self.format.channels() as usize
    }
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.sample_frames() as f64 / self.format.sample_rate() as f64)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_interleaving_and_duration() {
        let f = AudioFormat::new(48000, 2).unwrap();
        assert!(AudioFrame::new(AudioSourceKind::System, Duration::ZERO, f, vec![0.0]).is_err());
        assert!(AudioFrame::new(AudioSourceKind::System, Duration::ZERO, f, vec![]).is_err());
        assert!(
            AudioFrame::new(
                AudioSourceKind::System,
                Duration::ZERO,
                f,
                vec![f32::NAN; 2]
            )
            .is_err()
        );
        let a =
            AudioFrame::new(AudioSourceKind::System, Duration::ZERO, f, vec![0.0; 96000]).unwrap();
        assert_eq!(a.duration(), Duration::from_secs(1));
        assert_eq!(a.source(), AudioSourceKind::System);
    }
}
