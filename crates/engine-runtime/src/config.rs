use crate::{EngineError, EngineErrorCode, Result, SourceConfig};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct SourceSettings {
    pub enabled: bool,
    pub configured: SourceConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub recordings_directory: PathBuf,
    pub microphone: SourceSettings,
    pub system_audio: SourceSettings,
    pub transcription_enabled: bool,
    pub transcription: TranscriptionSettings,
    pub command_capacity: usize,
    pub subscriber_capacity: usize,
    pub max_subscribers: usize,
    /// Snapshot/timer publication rate, independent from native callback rate.
    pub snapshot_interval: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            recordings_directory: PathBuf::from("recordings"),
            microphone: SourceSettings {
                enabled: true,
                ..Default::default()
            },
            system_audio: SourceSettings::default(),
            transcription_enabled: false,
            transcription: TranscriptionSettings::default(),
            command_capacity: 32,
            subscriber_capacity: 64,
            max_subscribers: 32,
            snapshot_interval: Duration::from_millis(500),
        }
    }
}

/// Runtime-only local speech configuration. Models are referenced by path and
/// intentionally never bundled or committed to the repository.
#[derive(Debug, Clone)]
pub struct TranscriptionSettings {
    pub backend: speech_transcription::AsrBackendKind,
    pub model_path: Option<PathBuf>,
    pub language: Option<String>,
    pub threads: usize,
    pub use_gpu: bool,
    pub provider: String,
    pub window_ms: u64,
    pub step_ms: u64,
    pub queue_capacity: usize,
    pub vad: speech_transcription::VadConfig,
}
impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            backend: speech_transcription::AsrBackendKind::Parakeet,
            model_path: std::env::var_os("RIMV_PARAKEET_MODEL_DIR").map(PathBuf::from),
            language: None,
            threads: 4,
            use_gpu: cfg!(target_os = "macos"),
            provider: std::env::var("RIMV_ASR_PROVIDER").unwrap_or_else(|_| "cpu".into()),
            window_ms: 6_000,
            step_ms: 3_000,
            queue_capacity: 64,
            vad: speech_transcription::VadConfig::default(),
        }
    }
}
impl TranscriptionSettings {
    pub(crate) fn to_speech(&self) -> speech_transcription::SpeechConfig {
        speech_transcription::SpeechConfig {
            backend: self.backend,
            model_path: self.model_path.clone(),
            language: self.language.clone(),
            threads: self.threads,
            use_gpu: self.use_gpu,
            provider: self.provider.clone(),
            window_ms: self.window_ms,
            step_ms: self.step_ms,
            queue_capacity: self.queue_capacity,
            vad: self.vad.clone(),
        }
    }
}

impl EngineConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        if !(1..=1024).contains(&self.command_capacity)
            || !(2..=1024).contains(&self.subscriber_capacity)
            || !(1..=128).contains(&self.max_subscribers)
            || !(Duration::from_millis(250)..=Duration::from_secs(5))
                .contains(&self.snapshot_interval)
            || self.recordings_directory.as_os_str().is_empty()
        {
            return Err(EngineError::new(
                EngineErrorCode::InvalidConfiguration,
                "command_capacity: 1..1024; subscriber_capacity: 2..1024; max_subscribers: 1..128; snapshot_interval: 250ms..5s; directory must be nonempty",
            ));
        }
        for source in [&self.microphone, &self.system_audio] {
            crate::backend::native_config(&source.configured)
                .validate()
                .map_err(|e| {
                    EngineError::new(EngineErrorCode::InvalidConfiguration, e.to_string())
                })?;
        }
        Ok(())
    }
}
