use crate::TranscriptUpdate;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum EngineCommand {
    StartCapture,
    StopCapture,
    SetMicrophoneEnabled {
        enabled: bool,
    },
    /// Selects a native input endpoint for subsequent microphone capture;
    /// `None` follows the operating-system default input device.
    SetMicrophoneDevice {
        device_id: Option<String>,
    },
    SetSystemAudioEnabled {
        enabled: bool,
    },
    /// Enables or disables ASR for future/current capture sessions.
    SetTranscriptionEnabled {
        enabled: bool,
    },
    SetTranscriptionModel {
        path: String,
    },
    /// Clears the selected ASR model and makes transcription unavailable.
    ClearTranscriptionModel,
    /// Selects the loaded ASR implementation for a future transcription
    /// session. The model path remains a separate setting so native clients
    /// can select catalog entries without exposing backend internals.
    SetTranscriptionBackend {
        backend: String,
    },
    /// Applies to the next transcription session. `None` leaves language
    /// selection to the configured ASR backend.
    SetTranscriptionLanguage {
        language: Option<String>,
    },
    /// Handshake from the Windows host after its local SpeechAudioProvider
    /// streams are prepared. Capture must not start before this is true.
    SetNativeProviderReady {
        ready: bool,
    },
    /// Normalized Windows Native Recognizing/Recognized event. Reuses RimV's
    /// shared partial/final transcript state and recording persistence.
    SubmitNativeTranscript {
        update: TranscriptUpdate,
    },
    GetState,
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    #[test]
    fn command_json_contract() {
        let command = EngineCommand::SetMicrophoneEnabled { enabled: false };
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type":"set_microphone_enabled","enabled":false})
        );
        assert_eq!(
            serde_json::from_value::<EngineCommand>(json).unwrap(),
            command
        );
    }

    #[test]
    fn transcription_language_allows_backend_auto_detection() {
        let command = EngineCommand::SetTranscriptionLanguage { language: None };
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type":"set_transcription_language","language":null})
        );
        assert_eq!(
            serde_json::from_value::<EngineCommand>(json).unwrap(),
            command
        );
    }

    #[test]
    fn microphone_device_selection_has_a_stable_json_contract() {
        let command = EngineCommand::SetMicrophoneDevice {
            device_id: Some("input:USB Microphone:0".into()),
        };
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type":"set_microphone_device","device_id":"input:USB Microphone:0"})
        );
        assert_eq!(
            serde_json::from_value::<EngineCommand>(json).unwrap(),
            command
        );
    }
}
