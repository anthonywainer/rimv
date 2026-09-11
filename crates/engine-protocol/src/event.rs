use crate::{AudioSource, EngineError, EngineSnapshot};

/// A source-preserving text span on the capture session timeline. This is not
/// speaker diarization: microphone and system audio remain separate sources.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TranscriptSegment {
    pub source: AudioSource,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}
/// Backend-neutral transcript state. Stable and unstable text are distinct so
/// clients never need to guess whether a result may change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TranscriptUpdate {
    pub source: AudioSource,
    pub utterance_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub stable_text: String,
    pub unstable_text: String,
    pub is_final: bool,
    pub language: Option<String>,
    pub confidence: Option<String>,
}

/// State changes and UI-rate timer updates carry complete snapshots. Errors
/// also arrive separately so clients can show notifications without diffing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum EngineEvent {
    Snapshot { snapshot: Box<EngineSnapshot> },
    Error { error: EngineError },
    TranscriptPartial { segment: TranscriptSegment },
    TranscriptFinal { segment: TranscriptSegment },
    TranscriptUpdate { update: TranscriptUpdate },
    TranscriptionError { error: EngineError },
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    #[test]
    fn snapshot_event_round_trip() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.transcription.asr.inferences = 3;
        snapshot.transcription.asr.coalesced_work = 7;
        snapshot.transcription.asr.average_rtf_milli = 180;
        let event = EngineEvent::Snapshot {
            snapshot: Box::new(snapshot),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<EngineEvent>(&json).unwrap(), event);
    }
    #[test]
    fn transcript_update_json_round_trip() {
        let event = EngineEvent::TranscriptUpdate {
            update: TranscriptUpdate {
                source: AudioSource::Microphone,
                utterance_id: "u1".into(),
                start_ms: 10,
                end_ms: 20,
                stable_text: "hello".into(),
                unstable_text: " world".into(),
                is_final: false,
                language: Some("en".into()),
                confidence: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<EngineEvent>(&json).unwrap(), event);
    }
    #[test]
    fn old_snapshots_default_new_asr_metrics() {
        let snapshot = EngineSnapshot::default();
        let mut json = serde_json::to_value(&snapshot).unwrap();
        json["transcription"].as_object_mut().unwrap().remove("asr");
        assert_eq!(
            serde_json::from_value::<EngineSnapshot>(json).unwrap(),
            snapshot
        );
    }
}
