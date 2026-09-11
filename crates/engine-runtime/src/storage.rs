use crate::{
    AudioSource, EngineError, EngineErrorCode, EngineSnapshot, Result, SessionId, SessionInfo,
};
use audio_core::{AudioFormat, AudioFrame};
use serde::Serialize;
use speech_transcription::SpeechSegment;
use std::{
    fs::{self, File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) fn storage_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::new(EngineErrorCode::StorageFailed, error.to_string())
}

pub(crate) fn create_session(root: &Path) -> Result<(SessionInfo, PathBuf)> {
    let create_error =
        |e: std::io::Error| EngineError::new(EngineErrorCode::SessionCreationFailed, e.to_string());
    let root = std::env::current_dir().map_err(create_error)?.join(root);
    fs::create_dir_all(&root).map_err(create_error)?;
    let id = SessionId(uuid::Uuid::new_v4().to_string());
    let directory = root.join(&id.0);
    fs::create_dir(&directory).map_err(create_error)?;
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| EngineError::new(EngineErrorCode::SessionCreationFailed, e.to_string()))?
        .as_millis() as u64;
    Ok((
        SessionInfo {
            id,
            started_at_unix_ms,
            recording_directory: directory.to_string_lossy().into_owned(),
        },
        directory,
    ))
}

#[derive(Debug, Serialize)]
pub(crate) struct RecordingInfo {
    source: AudioSource,
    file: String,
    sample_rate: u32,
    channels: u16,
    written_sample_frames: u64,
    received_sample_frames: u64,
}

/// Session-specific WAV sink. Unlike the fixed-duration v0.1 CLI sink, this
/// accepts repeated source activations against a session-relative timeline.
pub(crate) struct Recording {
    path: PathBuf,
    source: AudioSource,
    writer: Option<hound::WavWriter<BufWriter<File>>>,
    format: Option<AudioFormat>,
    written: u64,
    received: u64,
}
impl Recording {
    pub fn new(directory: &Path, source: AudioSource) -> Self {
        let name = match source {
            AudioSource::Microphone => "microphone.wav",
            AudioSource::System => "system.wav",
        };
        Self {
            path: directory.join(name),
            source,
            writer: None,
            format: None,
            written: 0,
            received: 0,
        }
    }

    pub fn write(&mut self, frame: &AudioFrame, timestamp: Duration) -> Result<()> {
        let format = frame.format();
        if self.format.is_some_and(|old| old != format) {
            return Err(storage_error(
                "source format changed on re-enable; start a new session to use the new format",
            ));
        }
        if self.writer.is_none() {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.path)
                .map_err(storage_error)?;
            self.writer = Some(
                hound::WavWriter::new(
                    BufWriter::new(file),
                    hound::WavSpec {
                        channels: format.channels(),
                        sample_rate: format.sample_rate(),
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float,
                    },
                )
                .map_err(storage_error)?,
            );
            self.format = Some(format);
        }
        let position = sample_position(timestamp, format)?;
        let end = position
            .checked_add(frame.sample_frames() as u64)
            .ok_or_else(|| storage_error("sample timeline overflow"))?;
        self.check_limit(end)?;
        self.pad(position)?;
        let skip = self
            .written
            .saturating_sub(position)
            .min(frame.sample_frames() as u64) as usize;
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| storage_error("WAV writer unavailable"))?;
        for &sample in &frame.samples()[skip * format.channels() as usize..] {
            writer.write_sample(sample).map_err(storage_error)?;
        }
        self.written += (frame.sample_frames() - skip) as u64;
        self.received += frame.sample_frames() as u64;
        Ok(())
    }

    fn check_limit(&self, frames: u64) -> Result<()> {
        let format = self
            .format
            .ok_or_else(|| storage_error("WAV format unavailable"))?;
        let bytes = frames
            .checked_mul(format.channels() as u64)
            .and_then(|n| n.checked_mul(4));
        if bytes.is_none_or(|n| n > u32::MAX as u64 - 128) {
            return Err(storage_error(
                "WAV reached its 4 GiB limit; stop and start a new session (rotation is not implemented)",
            ));
        }
        Ok(())
    }

    fn pad(&mut self, position: u64) -> Result<()> {
        self.check_limit(position)?;
        let channels = self
            .format
            .ok_or_else(|| storage_error("WAV format unavailable"))?
            .channels();
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| storage_error("WAV writer unavailable"))?;
        for _ in self.written..position {
            for _ in 0..channels {
                writer.write_sample(0.0f32).map_err(storage_error)?;
            }
        }
        self.written = self.written.max(position);
        Ok(())
    }

    pub fn finish(mut self, duration: Duration) -> Result<Option<RecordingInfo>> {
        let Some(format) = self.format else {
            return Ok(None);
        };
        // Attempt finalization even if padding fails (disk full or WAV limit).
        let padding = sample_position(duration, format).and_then(|end| self.pad(end));
        let finalization = self
            .writer
            .take()
            .ok_or_else(|| storage_error("WAV writer unavailable"))?
            .finalize()
            .map_err(storage_error);
        padding?;
        finalization?;
        Ok(Some(RecordingInfo {
            source: self.source,
            file: self
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            sample_rate: format.sample_rate(),
            channels: format.channels(),
            written_sample_frames: self.written,
            received_sample_frames: self.received,
        }))
    }
}

fn sample_position(timestamp: Duration, format: AudioFormat) -> Result<u64> {
    let frames = timestamp
        .as_nanos()
        .checked_mul(format.sample_rate() as u128)
        .and_then(|n| n.checked_add(500_000_000))
        .map(|n| n / 1_000_000_000);
    frames
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| storage_error("sample timeline overflow"))
}

pub(crate) fn write_metadata(
    directory: &Path,
    snapshot: &EngineSnapshot,
    files: &[RecordingInfo],
) -> Result<()> {
    #[derive(Serialize)]
    struct Metadata<'a> {
        schema_version: u32,
        snapshot: &'a EngineSnapshot,
        recordings: &'a [RecordingInfo],
    }
    let path = directory.join("session.json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(storage_error)?;
    serde_json::to_writer_pretty(
        &mut file,
        &Metadata {
            schema_version: 1,
            snapshot,
            recordings: files,
        },
    )
    .map_err(storage_error)?;
    file.sync_all().map_err(storage_error)?;
    fs::rename(path, directory.join("session.json")).map_err(storage_error)
}

/// Only final segments reach disk; partial UI updates are intentionally volatile.
pub(crate) fn write_transcript(directory: &Path, segments: &[SpeechSegment]) -> Result<()> {
    if segments.is_empty() {
        return Ok(());
    }
    let json = directory.join("transcript.json");
    let text = directory.join("transcript.txt");
    let data = serde_json::to_vec_pretty(segments).map_err(storage_error)?;
    fs::write(&json, data).map_err(storage_error)?;
    let lines = segments
        .iter()
        .map(|segment| {
            format!(
                "[{}–{} ms] {:?}: {}",
                segment.start_ms, segment.end_ms, segment.source, segment.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(text, lines).map_err(storage_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_stereo_and_disabled_gap_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let mut recording = Recording::new(dir.path(), AudioSource::System);
        let format = AudioFormat::new(10, 2).unwrap();
        let frame = AudioFrame::new(
            audio_core::AudioSourceKind::System,
            Duration::ZERO,
            format,
            vec![0.25, -0.25],
        )
        .unwrap();
        recording.write(&frame, Duration::ZERO).unwrap();
        recording.write(&frame, Duration::from_millis(500)).unwrap();
        recording.finish(Duration::from_secs(1)).unwrap();
        let mut reader = hound::WavReader::open(dir.path().join("system.wav")).unwrap();
        assert_eq!(reader.duration(), 10);
        let samples: Vec<_> = reader
            .samples::<f32>()
            .map(std::result::Result::unwrap)
            .collect();
        assert_eq!(&samples[..2], &[0.25, -0.25]);
        assert_eq!(&samples[2..10], &[0.0; 8]);
        assert_eq!(&samples[10..12], &[0.25, -0.25]);
        let mut duplicate = Recording::new(dir.path(), AudioSource::System);
        assert!(duplicate.write(&frame, Duration::ZERO).is_err());
    }
    #[test]
    fn changed_format_and_oversized_timeline_fail_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let mut recording = Recording::new(dir.path(), AudioSource::Microphone);
        let frame = |rate| {
            AudioFrame::new(
                audio_core::AudioSourceKind::Microphone,
                Duration::ZERO,
                AudioFormat::new(rate, 1).unwrap(),
                vec![0.1],
            )
            .unwrap()
        };
        recording.write(&frame(48000), Duration::ZERO).unwrap();
        assert!(recording.write(&frame(24000), Duration::ZERO).is_err());
        assert!(
            recording
                .write(&frame(48000), Duration::from_secs(u64::MAX))
                .is_err()
        );
    }

    #[test]
    fn transcript_json_preserves_only_supplied_final_segments() {
        let directory = tempfile::tempdir().unwrap();
        let segments = vec![SpeechSegment {
            source: AudioSource::Microphone,
            start_ms: 100,
            end_ms: 900,
            text: "go go now".into(),
        }];
        write_transcript(directory.path(), &segments).unwrap();
        let persisted: Vec<SpeechSegment> =
            serde_json::from_slice(&fs::read(directory.path().join("transcript.json")).unwrap())
                .unwrap();
        assert_eq!(persisted, segments);
        assert_eq!(
            fs::read_to_string(directory.path().join("transcript.txt")).unwrap(),
            "[100–900 ms] Microphone: go go now\n"
        );
    }
}
