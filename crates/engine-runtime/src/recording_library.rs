//! Shared browsing and management of persisted recording sessions.
//!
//! UI adapters supply only a session ID and a destination for exports; paths
//! are resolved and validated beneath the configured recordings directory.
use crate::{EngineError, EngineErrorCode, EngineRuntime, EngineStatus, Result};
use engine_protocol::{AudioSource, EngineSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const TITLE_FILE: &str = "rimv-session.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub source: AudioSource,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingSummary {
    pub session_id: String,
    pub title: String,
    pub started_at_unix_ms: u64,
    pub duration_ms: u64,
    pub state: String,
    pub sources: Vec<String>,
    pub has_transcript: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingDetails {
    pub summary: RecordingSummary,
    pub transcript: Vec<TranscriptLine>,
}

#[derive(Debug, Clone)]
pub struct RecordingLibrary {
    root: PathBuf,
}

impl RecordingLibrary {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn error(message: impl std::fmt::Display) -> EngineError {
        EngineError::new(EngineErrorCode::StorageFailed, message.to_string())
    }

    fn directory(&self, id: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            return Err(Self::error("invalid recording identifier"));
        }
        let root = self.root.canonicalize().map_err(Self::error)?;
        let directory = root.join(id).canonicalize().map_err(Self::error)?;
        if directory.parent() != Some(root.as_path())
            || directory.file_name().and_then(|name| name.to_str()) != Some(id)
            || !directory.is_dir()
        {
            return Err(Self::error("recording is outside the recordings directory"));
        }
        Ok(directory)
    }

    fn active(snapshot: &EngineSnapshot, id: &str) -> bool {
        (matches!(
            snapshot.status,
            EngineStatus::Starting | EngineStatus::Recording | EngineStatus::Stopping
        ) || snapshot.microphone.active
            || snapshot.system_audio.active)
            && snapshot
                .session
                .as_ref()
                .is_some_and(|session| session.id.0 == id)
    }

    fn persisted(directory: &Path, id: &str) -> Result<serde_json::Value> {
        let data: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("session.json")).map_err(Self::error)?)
                .map_err(Self::error)?;
        let session = data
            .pointer("/snapshot/session")
            .ok_or_else(|| Self::error("recording metadata is incomplete"))?;
        if session.get("id").and_then(serde_json::Value::as_str) != Some(id)
            || directory.file_name().and_then(|name| name.to_str()) != Some(id)
        {
            return Err(Self::error("recording identity does not match its folder"));
        }
        Ok(data)
    }

    fn title(directory: &Path, id: &str) -> Option<String> {
        let sidecar: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join(TITLE_FILE)).ok()?).ok()?;
        (sidecar
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            == Some(id))
        .then(|| {
            sidecar
                .get("title")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .flatten()
    }

    fn summary(
        directory: &Path,
        id: &str,
        metadata: &serde_json::Value,
        state: &str,
    ) -> RecordingSummary {
        let session = metadata.pointer("/snapshot/session");
        let started_at_unix_ms = session
            .and_then(|s| s.get("started_at_unix_ms"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let duration_ms = metadata
            .pointer("/snapshot/elapsed_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let mut sources = metadata
            .get("recordings")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|r| r.get("source").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if state == "recording" {
            for source in ["microphone", "system"] {
                if directory.join(format!("{source}.wav")).is_file()
                    && !sources.iter().any(|s| s == source)
                {
                    sources.push(source.to_owned());
                }
            }
        }
        RecordingSummary {
            session_id: id.to_owned(),
            title: Self::title(directory, id).unwrap_or_else(|| "Recording".into()),
            started_at_unix_ms,
            duration_ms,
            state: state.into(),
            sources,
            has_transcript: directory.join("transcript.json").is_file(),
        }
    }

    pub fn list(&self, engine: &EngineRuntime) -> Result<Vec<RecordingSummary>> {
        self.list_snapshot(&engine.snapshot())
    }

    pub fn list_snapshot(&self, snapshot: &EngineSnapshot) -> Result<Vec<RecordingSummary>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut output = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(session) = snapshot
            .session
            .as_ref()
            .filter(|s| Self::active(&snapshot, &s.id.0))
            && let Ok(directory) = self.directory(&session.id.0)
            && Path::new(&session.recording_directory)
                .canonicalize()
                .ok()
                .as_ref()
                == Some(&directory)
        {
            let metadata = serde_json::json!({"snapshot":{"session":{"id":session.id.0,"started_at_unix_ms":session.started_at_unix_ms},"elapsed_ms":snapshot.elapsed_ms},"recordings":[]});
            output.push(Self::summary(
                &directory,
                &session.id.0,
                &metadata,
                "recording",
            ));
            seen.insert(session.id.0.clone());
        }
        for entry in fs::read_dir(&self.root).map_err(Self::error)?.flatten() {
            let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !valid_id(&id) || seen.contains(&id) {
                continue;
            }
            let Ok(directory) = self.directory(&id) else {
                continue;
            };
            let Ok(metadata) = Self::persisted(&directory, &id) else {
                continue;
            };
            output.push(Self::summary(&directory, &id, &metadata, "completed"));
        }
        output.sort_by(|a, b| b.started_at_unix_ms.cmp(&a.started_at_unix_ms));
        Ok(output)
    }

    pub fn get(&self, engine: &EngineRuntime, id: &str) -> Result<RecordingDetails> {
        let directory = self.directory(id)?;
        let snapshot = engine.snapshot();
        let active = Self::active(&snapshot, id);
        let metadata = if active {
            let session = snapshot.session.as_ref().expect("active session checked");
            serde_json::json!({"snapshot":{"session":{"id":session.id.0,"started_at_unix_ms":session.started_at_unix_ms},"elapsed_ms":snapshot.elapsed_ms},"recordings":[]})
        } else {
            Self::persisted(&directory, id)?
        };
        let mut transcript: Vec<TranscriptLine> = if active {
            let live = engine.transcript_snapshot();
            let updates = if live.session_id.as_deref() == Some(id) {
                live.updates
            } else {
                Vec::new()
            };
            updates
                .into_iter()
                .map(|update| TranscriptLine {
                    source: update.source,
                    start_ms: update.start_ms,
                    end_ms: update.end_ms,
                    text: [&update.stable_text, &update.unstable_text]
                        .into_iter()
                        .filter(|text| !text.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" "),
                })
                .filter(|line| !line.text.trim().is_empty())
        } else if directory.join("transcript.json").exists() {
            serde_json::from_slice(
                &fs::read(directory.join("transcript.json")).map_err(Self::error)?,
            )
            .map_err(Self::error)?
        } else {
            Vec::new()
        };
        transcript.sort_by_key(|line| line.start_ms);
        let mut summary = Self::summary(
            &directory,
            id,
            &metadata,
            if active { "recording" } else { "completed" },
        );
        summary.has_transcript = !transcript.is_empty();
        Ok(RecordingDetails {
            summary,
            transcript,
        })
    }

    pub fn rename(&self, engine: &EngineRuntime, id: &str, title: &str) -> Result<()> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 120 || title.chars().any(char::is_control) {
            return Err(Self::error(
                "recording names must contain 1–120 visible characters",
            ));
        }
        let directory = self.directory(id)?;
        let snapshot = engine.snapshot();
        let started = if Self::active(&snapshot, id) {
            snapshot
                .session
                .as_ref()
                .expect("active session checked")
                .started_at_unix_ms
        } else {
            Self::persisted(&directory, id)?
                .pointer("/snapshot/session/started_at_unix_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| Self::error("recording start time is missing"))?
        };
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({"schema_version":1,"session_id":id,"started_at_unix_ms":started,"title":title})).map_err(Self::error)?;
        let destination = directory.join(TITLE_FILE);
        let temporary = directory.join(format!("{TITLE_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, bytes).map_err(Self::error)?;
        let backup = directory.join(format!("{TITLE_FILE}.bak"));
        let had_destination = destination.exists();
        if had_destination {
            let _ = fs::remove_file(&backup);
            fs::rename(&destination, &backup).map_err(|error| {
                let _ = fs::remove_file(&temporary);
                Self::error(error)
            })?;
        }
        if let Err(error) = fs::rename(&temporary, &destination) {
            if had_destination {
                let _ = fs::rename(&backup, &destination);
            }
            let _ = fs::remove_file(&temporary);
            return Err(Self::error(error));
        }
        if had_destination {
            let _ = fs::remove_file(backup);
        }
        Ok(())
    }

    pub fn delete(&self, engine: &EngineRuntime, id: &str) -> Result<()> {
        let directory = self.directory(id)?;
        if Self::active(&engine.snapshot(), id) {
            return Err(Self::error(
                "an active or finalizing recording cannot be deleted",
            ));
        }
        Self::persisted(&directory, id)?;
        fs::remove_dir_all(directory).map_err(Self::error)
    }

    pub fn export(
        &self,
        engine: &EngineRuntime,
        id: &str,
        format: ExportFormat,
    ) -> Result<Vec<u8>> {
        let details = self.get(engine, id)?;
        format_transcript(&details.transcript, format)
    }

    /// Canonical export for completed sessions. UI adapters choose the save
    /// destination; the shared core owns serialization and formatting.
    pub fn export_persisted(&self, id: &str, format: ExportFormat) -> Result<Vec<u8>> {
        let directory = self.directory(id)?;
        Self::persisted(&directory, id)?;
        let transcript: Vec<TranscriptLine> = if directory.join("transcript.json").is_file() {
            serde_json::from_slice(
                &fs::read(directory.join("transcript.json")).map_err(Self::error)?,
            )
            .map_err(Self::error)?
        } else {
            Vec::new()
        };
        format_transcript(&transcript, format)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Txt,
    Json,
}

fn format_transcript(transcript: &[TranscriptLine], format: ExportFormat) -> Result<Vec<u8>> {
    match format {
        ExportFormat::Json => {
            serde_json::to_vec_pretty(transcript).map_err(RecordingLibrary::error)
        }
        ExportFormat::Txt => Ok(transcript
            .iter()
            .map(|line| {
                format!(
                    "[{}–{} ms] {:?}: {}",
                    line.start_ms, line.end_ms, line.source, line.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
            .pipe_newline()),
    }
}

trait Newline {
    fn pipe_newline(self) -> Self;
}
impl Newline for Vec<u8> {
    fn pipe_newline(mut self) -> Self {
        self.push(b'\n');
        self
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, EngineRuntime, RecordingLibrary, String) {
        let root = tempfile::tempdir().unwrap();
        let engine = EngineRuntime::new(crate::EngineConfig {
            recordings_directory: root.path().into(),
            ..Default::default()
        })
        .unwrap();
        let id = "123e4567-e89b-12d3-a456-426614174000".to_owned();
        let directory = root.path().join(&id);
        fs::create_dir(&directory).unwrap();
        let metadata = serde_json::json!({
            "schema_version": 1,
            "snapshot": {"session":{"id":id,"started_at_unix_ms":1234},"elapsed_ms":5600},
            "recordings":[{"source":"microphone","file":"microphone.wav"}]
        });
        fs::write(
            directory.join("session.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        let transcript = vec![TranscriptLine {
            source: AudioSource::Microphone,
            start_ms: 10,
            end_ms: 40,
            text: "hello".into(),
        }];
        fs::write(
            directory.join("transcript.json"),
            serde_json::to_vec(&transcript).unwrap(),
        )
        .unwrap();
        let library = RecordingLibrary::new(root.path());
        (root, engine, library, id)
    }

    #[test]
    fn listing_detail_rename_and_exports_use_the_shared_persisted_session() {
        let (_root, engine, library, id) = fixture();
        let listed = library.list(&engine).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].session_id, id);
        let detail = library.get(&engine, &id).unwrap();
        assert_eq!(detail.transcript[0].text, "hello");
        library.rename(&engine, &id, "Team sync").unwrap();
        assert_eq!(
            library.get(&engine, &id).unwrap().summary.title,
            "Team sync"
        );
        assert_eq!(
            library.export(&engine, &id, ExportFormat::Txt).unwrap(),
            "[10–40 ms] Microphone: hello\n".as_bytes()
        );
        let json: Vec<TranscriptLine> =
            serde_json::from_slice(&library.export(&engine, &id, ExportFormat::Json).unwrap())
                .unwrap();
        assert_eq!(json, detail.transcript);
        engine.shutdown().unwrap();
    }

    #[test]
    fn library_rejects_invalid_ids_and_deletes_only_valid_completed_sessions() {
        let (_root, engine, library, id) = fixture();
        assert!(library.get(&engine, "../../outside").is_err());
        library.delete(&engine, &id).unwrap();
        assert!(library.list(&engine).unwrap().is_empty());
        engine.shutdown().unwrap();
    }
}
