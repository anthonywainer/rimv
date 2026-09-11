use audio_core::{AudioError, Result};
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub device_id: Option<String>,
    /// An exact preference: unsupported requested values return an error.
    pub preferred_sample_rate: Option<u32>,
    pub preferred_channels: Option<u16>,
    /// Maximum queued callback blocks (not individual samples).
    pub buffer_capacity: usize,
    /// Maximum interleaved samples in a microphone callback block.
    /// Larger blocks are dropped rather than allocating in the callback.
    pub max_samples_per_frame: usize,
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            preferred_sample_rate: None,
            preferred_channels: None,
            buffer_capacity: 32,
            max_samples_per_frame: 32768,
        }
    }
}
impl CaptureConfig {
    pub fn validate(&self) -> Result<()> {
        if self.buffer_capacity == 0 || self.buffer_capacity > 1024 {
            return Err(AudioError::InvalidConfiguration(
                "buffer_capacity must be 1..=1024",
            ));
        }
        if self.max_samples_per_frame == 0 || self.max_samples_per_frame > 1_048_576 {
            return Err(AudioError::InvalidConfiguration(
                "max_samples_per_frame must be 1..=1048576",
            ));
        }
        if (self.buffer_capacity + 1).saturating_mul(self.max_samples_per_frame) > 16_777_216 {
            return Err(AudioError::InvalidConfiguration(
                "buffer pool exceeds 64 MiB",
            ));
        }
        if self.preferred_sample_rate == Some(0) || self.preferred_channels == Some(0) {
            return Err(AudioError::InvalidConfiguration(
                "sample rate and channels must be nonzero",
            ));
        }
        if self.device_id.as_ref().is_some_and(|s| s.is_empty()) {
            return Err(AudioError::InvalidConfiguration(
                "device_id must not be empty",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_config() {
        assert!(CaptureConfig::default().validate().is_ok());
        for c in [
            CaptureConfig {
                buffer_capacity: 0,
                ..Default::default()
            },
            CaptureConfig {
                preferred_sample_rate: Some(0),
                ..Default::default()
            },
            CaptureConfig {
                preferred_channels: Some(0),
                ..Default::default()
            },
            CaptureConfig {
                max_samples_per_frame: 0,
                ..Default::default()
            },
            CaptureConfig {
                buffer_capacity: 1024,
                max_samples_per_frame: 1_048_576,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                c.validate(),
                Err(AudioError::InvalidConfiguration(_))
            ));
        }
    }
}
