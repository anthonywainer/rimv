//! Local, source-preserving speech-to-text primvtives. The public interface is
//! independent of whisper.cpp so another local backend can replace it later.
mod audio;
#[cfg(feature = "parakeet")]
mod parakeet;
mod realtime;
mod rolling;
mod vad;
#[cfg(feature = "whisper")]
mod whisper;
mod worker;

pub use audio::{Mono16k, Preprocessor};
#[cfg(feature = "parakeet")]
pub use parakeet::{ParakeetEngine, ParakeetModelLayout};
pub use rolling::{RollingWindow, deduplicate_overlap};
#[cfg(feature = "silero-vad")]
pub use vad::SileroVad;
pub use vad::{SpeechSegmenter, Utterance, VadConfig, VoiceActivityGate};
#[cfg(feature = "whisper")]
pub use whisper::WhisperEngine;
pub use worker::{SpeechConfig, SpeechEvent, SpeechMetrics, SpeechToTextEngine, SpeechWorker};

use engine_protocol::AudioSource;
use thiserror::Error;

pub const ASR_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AsrBackendKind {
    #[default]
    Parakeet,
    Whisper,
}

#[derive(Debug, Clone, Error)]
pub enum SpeechError {
    #[error("model path is not configured")]
    ModelMissing,
    #[error("failed to load model: {0}")]
    ModelLoad(String),
    #[error("audio preprocessing failed: {0}")]
    Preprocess(String),
    #[error("speech inference failed: {0}")]
    Inference(String),
    #[error("voice activity detection failed: {0}")]
    Vad(String),
    #[error("speech worker is closed")]
    Closed,
}

pub type Result<T> = std::result::Result<T, SpeechError>;

/// Backend-neutral description exposed to runtime clients.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AsrBackendInfo {
    pub backend_id: String,
    pub backend_name: String,
    pub model_id: String,
    pub model_name: String,
    pub capabilities: AsrCapabilities,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AsrCapabilities {
    pub supports_incremental_audio: bool,
    pub supports_partial_results: bool,
    pub supports_word_timestamps: bool,
    pub supports_language_detection: bool,
    pub supports_true_streaming: bool,
}
pub use engine_protocol::TranscriptUpdate;
pub use realtime::TranscriptStabilizer;

/// A generic backend loading boundary. Runtime code depends only on this
/// function and `SpeechToTextEngine`, never a concrete ASR implementation.
pub fn load_configured_backend(config: SpeechConfig) -> Result<Box<dyn SpeechToTextEngine>> {
    match config.backend {
        AsrBackendKind::Parakeet => {
            #[cfg(feature = "parakeet")]
            {
                Ok(Box::new(ParakeetEngine::load(config)?))
            }
            #[cfg(not(feature = "parakeet"))]
            Err(SpeechError::ModelLoad(
                "Parakeet backend is not compiled".into(),
            ))
        }
        AsrBackendKind::Whisper => {
            #[cfg(feature = "whisper")]
            {
                Ok(Box::new(WhisperEngine::load(config)?))
            }
            #[cfg(not(feature = "whisper"))]
            Err(SpeechError::ModelLoad(
                "Whisper backend is not compiled".into(),
            ))
        }
    }
}

/// Final source-preserving output. Source identifies an input track, never a
/// human speaker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpeechSegment {
    pub source: AudioSource,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Deterministic backend for hardware-free runtime tests. It never loads a
/// model and keeps backend behavior outside capture callbacks.
#[derive(Debug, Default)]
pub struct MockAsrBackend {
    pub responses: std::collections::VecDeque<Result<Vec<SpeechSegment>>>,
}
impl SpeechToTextEngine for MockAsrBackend {
    fn info(&self) -> AsrBackendInfo {
        AsrBackendInfo {
            backend_id: "mock".into(),
            backend_name: "Mock ASR".into(),
            model_id: "mock".into(),
            model_name: "Deterministic mock".into(),
            capabilities: AsrCapabilities {
                supports_incremental_audio: false,
                supports_partial_results: false,
                supports_word_timestamps: false,
                supports_language_detection: false,
                supports_true_streaming: false,
            },
        }
    }

    fn transcribe(
        &mut self,
        _audio_16khz_mono: &[f32],
        _offset_ms: u64,
    ) -> Result<Vec<SpeechSegment>> {
        self.responses.pop_front().unwrap_or_else(|| Ok(Vec::new()))
    }
}

#[cfg(test)]
mod generic_tests {
    use super::*;
    #[test]
    fn mock_backend_is_deterministic() {
        let mut backend = MockAsrBackend::default();
        backend.responses.push_back(Ok(vec![SpeechSegment {
            source: AudioSource::Microphone,
            start_ms: 0,
            end_ms: 1,
            text: "ok".into(),
        }]));
        assert_eq!(backend.transcribe(&[], 0).unwrap()[0].text, "ok");
        assert!(backend.transcribe(&[], 0).unwrap().is_empty());
        assert_eq!(backend.info().backend_id, "mock");
    }

    #[cfg(feature = "parakeet")]
    #[test]
    fn configured_parakeet_reports_a_missing_model_without_native_loading() {
        let error = match load_configured_backend(SpeechConfig {
            backend: AsrBackendKind::Parakeet,
            model_path: None,
            ..Default::default()
        }) {
            Ok(_) => panic!("missing Parakeet model unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(matches!(error, SpeechError::ModelMissing));
    }
}
