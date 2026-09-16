//! Generic local model catalog, inspection, and installation helpers.
//!
//! This crate owns model metadata and file handling. Clients receive generic
//! descriptors and never need to know ASR implementation details.
use bzip2::read::BzDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tar::Archive;

const PARAKEET_ID: &str = "parakeet-tdt-0.6b-v3-int8";
const PARAKEET_ARCHIVE: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2";
const PARAKEET_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2";

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
    #[error("model file has no download URL: {0}")]
    NoDownloadUrl(String),
    #[error("download cancelled")]
    Cancelled,
    #[error("unexpected file size")]
    SizeMismatch,
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
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
        home.join("Library/Application Support/rimv/models")
    }

    pub fn catalog(&self) -> &[ModelDescriptor] {
        &self.catalog
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
            let mut response = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?
                .get(url)
                .send()?
                .error_for_status()?;
            let mut output = fs::File::create(&part)?;
            let mut hash = Sha256::new();
            let mut written = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                if cancelled.load(Ordering::Acquire) {
                    return Err(ModelError::Cancelled);
                }
                let count = response.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
                hash.update(&buffer[..count]);
                written += count as u64;
                progress(written, file.expected_size_bytes);
            }
            if file.expected_size_bytes.is_some_and(|size| size != written) {
                return Err(ModelError::SizeMismatch);
            }
            if file
                .sha256
                .as_ref()
                .is_some_and(|expected| *expected != format!("{:x}", hash.finalize()))
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

    /// Install the verified Parakeet archive into this manager's model root.
    pub fn install_parakeet(
        &self,
        cancelled: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<PathBuf, ModelError> {
        let descriptor = self.descriptor(PARAKEET_ID)?;
        if self.state(descriptor) == ModelState::Ready {
            return Ok(self.directory(descriptor));
        }
        fs::create_dir_all(&self.root)?;
        let archive_path = self.root.join(PARAKEET_ARCHIVE);
        let part = archive_path.with_file_name(format!("{PARAKEET_ARCHIVE}.part"));
        let result = (|| {
            let mut response = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?
                .get(PARAKEET_URL)
                .send()?
                .error_for_status()?;
            let mut output = fs::File::create(&part)?;
            let mut written = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                if cancelled.load(Ordering::Acquire) {
                    return Err(ModelError::Cancelled);
                }
                let count = response.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
                written += count as u64;
                progress(written, None);
            }
            fs::rename(&part, &archive_path)?;
            Archive::new(BzDecoder::new(fs::File::open(&archive_path)?)).unpack(&self.root)?;
            if self.state(descriptor) != ModelState::Ready {
                return Err(ModelError::Io(io::Error::other(
                    "Parakeet archive did not contain the required model files",
                )));
            }
            Ok(self.directory(descriptor))
        })();
        let _ = fs::remove_file(&part);
        let _ = fs::remove_file(&archive_path);
        result
    }
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
            languages: ["en", "es", "ru"].into_iter().map(str::to_string).collect(),
            quantization: Some("int8".into()),
            capabilities: no_streaming.clone(),
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
            expected_size_bytes: Some(expected_size_bytes),
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
    fn catalog_uses_generic_metadata_and_required_file_validation() {
        let directory = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(directory.path());
        let parakeet = manager.descriptor("parakeet-tdt-0.6b-v3-int8").unwrap();
        assert_eq!(parakeet.backend, "parakeet");
        assert_eq!(manager.state(parakeet), ModelState::Missing);
        let path = manager.path(parakeet, &parakeet.files[0]);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, []).unwrap();
        assert_eq!(manager.state(parakeet), ModelState::Incomplete);
    }

    #[test]
    fn legacy_whisper_is_described_without_leaking_into_generic_types() {
        let manager = ModelManager::new("/tmp/models");
        let descriptor = manager.descriptor("whisper-tiny").unwrap();
        assert_eq!(descriptor.backend, "whisper");
        assert_eq!(descriptor.files[0].filename, "ggml-tiny.bin");
    }
}
