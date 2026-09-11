use crate::{ASR_SAMPLE_RATE, Result, SpeechError};
use audio_core::AudioFrame;
use rubato::{FftFixedInOut, Resampler};
use std::collections::VecDeque;

/// A timestamped mono 16 kHz block suitable for Whisper.
#[derive(Debug, Clone)]
pub struct Mono16k {
    pub start_ms: u64,
    pub samples: Vec<f32>,
}

/// Stateful mono conversion and sample-rate conversion. One instance is owned
/// per source by the speech worker, preserving stream continuity.
pub struct Preprocessor {
    input_rate: u32,
    resampler: Option<FftFixedInOut<f32>>,
    pending: VecDeque<f32>,
    chunk: usize,
    next_start_ms: Option<u64>,
}
impl Preprocessor {
    pub fn new(input_rate: u32) -> Result<Self> {
        if input_rate == 0 {
            return Err(SpeechError::Preprocess(
                "sample rate must be nonzero".into(),
            ));
        }
        let chunk = 1024;
        let resampler = if input_rate == ASR_SAMPLE_RATE {
            None
        } else {
            Some(
                FftFixedInOut::new(input_rate as usize, ASR_SAMPLE_RATE as usize, chunk, 1)
                    .map_err(|error| SpeechError::Preprocess(error.to_string()))?,
            )
        };
        Ok(Self {
            input_rate,
            resampler,
            pending: VecDeque::new(),
            chunk,
            next_start_ms: None,
        })
    }

    pub fn push(&mut self, frame: &AudioFrame) -> Result<Vec<Mono16k>> {
        if frame.format().sample_rate() != self.input_rate {
            return Err(SpeechError::Preprocess(
                "source sample rate changed during transcription".into(),
            ));
        }
        let channels = frame.format().channels() as usize;
        let frame_ms = frame.timestamp().as_millis().min(u64::MAX as u128) as u64;
        if self.next_start_ms.is_none() {
            self.next_start_ms = Some(frame_ms);
        }
        for interleaved in frame.samples().chunks_exact(channels) {
            let sum: f32 = interleaved.iter().sum();
            self.pending.push_back(sum / channels as f32);
        }
        let mut result = Vec::new();
        while self.pending.len() >= self.input_frames_next() {
            let input_frames = self.input_frames_next();
            let input: Vec<f32> = self.pending.drain(..input_frames).collect();
            let start_ms = self.next_start_ms.unwrap_or(frame_ms);
            let samples = if let Some(resampler) = &mut self.resampler {
                resampler
                    .process(&[input], None)
                    .map_err(|error| SpeechError::Preprocess(error.to_string()))?
                    .into_iter()
                    .next()
                    .unwrap_or_default()
            } else {
                input
            };
            self.next_start_ms = Some(
                start_ms.saturating_add((input_frames as u64 * 1000) / self.input_rate as u64),
            );
            result.push(Mono16k { start_ms, samples });
        }
        Ok(result)
    }
    fn input_frames_next(&self) -> usize {
        self.resampler
            .as_ref()
            .map_or(self.chunk, Resampler::input_frames_next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::{AudioFormat, AudioSourceKind};
    use std::time::Duration;
    fn frame(rate: u32, channels: u16, frames: usize) -> AudioFrame {
        let mut samples = Vec::with_capacity(frames * channels as usize);
        for i in 0..frames {
            for c in 0..channels {
                samples.push(i as f32 + c as f32);
            }
        }
        AudioFrame::new(
            AudioSourceKind::Microphone,
            Duration::ZERO,
            AudioFormat::new(rate, channels).unwrap(),
            samples,
        )
        .unwrap()
    }
    #[test]
    fn mono_averages_channels() {
        let mut p = Preprocessor::new(16_000).unwrap();
        let out = p.push(&frame(16_000, 2, 1024)).unwrap();
        assert_eq!(out[0].samples[1], 1.5);
    }
    #[test]
    fn standard_rates_reach_16k() {
        for rate in [48_000, 44_100, 16_000] {
            let mut p = Preprocessor::new(rate).unwrap();
            let mut total = 0;
            for _ in 0..8 {
                total += p
                    .push(&frame(rate, 1, 1024))
                    .unwrap()
                    .into_iter()
                    .map(|b| b.samples.len())
                    .sum::<usize>();
            }
            assert!(total > 0, "{rate}");
        }
    }
}
