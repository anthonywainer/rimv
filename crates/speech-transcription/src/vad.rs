use crate::{ASR_SAMPLE_RATE, Result, SpeechError};
use std::{collections::VecDeque, path::PathBuf};

/// Centralized VAD and utterance segmentation settings.
#[derive(Debug, Clone)]
pub struct VadConfig {
    pub model_path: Option<PathBuf>,
    pub threshold: f32,
    pub min_speech_ms: u64,
    pub end_silence_ms: u64,
    pub pre_roll_ms: u64,
    pub speech_pad_ms: u64,
    pub max_utterance_ms: u64,
    pub window_size: usize,
    pub threads: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            model_path: std::env::var_os("RIMV_SILERO_VAD_MODEL").map(PathBuf::from),
            threshold: 0.5,
            min_speech_ms: 250,
            end_silence_ms: 800,
            pre_roll_ms: 200,
            speech_pad_ms: 200,
            max_utterance_ms: 30_000,
            window_size: 512,
            threads: 1,
        }
    }
}

impl VadConfig {
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.threshold)
            || self.min_speech_ms == 0
            || self.end_silence_ms == 0
            || self.max_utterance_ms == 0
            || self.window_size == 0
            || self.threads == 0
        {
            return Err(SpeechError::Vad("invalid VAD configuration".into()));
        }
        Ok(())
    }

    fn samples(&self, milliseconds: u64) -> usize {
        ((milliseconds as u128 * ASR_SAMPLE_RATE as u128) / 1000).min(usize::MAX as u128) as usize
    }
}

/// Small interface used by the source-local segmenter and deterministic tests.
/// Implementations run on the speech worker thread.
pub trait VoiceActivityGate: Send + 'static {
    fn is_speech(&mut self, samples: &[f32]) -> Result<bool>;
    fn flush(&mut self) -> Result<()>;
}

impl<T: VoiceActivityGate + ?Sized> VoiceActivityGate for Box<T> {
    fn is_speech(&mut self, samples: &[f32]) -> Result<bool> {
        (**self).is_speech(samples)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

/// Maintained sherpa-onnx adapter for the supported Silero ONNX model.
#[cfg(feature = "silero-vad")]
pub struct SileroVad {
    detector: sherpa_onnx::VoiceActivityDetector,
    window_size: usize,
}

#[cfg(feature = "silero-vad")]
impl SileroVad {
    pub fn load(config: &VadConfig) -> Result<Self> {
        config.validate()?;
        let path = config
            .model_path
            .as_ref()
            .ok_or_else(|| SpeechError::Vad("Silero VAD model path is not configured".into()))?;
        if !path.is_file() {
            return Err(SpeechError::Vad(format!(
                "Silero VAD model does not exist: {}",
                path.display()
            )));
        }
        let native = sherpa_onnx::VadModelConfig {
            silero_vad: sherpa_onnx::SileroVadModelConfig {
                model: Some(path.to_string_lossy().into_owned()),
                threshold: config.threshold,
                min_silence_duration: config.end_silence_ms as f32 / 1000.0,
                min_speech_duration: config.min_speech_ms as f32 / 1000.0,
                window_size: config.window_size as i32,
                max_speech_duration: config.max_utterance_ms as f32 / 1000.0,
            },
            sample_rate: ASR_SAMPLE_RATE as i32,
            num_threads: config.threads as i32,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let buffer_seconds =
            (config.max_utterance_ms + config.pre_roll_ms + config.speech_pad_ms) as f32 / 1000.0;
        let detector = sherpa_onnx::VoiceActivityDetector::create(&native, buffer_seconds)
            .ok_or_else(|| SpeechError::Vad("failed to create Silero VAD".into()))?;
        Ok(Self {
            detector,
            window_size: config.window_size,
        })
    }

    fn discard_native_segments(&self) {
        while !self.detector.is_empty() {
            self.detector.pop();
        }
    }
}

#[cfg(feature = "silero-vad")]
impl VoiceActivityGate for SileroVad {
    fn is_speech(&mut self, samples: &[f32]) -> Result<bool> {
        for chunk in samples.chunks(self.window_size) {
            self.detector.accept_waveform(chunk);
        }
        let detected = self.detector.detected();
        self.discard_native_segments();
        Ok(detected)
    }

    fn flush(&mut self) -> Result<()> {
        self.detector.flush();
        self.discard_native_segments();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Utterance {
    pub start_ms: u64,
    pub end_ms: u64,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpeechState {
    Silent,
    Speaking {
        start_ms: u64,
        trailing_silence_samples: usize,
    },
}

/// One instance belongs to one audio source. Its VAD state, pre-roll, and
/// active utterance are never shared with another source.
pub struct SpeechSegmenter<V> {
    vad: V,
    config: VadConfig,
    state: SpeechState,
    pre_roll: VecDeque<f32>,
    active: Vec<f32>,
}

impl<V: VoiceActivityGate> SpeechSegmenter<V> {
    pub fn new(vad: V, config: VadConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            vad,
            pre_roll: VecDeque::with_capacity(config.samples(config.pre_roll_ms)),
            active: Vec::with_capacity(config.samples(config.max_utterance_ms)),
            config,
            state: SpeechState::Silent,
        })
    }

    pub fn push(&mut self, start_ms: u64, samples: &[f32]) -> Result<Vec<Utterance>> {
        let speech = self.vad.is_speech(samples)?;
        if self.state == SpeechState::Silent {
            if !speech {
                self.push_pre_roll(samples);
                return Ok(Vec::new());
            }
            let pre_roll_ms = self.pre_roll.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64;
            let utterance_start = start_ms.saturating_sub(pre_roll_ms);
            self.active.extend(self.pre_roll.drain(..));
            self.state = SpeechState::Speaking {
                start_ms: utterance_start,
                trailing_silence_samples: 0,
            };
        }

        self.active.extend_from_slice(samples);
        if let SpeechState::Speaking {
            trailing_silence_samples,
            ..
        } = &mut self.state
        {
            if speech {
                *trailing_silence_samples = 0;
            } else {
                *trailing_silence_samples = trailing_silence_samples.saturating_add(samples.len());
            }
        }

        let mut output = self.split_at_maximum();
        let should_finish = matches!(
            self.state,
            SpeechState::Speaking {
                trailing_silence_samples,
                ..
            } if !speech && trailing_silence_samples >= self.config.samples(self.config.speech_pad_ms)
        );
        if should_finish && let Some(utterance) = self.finish_active() {
            output.push(utterance);
        }
        Ok(output)
    }

    pub fn flush(&mut self) -> Result<Option<Utterance>> {
        self.vad.flush()?;
        Ok(self.finish_active())
    }

    /// Borrow growing audio only while the source is actively speaking.
    pub fn active_audio(&self) -> Option<(u64, &[f32])> {
        match self.state {
            SpeechState::Speaking {
                start_ms,
                trailing_silence_samples: 0,
            } if !self.active.is_empty() => Some((start_ms, &self.active)),
            _ => None,
        }
    }

    pub fn buffered_samples(&self) -> usize {
        self.pre_roll.len() + self.active.len()
    }

    fn push_pre_roll(&mut self, samples: &[f32]) {
        let capacity = self.config.samples(self.config.pre_roll_ms);
        self.pre_roll.extend(samples.iter().copied());
        while self.pre_roll.len() > capacity {
            self.pre_roll.pop_front();
        }
    }

    fn split_at_maximum(&mut self) -> Vec<Utterance> {
        let maximum = self.config.samples(self.config.max_utterance_ms);
        let mut output = Vec::new();
        while self.active.len() >= maximum {
            let SpeechState::Speaking { start_ms, .. } = self.state else {
                break;
            };
            let remainder = self.active.split_off(maximum);
            let samples = std::mem::replace(&mut self.active, remainder);
            let end_ms = start_ms + samples.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64;
            output.push(Utterance {
                start_ms,
                end_ms,
                samples,
            });
            self.state = SpeechState::Speaking {
                start_ms: end_ms,
                trailing_silence_samples: 0,
            };
        }
        output
    }

    fn finish_active(&mut self) -> Option<Utterance> {
        let SpeechState::Speaking { start_ms, .. } = self.state else {
            return None;
        };
        self.state = SpeechState::Silent;
        if self.active.is_empty() {
            return None;
        }
        let samples = std::mem::take(&mut self.active);
        let end_ms = start_ms + samples.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64;
        Some(Utterance {
            start_ms,
            end_ms,
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScriptedVad(VecDeque<bool>);
    impl VoiceActivityGate for ScriptedVad {
        fn is_speech(&mut self, _samples: &[f32]) -> Result<bool> {
            Ok(self.0.pop_front().unwrap_or(false))
        }
        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }

    fn config() -> VadConfig {
        VadConfig {
            pre_roll_ms: 100,
            speech_pad_ms: 100,
            max_utterance_ms: 500,
            ..Default::default()
        }
    }

    fn samples(milliseconds: u64, value: f32) -> Vec<f32> {
        vec![value; (milliseconds * ASR_SAMPLE_RATE as u64 / 1000) as usize]
    }

    #[test]
    fn silence_never_emits_an_utterance() {
        let mut segmenter =
            SpeechSegmenter::new(ScriptedVad(VecDeque::from([false, false, false])), config())
                .unwrap();
        for timestamp in [0, 100, 200] {
            assert!(
                segmenter
                    .push(timestamp, &samples(100, 0.0))
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(segmenter.flush().unwrap().is_none());
    }

    #[test]
    fn pre_roll_and_trailing_padding_are_preserved() {
        let mut segmenter = SpeechSegmenter::new(
            ScriptedVad(VecDeque::from([false, true, true, false])),
            config(),
        )
        .unwrap();
        let mut utterances = Vec::new();
        for (timestamp, value) in [(0, 0.0), (100, 1.0), (200, 1.0), (300, 0.0)] {
            utterances.extend(segmenter.push(timestamp, &samples(100, value)).unwrap());
        }
        assert_eq!(utterances.len(), 1);
        assert_eq!((utterances[0].start_ms, utterances[0].end_ms), (0, 400));
        assert_eq!(utterances[0].samples.len(), samples(400, 0.0).len());
    }

    #[test]
    fn maximum_duration_splits_long_speech_and_bounds_memory() {
        let mut segmenter = SpeechSegmenter::new(
            ScriptedVad(VecDeque::from([true, true, true])),
            VadConfig {
                max_utterance_ms: 200,
                ..config()
            },
        )
        .unwrap();
        let mut utterances = Vec::new();
        for timestamp in [0, 100, 200] {
            utterances.extend(segmenter.push(timestamp, &samples(100, 1.0)).unwrap());
        }
        assert_eq!(utterances.len(), 1);
        assert_eq!((utterances[0].start_ms, utterances[0].end_ms), (0, 200));
        assert!(segmenter.buffered_samples() <= samples(200, 0.0).len());
        let tail = segmenter.flush().unwrap().unwrap();
        assert_eq!((tail.start_ms, tail.end_ms), (200, 300));
    }

    #[test]
    fn stop_flush_returns_unfinished_speech() {
        let mut segmenter =
            SpeechSegmenter::new(ScriptedVad(VecDeque::from([true])), config()).unwrap();
        assert!(segmenter.push(50, &samples(100, 1.0)).unwrap().is_empty());
        let utterance = segmenter.flush().unwrap().unwrap();
        assert_eq!((utterance.start_ms, utterance.end_ms), (50, 150));
    }

    #[test]
    fn source_local_instances_do_not_share_state() {
        let mut microphone =
            SpeechSegmenter::new(ScriptedVad(VecDeque::from([true])), config()).unwrap();
        let mut system =
            SpeechSegmenter::new(ScriptedVad(VecDeque::from([false])), config()).unwrap();
        microphone.push(0, &samples(100, 1.0)).unwrap();
        system.push(0, &samples(100, 0.0)).unwrap();
        assert!(microphone.flush().unwrap().is_some());
        assert!(system.flush().unwrap().is_none());
    }

    #[cfg(feature = "silero-vad")]
    #[test]
    #[ignore = "requires RIMV_SILERO_VAD_MODEL to point to silero_vad.onnx"]
    fn maintained_silero_model_loads_and_processes_silence() {
        let path = std::env::var_os("RIMV_SILERO_VAD_MODEL")
            .expect("RIMV_SILERO_VAD_MODEL must be set for this test");
        let mut vad = SileroVad::load(&VadConfig {
            model_path: Some(path.into()),
            ..Default::default()
        })
        .unwrap();
        assert!(!vad.is_speech(&vec![0.0; 512]).unwrap());
        vad.flush().unwrap();
    }
}
