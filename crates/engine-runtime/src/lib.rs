//! Synchronous, UI-independent capture controller. Native streams live on one
//! worker; cloneable handles send bounded commands and subscribe to broadcasts.
pub mod backend;
mod config;
mod controller;
mod event_bus;
mod runtime;
mod session;
mod storage;
pub mod websocket;

pub use config::{EngineConfig, SourceSettings, TranscriptionSettings};
pub use engine_protocol::*;
pub use event_bus::{Subscription, SubscriptionError};
pub use runtime::EngineRuntime;
pub use speech_transcription::AsrBackendKind;
pub type Result<T> = std::result::Result<T, EngineError>;

pub(crate) fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
