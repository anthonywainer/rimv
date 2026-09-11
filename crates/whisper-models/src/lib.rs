//! Model catalog, selection, storage, and atomic verification for rimv.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use system_profile::SystemProfile;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WhisperModelDescriptor {
    pub id: WhisperModel,
    pub display_name: &'static str,
    pub filename: &'static str,
    pub download_url: &'static str,
    pub expected_size_bytes: u64,
    pub sha256: &'static str,
    pub description: &'static str,
}
pub const CATALOG: [WhisperModelDescriptor; 3] = [
    WhisperModelDescriptor {
        id: WhisperModel::Tiny,
        display_name: "Tiny",
        filename: "ggml-tiny.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        expected_size_bytes: 77691713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        description: "Fastest and lowest-memory option.",
    },
    WhisperModelDescriptor {
        id: WhisperModel::Base,
        display_name: "Base",
        filename: "ggml-base.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        expected_size_bytes: 147951465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        description: "Balanced realtime default.",
    },
    WhisperModelDescriptor {
        id: WhisperModel::Small,
        display_name: "Small",
        filename: "ggml-small.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        expected_size_bytes: 487601967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        description: "Higher accuracy with more memory and compute.",
    },
];
pub fn descriptor(model: WhisperModel) -> &'static WhisperModelDescriptor {
    &CATALOG[match model {
        WhisperModel::Tiny => 0,
        WhisperModel::Base => 1,
        WhisperModel::Small => 2,
    }]
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRecommendation {
    pub model: WhisperModel,
    pub reason: String,
}
pub fn recommend(p: &SystemProfile) -> ModelRecommendation {
    let gib = p.physical_memory_bytes / (1024 * 1024 * 1024);
    let model = if gib >= 16 && p.apple_silicon && p.metal_available {
        WhisperModel::Small
    } else if gib >= 8 {
        WhisperModel::Base
    } else {
        WhisperModel::Tiny
    };
    ModelRecommendation {
        model,
        reason: format!(
            "Selected for {} GiB RAM, {:?} architecture, and Metal {} while prioritizing realtime use.",
            gib,
            p.architecture,
            if p.metal_available {
                "availability"
            } else {
                "unavailability"
            }
        ),
    }
}
pub fn models_root(home: &Path) -> PathBuf {
    home.join("Library/Application Support/rimv/models/whisper")
}
pub fn model_path(root: &Path, model: WhisperModel) -> PathBuf {
    root.join(match model {
        WhisperModel::Tiny => "tiny",
        WhisperModel::Base => "base",
        WhisperModel::Small => "small",
    })
    .join(descriptor(model).filename)
}
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("unexpected size")]
    SizeMismatch,
    #[error("download cancelled")]
    Cancelled,
}
pub fn install_from_reader(
    root: &Path,
    model: WhisperModel,
    mut input: impl Read,
) -> Result<PathBuf, ModelError> {
    let final_path = model_path(root, model);
    let parent = final_path
        .parent()
        .ok_or_else(|| io::Error::other("no model parent"))?;
    fs::create_dir_all(parent)?;
    let part = final_path.with_extension("bin.part");
    let mut file = fs::File::create(&part)?;
    let mut hash = Sha256::new();
    let mut n = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buf)?;
        if count == 0 {
            break;
        }
        file.write_all(&buf[..count])?;
        hash.update(&buf[..count]);
        n += count as u64;
    }
    if n != descriptor(model).expected_size_bytes {
        let _ = fs::remove_file(&part);
        return Err(ModelError::SizeMismatch);
    }
    if format!("{:x}", hash.finalize()) != descriptor(model).sha256 {
        let _ = fs::remove_file(&part);
        return Err(ModelError::ChecksumMismatch);
    }
    fs::rename(part, &final_path)?;
    Ok(final_path)
}
use std::io::Write;
/// Downloads one catalog model over HTTPS without retaining its body in RAM.
/// `progress` runs on the caller's worker thread and must return quickly.
pub fn download(
    root: &Path,
    model: WhisperModel,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, ModelError> {
    let d = descriptor(model);
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?
        .get(d.download_url)
        .send()?
        .error_for_status()?;
    let final_path = model_path(root, model);
    let parent = final_path
        .parent()
        .ok_or_else(|| io::Error::other("no model parent"))?;
    fs::create_dir_all(parent)?;
    let part = final_path.with_extension("bin.part");
    let result = (|| {
        let mut input = response;
        let mut file = fs::File::create(&part)?;
        let mut hash = Sha256::new();
        let mut n = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err(ModelError::Cancelled);
            }
            let count = input.read(&mut buf)?;
            if count == 0 {
                break;
            }
            file.write_all(&buf[..count])?;
            hash.update(&buf[..count]);
            n += count as u64;
            progress(n, d.expected_size_bytes);
        }
        if n != d.expected_size_bytes {
            return Err(ModelError::SizeMismatch);
        }
        if format!("{:x}", hash.finalize()) != d.sha256 {
            return Err(ModelError::ChecksumMismatch);
        }
        fs::rename(&part, &final_path)?;
        Ok(final_path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(part);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thresholds() {
        let mut p = SystemProfile::detect();
        p.physical_memory_bytes = 7 * 1024u64.pow(3);
        assert_eq!(recommend(&p).model, WhisperModel::Tiny);
        p.physical_memory_bytes = 8 * 1024u64.pow(3);
        assert_eq!(recommend(&p).model, WhisperModel::Base);
    }
    #[test]
    fn paths() {
        assert!(
            model_path(Path::new("/tmp"), WhisperModel::Small).ends_with("small/ggml-small.bin")
        );
    }
}
