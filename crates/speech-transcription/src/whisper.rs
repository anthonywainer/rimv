use crate::{
    AsrBackendInfo, AsrCapabilities, Result, SpeechConfig, SpeechError, SpeechSegment,
    SpeechToTextEngine,
};
use engine_protocol::AudioSource;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// In-process whisper.cpp backend. The context and its decoder state are
/// created once; `transcribe` never reloads a model.
pub struct WhisperEngine {
    context: WhisperContext,
    config: SpeechConfig,
}
impl WhisperEngine {
    pub fn load(config: SpeechConfig) -> Result<Self> {
        let path = config
            .model_path
            .as_ref()
            .ok_or(SpeechError::ModelMissing)?;
        if !path.is_file() {
            return Err(SpeechError::ModelMissing);
        }
        let mut parameters = WhisperContextParameters::new();
        parameters.use_gpu(config.use_gpu);
        let context = WhisperContext::new_with_params(path.to_string_lossy().as_ref(), parameters)
            .map_err(|e| SpeechError::ModelLoad(e.to_string()))?;
        Ok(Self { context, config })
    }
}
impl SpeechToTextEngine for WhisperEngine {
    fn info(&self) -> AsrBackendInfo {
        let model_name = self
            .config
            .model_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map_or_else(
                || "Whisper model".into(),
                |name| name.to_string_lossy().into(),
            );
        AsrBackendInfo {
            backend_id: "whisper.cpp".into(),
            backend_name: "Whisper".into(),
            model_id: "whisper-local".into(),
            model_name,
            capabilities: AsrCapabilities {
                supports_incremental_audio: false,
                supports_partial_results: false,
                supports_word_timestamps: false,
                supports_language_detection: false,
                supports_true_streaming: false,
            },
        }
    }

    fn transcribe(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        let mut state = self
            .context
            .create_state()
            .map_err(|e| SpeechError::Inference(e.to_string()))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
        params.set_n_threads(self.config.threads.clamp(1, i32::MAX as usize) as i32);
        params.set_language(self.config.language.as_deref());
        params.set_translate(false);
        params.set_no_context(true);
        state
            .full(params, audio)
            .map_err(|e| SpeechError::Inference(e.to_string()))?;
        let count = state.full_n_segments();
        let mut output = Vec::with_capacity(count as usize);
        for index in 0..count {
            let segment = state
                .get_segment(index)
                .ok_or_else(|| SpeechError::Inference("missing Whisper segment".into()))?;
            let text = segment
                .to_str()
                .map_err(|e| SpeechError::Inference(e.to_string()))?
                .trim()
                .to_owned();
            if text.is_empty() {
                continue;
            }
            output.push(SpeechSegment {
                source: AudioSource::Microphone,
                start_ms: offset_ms.saturating_add(segment.start_timestamp().max(0) as u64 * 10),
                end_ms: offset_ms.saturating_add(segment.end_timestamp().max(0) as u64 * 10),
                text,
            });
        }
        Ok(output)
    }
}
