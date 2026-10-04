use crate::{
    FinalRefiner, ParakeetEngine, Result, SpeechConfig, SpeechSegment, SpeechToTextEngine,
    WhisperEngine,
};

/// Parakeet owns live decoding; the preloaded Whisper context is detached into
/// the bounded refinement worker when the speech worker starts.
pub struct EnhancedEngine {
    parakeet: ParakeetEngine,
    whisper: Option<WhisperRefiner>,
}

impl EnhancedEngine {
    pub fn load(mut config: SpeechConfig) -> Result<Self> {
        let whisper_path = config.refinement_model_path.take();
        let parakeet = ParakeetEngine::load(config.clone())?;
        let whisper = whisper_path
            .ok_or(crate::SpeechError::ModelMissing)
            .and_then(|path| {
                config.backend = crate::AsrBackendKind::Whisper;
                config.model_path = Some(path);
                WhisperEngine::load(config).map(WhisperRefiner)
            })?;
        Ok(Self {
            parakeet,
            whisper: Some(whisper),
        })
    }
}

impl SpeechToTextEngine for EnhancedEngine {
    fn info(&self) -> crate::AsrBackendInfo {
        let mut info = self.parakeet.info();
        info.backend_id = "enhanced-parakeet-whisper".into();
        info.backend_name = "Enhanced — Parakeet + Whisper".into();
        info.model_name = "Parakeet with Whisper refinement".into();
        info
    }
    fn transcribe(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.parakeet.transcribe(audio, offset_ms)
    }
    fn transcribe_partial(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.parakeet.transcribe_partial(audio, offset_ms)
    }
    fn transcribe_final(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.parakeet.transcribe_final(audio, offset_ms)
    }
    fn take_final_refiner(&mut self) -> Option<Box<dyn FinalRefiner>> {
        self.whisper
            .take()
            .map(|refiner| Box::new(refiner) as Box<dyn FinalRefiner>)
    }
}

struct WhisperRefiner(WhisperEngine);
impl FinalRefiner for WhisperRefiner {
    fn refine(&mut self, audio: &[f32]) -> Result<String> {
        Ok(self
            .0
            .transcribe(audio, 0)?
            .into_iter()
            .map(|segment| segment.text)
            .collect::<Vec<_>>()
            .join(" "))
    }
}
