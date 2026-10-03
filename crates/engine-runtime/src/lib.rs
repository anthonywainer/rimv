//! Synchronous, UI-independent capture controller. Native streams live on one
//! worker; cloneable handles send bounded commands and subscribe to broadcasts.
pub mod backend;
mod config;
mod controller;
mod event_bus;
mod recording_library;
mod runtime;
mod session;
mod storage;
pub mod websocket;

pub use config::{EngineConfig, SourceSettings, TranscriptionSettings};
pub use engine_protocol::*;
pub use event_bus::{Subscription, SubscriptionError};
pub use recording_library::{
    ExportFormat, RecordingDetails, RecordingLibrary, RecordingSummary, TranscriptLine,
};
pub use runtime::EngineRuntime;
pub use speech_transcription::AsrBackendKind;
#[cfg(all(target_os = "macos", feature = "apple-speech"))]
pub use speech_transcription::{
    AppleSpeechLocaleCatalog, apple_locale, apple_speech_authorized, apple_speech_available,
    apple_speech_default_locale, apple_speech_locale_catalog,
};
pub fn supports_asr_backend(backend: &str) -> bool {
    match backend {
        "parakeet" => speech_transcription::supports_backend(AsrBackendKind::Parakeet),
        "whisper" => speech_transcription::supports_backend(AsrBackendKind::Whisper),
        "native_apple" => speech_transcription::supports_backend(AsrBackendKind::AppleNative),
        "native_windows" => speech_transcription::supports_backend(AsrBackendKind::WindowsNative),
        _ => false,
    }
}
pub fn supports_vad_backend(backend: &str) -> bool {
    backend == "vad" && speech_transcription::supports_vad()
}
pub type Result<T> = std::result::Result<T, EngineError>;

pub(crate) fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
