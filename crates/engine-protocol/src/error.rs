use crate::AudioSource;

/// Stable categories for client behavior; message preserves backend details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EngineErrorCode {
    AlreadyRecording,
    NotRecording,
    NoSourceEnabled,
    NoActiveSource,
    MicrophoneStartFailed,
    SystemAudioStartFailed,
    CaptureFailed,
    PermissionDenied,
    Unsupported,
    InvalidConfiguration,
    SessionCreationFailed,
    StorageFailed,
    CommandQueueFull,
    RuntimeClosed,
    SubscriberLimit,
    WorkerFailed,
    TranscriptionModelMissing,
    TranscriptionModelLoadFailed,
    TranscriptionFailed,
    TranscriptionQueueFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EngineError {
    pub code: EngineErrorCode,
    pub source: Option<AudioSource>,
    pub message: String,
}

impl EngineError {
    pub fn new(code: EngineErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            source: None,
            message: message.into(),
        }
    }

    pub fn for_source(mut self, source: AudioSource) -> Self {
        self.source = Some(source);
        self
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for EngineError {}
