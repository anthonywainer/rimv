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
    /// Stable, non-technical copy suitable for normal product UI. `message`
    /// remains the diagnostic detail used by logs and troubleshooting tools.
    #[cfg_attr(feature = "serde", serde(default))]
    pub user_message: String,
    pub message: String,
}

impl EngineError {
    pub fn new(code: EngineErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            source: None,
            user_message: user_message(code, None).to_owned(),
            message: message.into(),
        }
    }

    pub fn for_source(mut self, source: AudioSource) -> Self {
        self.source = Some(source);
        self.user_message = user_message(self.code, Some(source)).to_owned();
        self
    }
}

fn user_message(code: EngineErrorCode, source: Option<AudioSource>) -> &'static str {
    match code {
        EngineErrorCode::PermissionDenied => {
            "RimV does not have permission to access this audio source."
        }
        EngineErrorCode::MicrophoneStartFailed => "Microphone is unavailable.",
        EngineErrorCode::SystemAudioStartFailed => "System audio could not be captured.",
        EngineErrorCode::NoSourceEnabled | EngineErrorCode::NoActiveSource => {
            "All selected sources failed to start. Check your audio permissions and input devices."
        }
        EngineErrorCode::TranscriptionModelMissing
        | EngineErrorCode::TranscriptionModelLoadFailed
        | EngineErrorCode::TranscriptionFailed => "Speech engine could not start.",
        EngineErrorCode::Unsupported => match source {
            Some(AudioSource::Microphone) => "Microphone capture is not supported on this device.",
            Some(AudioSource::System) => "System audio capture is not supported on this device.",
            None => "This feature is not supported on this device.",
        },
        EngineErrorCode::SessionCreationFailed | EngineErrorCode::StorageFailed => {
            "RimV could not prepare recording storage."
        }
        _ => "RimV could not start listening. Try again.",
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_user_copy_from_diagnostic_detail() {
        let error = EngineError::new(
            EngineErrorCode::TranscriptionFailed,
            "Silero VAD model path is not configured",
        );
        assert_eq!(error.user_message, "Speech engine could not start.");
        assert!(error.to_string().contains("Silero VAD model path"));
    }

    #[test]
    fn source_mapping_is_updated_when_source_is_attached() {
        let error = EngineError::new(EngineErrorCode::Unsupported, "backend unavailable")
            .for_source(AudioSource::System);
        assert_eq!(
            error.user_message,
            "System audio capture is not supported on this device."
        );
    }
}
