//! Generic local model catalog, inspection, and installation helpers.
//!
//! This crate owns model metadata and file handling. Clients receive generic
//! descriptors and never need to know ASR implementation details.
use bzip2::read::BzDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tar::Archive;

pub use app_paths::data_root as app_data_root;

const PARAKEET_ID: &str = "parakeet-tdt-0.6b-v3-int8";
const PARAKEET_ARCHIVE: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2";
const PARAKEET_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2";
const PARAKEET_ARCHIVE_SIZE: u64 = 487_170_055;
const PARAKEET_ARCHIVE_SHA256: &str =
    "5793d0fd397c5778d2cf2126994d58e9d56b1be7c04d13c7a15bb1b4eafb16bf";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelCapabilities {
    pub supports_partials: bool,
    pub supports_true_streaming: bool,
    pub supports_language_detection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelFile {
    pub filename: String,
    pub source_url: Option<String>,
    pub expected_size_bytes: Option<u64>,
    pub sha256: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub backend: String,
    pub display_name: String,
    pub version: String,
    pub storage_directory: String,
    pub files: Vec<ModelFile>,
    pub languages: Vec<String>,
    pub quantization: Option<String>,
    pub capabilities: ModelCapabilities,
    /// Shown when a multi-file archive needs an explicit external installer.
    pub install_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    Missing,
    Incomplete,
    Ready,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("unknown model: {0}")]
    UnknownModel(String),
    #[error("{0} requires its documented external installer")]
    ExternalInstallerRequired(String),
    #[error("language is not supported by model: {0}")]
    UnsupportedLanguage(String),
    #[error("model is not installed: {0}")]
    NotInstalled(String),
    #[error("model file has no download URL: {0}")]
    NoDownloadUrl(String),
    #[error("download cancelled")]
    Cancelled,
    #[error("unexpected file size")]
    SizeMismatch,
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("model download stalled for 30 seconds")]
    DownloadStalled,
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
}

fn verify_parakeet_archive(written: u64, digest: &str) -> Result<(), ModelError> {
    if written != PARAKEET_ARCHIVE_SIZE {
        return Err(ModelError::SizeMismatch);
    }
    if digest != PARAKEET_ARCHIVE_SHA256 {
        return Err(ModelError::ChecksumMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ModelManager {
    root: PathBuf,
    catalog: Vec<ModelDescriptor>,
}

impl ModelManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            catalog: catalog(),
        }
    }

    pub fn default_root(home: &Path) -> PathBuf {
        app_data_root(home).join("models")
    }

    pub fn catalog(&self) -> &[ModelDescriptor] {
        &self.catalog
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn descriptor(&self, id: &str) -> Result<&ModelDescriptor, ModelError> {
        self.catalog
            .iter()
            .find(|model| model.id == id)
            .ok_or_else(|| ModelError::UnknownModel(id.into()))
    }

    pub fn directory(&self, descriptor: &ModelDescriptor) -> PathBuf {
        self.root.join(&descriptor.storage_directory)
    }

    pub fn path(&self, descriptor: &ModelDescriptor, file: &ModelFile) -> PathBuf {
        self.directory(descriptor).join(&file.filename)
    }

    /// Resolve the artifact passed to the shared transcription runtime.
    pub fn runtime_path(&self, descriptor: &ModelDescriptor) -> Result<PathBuf, ModelError> {
        if self.state(descriptor) != ModelState::Ready {
            return Err(ModelError::NotInstalled(descriptor.id.clone()));
        }
        if descriptor.backend == "parakeet" {
            return Ok(self.directory(descriptor));
        }
        let file = descriptor.files.first().ok_or_else(|| {
            ModelError::ExternalInstallerRequired(descriptor.display_name.clone())
        })?;
        Ok(self.path(descriptor, file))
    }

    /// Validate a UI-selected language against the model's canonical catalog.
    /// `None` means automatic language selection.
    pub fn validate_language(
        &self,
        descriptor: &ModelDescriptor,
        language: Option<&str>,
    ) -> Result<(), ModelError> {
        validate_language(descriptor, language)
    }

    /// Removes a fully managed model directory. Callers are responsible for
    /// ensuring that the model is not in use by an active transcription.
    pub fn remove(&self, id: &str) -> Result<(), ModelError> {
        let descriptor = self.descriptor(id)?;
        let directory = self.directory(descriptor);
        if directory.exists() {
            fs::remove_dir_all(directory)?;
        }
        Ok(())
    }

    pub fn state(&self, descriptor: &ModelDescriptor) -> ModelState {
        let required: Vec<_> = descriptor
            .files
            .iter()
            .filter(|file| file.required)
            .collect();
        if required.is_empty()
            || required
                .iter()
                .all(|file| self.path(descriptor, file).is_file())
        {
            ModelState::Ready
        } else if required
            .iter()
            .any(|file| self.path(descriptor, file).exists())
        {
            ModelState::Incomplete
        } else {
            ModelState::Missing
        }
    }

    /// Install a descriptor only when it has one self-contained downloadable
    /// artifact. Archive-based models deliberately require their explicit
    /// installer rather than guessing an extraction format in a UI client.
    pub fn install(
        &self,
        id: &str,
        cancelled: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<PathBuf, ModelError> {
        let descriptor = self.descriptor(id)?;
        if descriptor.files.len() != 1 {
            return Err(ModelError::ExternalInstallerRequired(
                descriptor.display_name.clone(),
            ));
        }
        let file = &descriptor.files[0];
        let url = file
            .source_url
            .as_ref()
            .ok_or_else(|| ModelError::NoDownloadUrl(file.filename.clone()))?;
        let destination = self.path(descriptor, file);
        let parent = destination
            .parent()
            .ok_or_else(|| io::Error::other("model destination has no parent"))?;
        fs::create_dir_all(parent)?;
        let part = destination.with_file_name(format!("{}.part", file.filename));
        let result = (|| {
            let (written, digest) = download_to_file(url, &part, cancelled, |done, _| {
                progress(done, file.expected_size_bytes);
            })?;
            if file.expected_size_bytes.is_some_and(|size| size != written) {
                return Err(ModelError::SizeMismatch);
            }
            if file
                .sha256
                .as_ref()
                .is_some_and(|expected| *expected != digest)
            {
                return Err(ModelError::ChecksumMismatch);
            }
            fs::rename(&part, &destination)?;
            Ok(destination.clone())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&part);
        }
        result
    }

    /// Install the Parakeet archive and validate its required model files.
    pub fn install_parakeet(
        &self,
        cancelled: &AtomicBool,
        progress: impl FnMut(u64, Option<u64>),
    ) -> Result<PathBuf, ModelError> {
        self.install_parakeet_with_phase(cancelled, |_| {}, progress)
    }

    /// Install Parakeet and report the transition from download to extraction.
    pub fn install_parakeet_with_phase(
        &self,
        cancelled: &AtomicBool,
        mut phase: impl FnMut(&'static str),
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<PathBuf, ModelError> {
        let descriptor = self.descriptor(PARAKEET_ID)?;
        if self.state(descriptor) == ModelState::Ready {
            return Ok(self.directory(descriptor));
        }
        fs::create_dir_all(&self.root)?;
        let archive_path = self.root.join(PARAKEET_ARCHIVE);
        let part = archive_path.with_file_name(format!("{PARAKEET_ARCHIVE}.part"));
        let staging = self
            .root
            .join(format!(".{PARAKEET_ID}-{}.staging", std::process::id()));
        let backup = self
            .root
            .join(format!(".{PARAKEET_ID}-{}.backup", std::process::id()));
        phase("downloading");
        let result = (|| {
            let (downloaded, digest) =
                download_to_file(PARAKEET_URL, &part, cancelled, |done, _| {
                    progress(done, Some(PARAKEET_ARCHIVE_SIZE));
                })?;
            verify_parakeet_archive(downloaded, &digest)?;
            if cancelled.load(Ordering::Acquire) {
                return Err(ModelError::Cancelled);
            }
            if archive_path.exists() {
                fs::remove_file(&archive_path)?;
            }
            fs::rename(&part, &archive_path)?;
            phase("extracting");
            if staging.exists() {
                fs::remove_dir_all(&staging)?;
            }
            fs::create_dir(&staging)?;
            Archive::new(BzDecoder::new(fs::File::open(&archive_path)?)).unpack(&staging)?;
            let staged_model = staging.join(&descriptor.storage_directory);
            let required_files_exist = descriptor
                .files
                .iter()
                .filter(|file| file.required)
                .all(|file| staged_model.join(&file.filename).is_file());
            if !required_files_exist {
                return Err(ModelError::Io(io::Error::other(
                    "Parakeet archive did not contain the required model files",
                )));
            }
            if backup.exists() {
                fs::remove_dir_all(&backup)?;
            }
            let destination = self.directory(descriptor);
            if destination.exists() {
                fs::rename(&destination, &backup)?;
            }
            if let Err(error) = fs::rename(&staged_model, &destination) {
                if backup.exists() {
                    let _ = fs::rename(&backup, &destination);
                }
                return Err(ModelError::Io(error));
            }
            let _ = fs::remove_dir_all(&backup);
            Ok(self.directory(descriptor))
        })();
        let _ = fs::remove_file(&part);
        let _ = fs::remove_file(&archive_path);
        let _ = fs::remove_dir_all(&staging);
        result
    }
}

pub fn validate_language(
    descriptor: &ModelDescriptor,
    language: Option<&str>,
) -> Result<(), ModelError> {
    if let Some(language) = language
        && !descriptor
            .languages
            .iter()
            .any(|supported| supported == language)
    {
        return Err(ModelError::UnsupportedLanguage(language.to_owned()));
    }
    Ok(())
}

/// Stream a model download to a temporary path, timing out stalled body reads
/// while allowing the total transfer to take as long as it needs.
fn download_to_file(
    url: &str,
    destination: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(u64, String), ModelError> {
    const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
    let client = reqwest::Client::builder()
        .connect_timeout(IDLE_TIMEOUT)
        .build()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let mut response = client.get(url).send().await?.error_for_status()?;
        let total = response.content_length();
        let mut output = fs::File::create(destination)?;
        let mut hash = Sha256::new();
        let mut written = 0_u64;
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err(ModelError::Cancelled);
            }
            let next = tokio::time::timeout(IDLE_TIMEOUT, response.chunk())
                .await
                .map_err(|_| ModelError::DownloadStalled)??;
            let Some(chunk) = next else {
                break;
            };
            output.write_all(&chunk)?;
            hash.update(&chunk);
            written = written.saturating_add(chunk.len() as u64);
            progress(written, total);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(ModelError::Cancelled);
        }
        Ok((written, format!("{:x}", hash.finalize())))
    })
}

pub fn catalog() -> Vec<ModelDescriptor> {
    let no_streaming = ModelCapabilities {
        supports_partials: false,
        supports_true_streaming: false,
        supports_language_detection: false,
    };
    vec![
        ModelDescriptor {
            id: PARAKEET_ID.into(),
            backend: "parakeet".into(),
            display_name: "Parakeet TDT 0.6B v3 INT8".into(),
            version: "v3".into(),
            storage_directory: "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8".into(),
            files: ["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"]
                .into_iter()
                .map(|filename| ModelFile {
                    filename: filename.into(),
                    source_url: None,
                    expected_size_bytes: None,
                    sha256: None,
                    required: true,
                })
                .collect(),
            languages: [
                "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu",
                "it", "lv", "lt", "mt", "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru",
                "uk",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            quantization: Some("int8".into()),
            capabilities: ModelCapabilities {
                supports_language_detection: true,
                ..no_streaming.clone()
            },
            install_hint: Some("Run `rimv models install` to install the verified Parakeet package.".into()),
        },
        ModelDescriptor {
            id: "silero-vad".into(),
            backend: "vad".into(),
            display_name: "Silero VAD".into(),
            version: "v5".into(),
            storage_directory: ".".into(),
            files: vec![ModelFile {
                filename: "silero_vad.onnx".into(),
                source_url: Some("https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx".into()),
                expected_size_bytes: None,
                sha256: None,
                required: true,
            }],
            languages: Vec::new(),
            quantization: None,
            capabilities: no_streaming.clone(),
            install_hint: None,
        },
        legacy_whisper("tiny", "Tiny", "ggml-tiny.bin", 77_691_713, "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21", "Fastest legacy Whisper option."),
        legacy_whisper("base", "Base", "ggml-base.bin", 147_951_465, "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe", "Balanced legacy Whisper option."),
        legacy_whisper("small", "Small", "ggml-small.bin", 487_601_967, "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b", "Higher-accuracy legacy Whisper option."),
        legacy_whisper("medium", "Medium", "ggml-medium.bin", 0, "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208", "Larger multilingual Whisper model."),
        legacy_whisper("large", "Large v3", "ggml-large-v3.bin", 0, "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2", "Highest-accuracy Whisper model in the local catalog."),
        legacy_whisper("turbo", "Turbo", "ggml-large-v3-turbo.bin", 1_624_555_275, "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69", "Speed-oriented Whisper large-v3 variant."),
    ]
}

fn legacy_whisper(
    id: &str,
    display_name: &str,
    filename: &str,
    expected_size_bytes: u64,
    sha256: &str,
    hint: &str,
) -> ModelDescriptor {
    ModelDescriptor {
        id: format!("whisper-{id}"),
        backend: "whisper".into(),
        display_name: format!("Whisper {display_name}"),
        version: "legacy".into(),
        storage_directory: format!("whisper/{id}"),
        files: vec![ModelFile {
            filename: filename.into(),
            source_url: Some(format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}"
            )),
            expected_size_bytes: (expected_size_bytes != 0).then_some(expected_size_bytes),
            sha256: Some(sha256.into()),
            required: true,
        }],
        languages: vec!["en".into(), "es".into(), "ru".into()],
        quantization: None,
        capabilities: ModelCapabilities {
            supports_partials: false,
            supports_true_streaming: false,
            supports_language_detection: true,
        },
        install_hint: Some(hint.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parakeet_archive_requires_the_pinned_size_and_sha256() {
        assert!(verify_parakeet_archive(PARAKEET_ARCHIVE_SIZE, PARAKEET_ARCHIVE_SHA256).is_ok());
        assert!(matches!(
            verify_parakeet_archive(PARAKEET_ARCHIVE_SIZE - 1, PARAKEET_ARCHIVE_SHA256),
            Err(ModelError::SizeMismatch)
        ));
        assert!(matches!(
            verify_parakeet_archive(PARAKEET_ARCHIVE_SIZE, "0"),
            Err(ModelError::ChecksumMismatch)
        ));
    }
}
