use crate::{
    AudioSource, EngineError, EngineErrorCode, Result, SourceConfig,
    backend::{CaptureBackend, CaptureStream},
    storage::{self, Recording, RecordingInfo},
};
use speech_transcription::{SpeechEvent, SpeechSegment, SpeechWorker};
use std::sync::mpsc::Receiver;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) struct ActiveSource {
    stream: Box<dyn CaptureStream>,
    offset: Duration,
    first_timestamp: Option<Duration>,
}
pub(crate) struct SourceSlot {
    source: AudioSource,
    active: Option<ActiveSource>,
    recording: Option<Recording>,
    retired_drops: u64,
}
impl SourceSlot {
    fn new(directory: &std::path::Path, source: AudioSource) -> Self {
        Self {
            source,
            active: None,
            recording: Some(Recording::new(directory, source)),
            retired_drops: 0,
        }
    }
    pub fn activate(
        &mut self,
        backend: &mut dyn CaptureBackend,
        config: &SourceConfig,
        started: Instant,
    ) -> Result<()> {
        let mut stream = backend.open(self.source, config)?;
        stream.start()?;
        if !stream.is_running() {
            return Err(EngineError::new(
                EngineErrorCode::CaptureFailed,
                "backend did not enter running state",
            )
            .for_source(self.source));
        }
        self.active = Some(ActiveSource {
            stream,
            offset: started.elapsed(),
            first_timestamp: None,
        });
        Ok(())
    }
    pub fn active(&self) -> bool {
        self.active.is_some()
    }
    pub fn drops(&self) -> u64 {
        self.retired_drops.saturating_add(
            self.active
                .as_ref()
                .map_or(0, |a| a.stream.metrics().dropped_frames),
        )
    }
    pub fn poll(&mut self) -> Result<Vec<audio_core::AudioFrame>> {
        let Some(active) = &mut self.active else {
            return Ok(Vec::new());
        };
        let recording = self
            .recording
            .as_mut()
            .ok_or_else(|| storage::storage_error("recording already finalized"))?;
        // Fairness: process at most the native queue capacity per worker pass.
        let frames = Self::drain(active, recording, self.source)?;
        if !active.stream.is_running() {
            return Err(EngineError::new(
                EngineErrorCode::CaptureFailed,
                "native source stopped unexpectedly",
            )
            .for_source(self.source));
        }
        Ok(frames)
    }
    fn drain(
        active: &mut ActiveSource,
        recording: &mut Recording,
        source: AudioSource,
    ) -> Result<Vec<audio_core::AudioFrame>> {
        let mut output = Vec::new();
        for _ in 0..32 {
            let Some(frame) = active.stream.try_recv()? else {
                break;
            };
            let expected = match source {
                AudioSource::Microphone => audio_core::AudioSourceKind::Microphone,
                AudioSource::System => audio_core::AudioSourceKind::System,
            };
            if frame.source() != expected {
                return Err(EngineError::new(
                    EngineErrorCode::CaptureFailed,
                    "backend delivered wrong audio source",
                )
                .for_source(source));
            }
            let first = *active.first_timestamp.get_or_insert(frame.timestamp());
            let relative = frame.timestamp().checked_sub(first).ok_or_else(|| {
                EngineError::new(
                    EngineErrorCode::CaptureFailed,
                    "source timestamp moved before activation",
                )
                .for_source(source)
            })?;
            let timestamp = active
                .offset
                .checked_add(relative)
                .ok_or_else(|| storage::storage_error("timestamp overflow"))?;
            recording
                .write(&frame, timestamp)
                .map_err(|e| e.for_source(source))?;
            output.push(frame);
        }
        Ok(output)
    }
    pub fn deactivate(&mut self) -> Result<Vec<audio_core::AudioFrame>> {
        let Some(mut active) = self.active.take() else {
            return Ok(Vec::new());
        };
        let stopped = active.stream.stop();
        // stop() synchronizes the native source before draining its bounded tail.
        let drained = if let Some(recording) = &mut self.recording {
            Self::drain(&mut active, recording, self.source)
        } else {
            Ok(Vec::new())
        };
        self.retired_drops = self
            .retired_drops
            .saturating_add(active.stream.metrics().dropped_frames);
        // Dropping even a failed stream releases its native resources.
        stopped.and(drained).map_err(|e| e.for_source(self.source))
    }
    pub fn finish(&mut self, duration: Duration) -> Result<Option<RecordingInfo>> {
        match self.recording.take() {
            Some(recording) => recording
                .finish(duration)
                .map_err(|e| e.for_source(self.source)),
            None => Ok(None),
        }
    }
}

pub(crate) struct CaptureSession {
    pub started: Instant,
    pub directory: PathBuf,
    pub microphone: SourceSlot,
    pub system: SourceSlot,
    pub transcription: Option<(SpeechWorker, Receiver<SpeechEvent>)>,
    pub transcript_segments: Vec<SpeechSegment>,
}
impl CaptureSession {
    pub fn begin_recording(&mut self) {
        self.started = Instant::now();
        for slot in [&mut self.microphone, &mut self.system] {
            if let Some(active) = &mut slot.active {
                active.offset = Duration::ZERO;
            }
        }
    }
    pub fn new(directory: PathBuf) -> Self {
        Self {
            started: Instant::now(),
            microphone: SourceSlot::new(&directory, AudioSource::Microphone),
            system: SourceSlot::new(&directory, AudioSource::System),
            transcription: None,
            transcript_segments: Vec::new(),
            directory,
        }
    }
    pub fn slot(&mut self, source: AudioSource) -> &mut SourceSlot {
        match source {
            AudioSource::Microphone => &mut self.microphone,
            AudioSource::System => &mut self.system,
        }
    }
}
