use crate::{AudioError, Result};
/// The engine exposes only normalized floating-point PCM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    F32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
}
impl AudioFormat {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self> {
        if sample_rate == 0 || channels == 0 {
            return Err(AudioError::InvalidConfiguration(
                "sample rate and channels must be nonzero",
            ));
        }
        Ok(Self {
            sample_rate,
            channels,
            sample_format: SampleFormat::F32,
        })
    }
    pub fn sample_rate(self) -> u32 {
        self.sample_rate
    }
    pub fn channels(self) -> u16 {
        self.channels
    }
    pub fn sample_format(self) -> SampleFormat {
        self.sample_format
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_zero_dimensions() {
        assert!(AudioFormat::new(0, 2).is_err());
        assert!(AudioFormat::new(48000, 0).is_err());
        assert_eq!(AudioFormat::new(48000, 2).unwrap().channels(), 2);
    }
}
