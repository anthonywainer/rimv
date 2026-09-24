#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum EngineCommand {
    StartCapture,
    StopCapture,
    SetMicrophoneEnabled {
        enabled: bool,
    },
    SetSystemAudioEnabled {
        enabled: bool,
    },
    /// Records a preference only; availability remains false in v0.2.
    SetTranscriptionEnabled {
        enabled: bool,
    },
    SetTranscriptionModel {
        path: String,
    },
    /// Applies to the next transcription session. `None` leaves language
    /// selection to the configured ASR backend.
    SetTranscriptionLanguage {
        language: Option<String>,
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
}
