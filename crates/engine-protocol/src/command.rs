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
}
