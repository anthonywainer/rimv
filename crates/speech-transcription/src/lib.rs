//! Local, source-preserving speech-to-text primvtives. The public interface is
//! independent of whisper.cpp so another local backend can replace it later.
#[cfg(all(target_os = "macos", feature = "apple-speech"))]
mod apple_speech;
mod audio;
#[cfg(feature = "parakeet")]
mod parakeet;
mod realtime;
mod rolling;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod vad;
#[cfg(feature = "whisper")]
mod whisper;
mod worker;

#[cfg(all(target_os = "macos", feature = "apple-speech"))]
pub use apple_speech::{
    AppleSpeechLocaleCatalog, apple_locale, apple_speech_authorized, apple_speech_available,
    apple_speech_locale_catalog, default_locale as apple_speech_default_locale,
};
pub use audio::{Mono16k, Preprocessor};
#[cfg(feature = "parakeet")]
pub use parakeet::{ParakeetEngine, ParakeetModelLayout};
pub use rolling::{RollingWindow, deduplicate_overlap};
#[cfg(test)]
pub(crate) use test_support::MockAsrBackend;
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
    AppleNative,
    /// Windows host bridge using Microsoft.Windows.AI.Speech. Recognition is
    /// hosted by the WinUI process; Rust retains capture and transcript state.
    WindowsNative,
}

/// Reports whether this build includes the requested inference backend.
pub const fn supports_backend(backend: AsrBackendKind) -> bool {
    match backend {
        AsrBackendKind::Parakeet => cfg!(feature = "parakeet"),
        AsrBackendKind::Whisper => cfg!(feature = "whisper"),
        AsrBackendKind::AppleNative => cfg!(all(target_os = "macos", feature = "apple-speech")),
        AsrBackendKind::WindowsNative => cfg!(target_os = "windows"),
    }
}

/// Reports whether the Silero voice-activity detector is compiled in.
pub const fn supports_vad() -> bool {
    cfg!(feature = "silero-vad")
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
        AsrBackendKind::AppleNative => {
            #[cfg(all(target_os = "macos", feature = "apple-speech"))]
            {
                Ok(Box::new(apple_speech::AppleSpeechEngine::new(
                    config.language.as_deref().unwrap_or("system"),
                )?))
            }
            #[cfg(not(all(target_os = "macos", feature = "apple-speech")))]
            Err(SpeechError::ModelLoad(
                "Apple Speech is only available in the macOS app".into(),
            ))
        }
        AsrBackendKind::WindowsNative => Err(SpeechError::ModelLoad(
            "Windows Native Speech is hosted by the Windows app bridge".into(),
        )),
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
