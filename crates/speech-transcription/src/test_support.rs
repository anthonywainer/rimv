use crate::{AsrBackendInfo, AsrCapabilities, Result, SpeechSegment, SpeechToTextEngine};
use std::collections::VecDeque;

/// Deterministic backend available to crate tests only.
#[derive(Debug, Default)]
pub(crate) struct MockAsrBackend {
    pub responses: VecDeque<Result<Vec<SpeechSegment>>>,
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

    fn transcribe(&mut self, _: &[f32], _: u64) -> Result<Vec<SpeechSegment>> {
        self.responses.pop_front().unwrap_or_else(|| Ok(Vec::new()))
    }
}
