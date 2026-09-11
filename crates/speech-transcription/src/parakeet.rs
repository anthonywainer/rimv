use crate::{
    ASR_SAMPLE_RATE, AsrBackendInfo, AsrCapabilities, Result, SpeechConfig, SpeechError,
    SpeechSegment, SpeechToTextEngine,
};
use engine_protocol::AudioSource;
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};
use std::path::{Path, PathBuf};

pub const PARAKEET_MODEL_ID: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";

/// The artifact layout published and documented by sherpa-onnx for the v3
/// INT8 NeMo transducer. Keeping resolution here prevents filenames from
/// leaking into the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParakeetModelLayout {
    pub directory: PathBuf,
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
}

impl ParakeetModelLayout {
    pub fn discover(directory: &Path) -> Result<Self> {
        if !directory.is_dir() {
            return Err(SpeechError::ModelMissing);
        }
        let layout = Self {
            directory: directory.to_owned(),
            encoder: directory.join("encoder.int8.onnx"),
            decoder: directory.join("decoder.int8.onnx"),
            joiner: directory.join("joiner.int8.onnx"),
            tokens: directory.join("tokens.txt"),
        };
        for artifact in [
            &layout.encoder,
            &layout.decoder,
            &layout.joiner,
            &layout.tokens,
        ] {
            if !artifact.is_file() {
                return Err(SpeechError::ModelLoad(format!(
                    "missing Parakeet model artifact: {}",
                    artifact.display()
                )));
            }
        }
        Ok(layout)
    }
}

/// Offline Parakeet TDT adapter. The recognizer is created once in `load` and
/// reused; each finalized VAD utterance gets only a short-lived stream.
pub struct ParakeetEngine {
    recognizer: OfflineRecognizer,
    layout: ParakeetModelLayout,
}

impl ParakeetEngine {
    pub fn load(config: SpeechConfig) -> Result<Self> {
        let directory = config
            .model_path
            .as_deref()
            .ok_or(SpeechError::ModelMissing)?;
        let layout = ParakeetModelLayout::discover(directory)?;
        if !matches!(config.provider.as_str(), "cpu" | "cuda" | "coreml") {
            return Err(SpeechError::ModelLoad(format!(
                "unsupported sherpa-onnx provider: {}",
                config.provider
            )));
        }
        let mut native = OfflineRecognizerConfig::default();
        native.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(path_string(&layout.encoder)),
            decoder: Some(path_string(&layout.decoder)),
            joiner: Some(path_string(&layout.joiner)),
        };
        native.model_config.tokens = Some(path_string(&layout.tokens));
        native.model_config.num_threads = config.threads.clamp(1, i32::MAX as usize) as i32;
        native.model_config.provider = Some(config.provider);
        native.model_config.model_type = Some("nemo_transducer".into());
        native.decoding_method = Some("greedy_search".into());
        let recognizer = OfflineRecognizer::create(&native)
            .ok_or_else(|| SpeechError::ModelLoad("failed to create Parakeet recognizer".into()))?;
        Ok(Self { recognizer, layout })
    }
}

impl SpeechToTextEngine for ParakeetEngine {
    fn info(&self) -> AsrBackendInfo {
        AsrBackendInfo {
            backend_id: "sherpa-onnx-offline-transducer".into(),
            backend_name: "sherpa-onnx Parakeet".into(),
            model_id: PARAKEET_MODEL_ID.into(),
            model_name: "NVIDIA Parakeet TDT 0.6B v3 INT8".into(),
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
        if audio.is_empty() {
            return Ok(Vec::new());
        }
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(ASR_SAMPLE_RATE as i32, audio);
        self.recognizer.decode(&stream);
        let result = stream
            .get_result()
            .ok_or_else(|| SpeechError::Inference("Parakeet returned no result".into()))?;
        let text = result.text.trim().to_owned();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let duration_ms = audio.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64;
        Ok(vec![SpeechSegment {
            source: AudioSource::Microphone,
            start_ms: offset_ms,
            end_ms: offset_ms.saturating_add(duration_ms),
            text,
        }])
    }
}

impl std::fmt::Debug for ParakeetEngine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParakeetEngine")
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AsrBackendKind;
    use std::{fs::File, path::Path};

    fn create_artifacts(directory: &Path) {
        for filename in [
            "encoder.int8.onnx",
            "decoder.int8.onnx",
            "joiner.int8.onnx",
            "tokens.txt",
        ] {
            File::create(directory.join(filename)).unwrap();
        }
    }

    #[test]
    fn discovers_the_verified_official_artifact_layout() {
        let directory = tempfile::tempdir().unwrap();
        create_artifacts(directory.path());
        let layout = ParakeetModelLayout::discover(directory.path()).unwrap();
        assert_eq!(layout.encoder, directory.path().join("encoder.int8.onnx"));
        assert_eq!(layout.tokens, directory.path().join("tokens.txt"));
    }

    #[test]
    fn missing_artifact_is_reported_before_native_loading() {
        let directory = tempfile::tempdir().unwrap();
        create_artifacts(directory.path());
        std::fs::remove_file(directory.path().join("joiner.int8.onnx")).unwrap();
        let error = ParakeetModelLayout::discover(directory.path()).unwrap_err();
        assert!(error.to_string().contains("joiner.int8.onnx"));
    }

    #[test]
    fn missing_model_directory_is_typed() {
        let error =
            ParakeetModelLayout::discover(Path::new("/missing/rimv-parakeet-model")).unwrap_err();
        assert!(matches!(error, SpeechError::ModelMissing));
    }

    #[test]
    #[ignore = "requires RIMV_PARAKEET_MODEL_DIR and RIMV_PARAKEET_TEST_WAV"]
    fn real_model_loads_once_and_is_reused_for_final_transcription() {
        let model = std::env::var_os("RIMV_PARAKEET_MODEL_DIR")
            .expect("RIMV_PARAKEET_MODEL_DIR must be set");
        let wav =
            std::env::var_os("RIMV_PARAKEET_TEST_WAV").expect("RIMV_PARAKEET_TEST_WAV must be set");
        let wav = PathBuf::from(wav);
        let wave = sherpa_onnx::Wave::read(&wav.to_string_lossy()).expect("read test WAV");
        assert_eq!(wave.sample_rate(), ASR_SAMPLE_RATE as i32);
        let mut engine = ParakeetEngine::load(SpeechConfig {
            backend: AsrBackendKind::Parakeet,
            model_path: Some(model.into()),
            provider: "cpu".into(),
            ..Default::default()
        })
        .unwrap();
        let first = engine.transcribe(wave.samples(), 0).unwrap();
        let second = engine.transcribe(wave.samples(), 0).unwrap();
        assert!(!first.is_empty());
        assert_eq!(first[0].text, second[0].text);
    }
}
