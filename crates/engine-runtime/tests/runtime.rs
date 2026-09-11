use audio_capture::CaptureMetrics;
use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use engine_runtime::{
    backend::{CaptureBackend, CaptureStream},
    *,
};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct SourceControl {
    fail_start: bool,
    fail_runtime: bool,
    drops: u64,
    opens: u64,
    stops: u64,
}
type Controls = Arc<Mutex<[SourceControl; 2]>>;
fn index(source: AudioSource) -> usize {
    match source {
        AudioSource::Microphone => 0,
        AudioSource::System => 1,
    }
}
struct FakeBackend(Controls);
impl CaptureBackend for FakeBackend {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            microphone_capture: true,
            system_audio_capture: true,
            transcription: false,
        }
    }
    fn open(&mut self, source: AudioSource, _: &SourceConfig) -> Result<Box<dyn CaptureStream>> {
        self.0.lock().unwrap()[index(source)].opens += 1;
        Ok(Box::new(FakeStream {
            control: self.0.clone(),
            source,
            running: false,
            pending: false,
            // Deliberately !Send: creation and lifecycle must stay on the worker.
            _thread_bound: Rc::new(()),
        }))
    }
}
struct FakeStream {
    control: Controls,
    source: AudioSource,
    running: bool,
    pending: bool,
    _thread_bound: Rc<()>,
}
impl CaptureStream for FakeStream {
    fn start(&mut self) -> Result<()> {
        if self.control.lock().unwrap()[index(self.source)].fail_start {
            return Err(EngineError::new(
                EngineErrorCode::PermissionDenied,
                "test permission denied",
            )
            .for_source(self.source));
        }
        self.running = true;
        self.pending = true;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.running = false;
        self.control.lock().unwrap()[index(self.source)].stops += 1;
        Ok(())
    }
    fn try_recv(&mut self) -> Result<Option<AudioFrame>> {
        if self.running && self.control.lock().unwrap()[index(self.source)].fail_runtime {
            return Err(
                EngineError::new(EngineErrorCode::CaptureFailed, "test device removed")
                    .for_source(self.source),
            );
        }
        if !std::mem::take(&mut self.pending) {
            return Ok(None);
        }
        let (source, channels) = match self.source {
            AudioSource::Microphone => (AudioSourceKind::Microphone, 1),
            AudioSource::System => (AudioSourceKind::System, 2),
        };
        Ok(Some(
            AudioFrame::new(
                source,
                Duration::ZERO,
                AudioFormat::new(48000, channels).unwrap(),
                vec![0.125; 48 * channels as usize],
            )
            .unwrap(),
        ))
    }
    fn is_running(&self) -> bool {
        self.running
    }
    fn metrics(&self) -> CaptureMetrics {
        CaptureMetrics {
            dropped_frames: self.control.lock().unwrap()[index(self.source)].drops,
            backend_errors: 0,
        }
    }
}
fn setup() -> (EngineRuntime, tempfile::TempDir, Controls) {
    let directory = tempfile::tempdir().unwrap();
    let controls: Controls = Arc::new(Mutex::new(Default::default()));
    let config = EngineConfig {
        recordings_directory: directory.path().into(),
        ..Default::default()
    };
    let runtime = EngineRuntime::with_backend(config, FakeBackend(controls.clone())).unwrap();
    (runtime, directory, controls)
}
fn next_snapshot(
    events: &Subscription,
    predicate: impl Fn(&EngineSnapshot) -> bool,
) -> EngineSnapshot {
    for _ in 0..30 {
        if let EngineEvent::Snapshot { snapshot } =
            events.recv_timeout(Duration::from_secs(2)).unwrap()
            && predicate(&snapshot)
        {
            return *snapshot;
        }
    }
    panic!("expected snapshot was not published");
}

#[test]
fn lifecycle_events_metadata_and_two_listeners() {
    let (engine, directory, _) = setup();
    let a = engine.subscribe().unwrap();
    let b = engine.subscribe().unwrap();
    assert_eq!(a.recv().unwrap(), b.recv().unwrap());
    assert_eq!(engine.snapshot().status, EngineStatus::Idle);
    let recording = engine.start_capture().unwrap();
    assert_eq!(recording.status, EngineStatus::Recording);
    assert!(recording.microphone.active);
    assert!(!recording.system_audio.active);
    for expected in [EngineStatus::Starting, EngineStatus::Recording] {
        let first = a.recv().unwrap();
        assert_eq!(first, b.recv().unwrap());
        assert!(matches!(first, EngineEvent::Snapshot { snapshot } if snapshot.status == expected));
    }
    let late = engine.subscribe().unwrap();
    assert!(
        matches!(late.recv().unwrap(), EngineEvent::Snapshot { snapshot } if snapshot.status == EngineStatus::Recording)
    );
    let stopped = engine.stop_capture().unwrap();
    assert_eq!(stopped.status, EngineStatus::Idle);
    assert!(!stopped.microphone.active);
    assert!(stopped.microphone.enabled);
    let info = stopped.session.unwrap();
    assert!(uuid::Uuid::parse_str(&info.id.0).is_ok());
    let session_dir = directory.path().join(&info.id.0);
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(session_dir.join("session.json")).unwrap()).unwrap();
    assert_eq!(metadata["snapshot"]["status"], "idle");
    assert_eq!(metadata["snapshot"]["revision"], engine.snapshot().revision);
    let wav = hound::WavReader::open(session_dir.join("microphone.wav")).unwrap();
    assert_eq!(wav.spec().sample_rate, 48000);
    assert!(!session_dir.join("system.wav").exists());
    engine.shutdown().unwrap();
}

#[test]
fn idle_toggles_are_preferences_no_empty_session() {
    let (engine, directory, controls) = setup();
    let off = engine.set_microphone_enabled(false).unwrap();
    assert!(!off.microphone.enabled && !off.microphone.active);
    let error = engine.start_capture().unwrap_err();
    assert_eq!(error.code, EngineErrorCode::NoSourceEnabled);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    let on = engine.set_system_audio_enabled(true).unwrap();
    assert!(on.system_audio.enabled && !on.system_audio.active);
    assert_eq!(controls.lock().unwrap()[1].opens, 0);
    assert!(engine.start_capture().unwrap().system_audio.active);
    assert_eq!(controls.lock().unwrap()[0].opens, 0);
    engine.shutdown().unwrap();
}

#[test]
fn live_toggles_preserve_session_and_clock_even_if_both_disabled() {
    let (engine, _directory, controls) = setup();
    let first = engine.start_capture().unwrap();
    let id = first.session.unwrap().id;
    std::thread::sleep(Duration::from_millis(15));
    let mic_off = engine.set_microphone_enabled(false).unwrap();
    assert_eq!(mic_off.status, EngineStatus::Recording);
    assert!(!mic_off.microphone.active);
    assert!(mic_off.elapsed_ms >= 10);
    std::thread::sleep(Duration::from_millis(15));
    let system_on = engine.set_system_audio_enabled(true).unwrap();
    assert!(system_on.system_audio.active);
    assert!(system_on.elapsed_ms > mic_off.elapsed_ms);
    assert_eq!(system_on.session.unwrap().id, id);
    let mic_on = engine.set_microphone_enabled(true).unwrap();
    assert!(mic_on.microphone.active && mic_on.system_audio.active);
    assert!(
        !engine
            .set_system_audio_enabled(false)
            .unwrap()
            .system_audio
            .active
    );
    let stopped = engine.stop_capture().unwrap();
    std::thread::sleep(Duration::from_millis(15));
    assert_eq!(
        engine.send(EngineCommand::GetState).unwrap().elapsed_ms,
        stopped.elapsed_ms
    );
    assert_eq!(controls.lock().unwrap()[0].opens, 2);
    assert_eq!(controls.lock().unwrap()[0].stops, 2);
    engine.shutdown().unwrap();
}

#[test]
fn repeated_commands_return_errors_and_publish_them() {
    let (engine, _directory, _) = setup();
    let events = engine.subscribe().unwrap();
    engine.start_capture().unwrap();
    assert_eq!(
        engine.start_capture().unwrap_err().code,
        EngineErrorCode::AlreadyRecording
    );
    next_snapshot(&events, |s| {
        s.last_error
            .as_ref()
            .is_some_and(|e| e.code == EngineErrorCode::AlreadyRecording)
    });
    engine.stop_capture().unwrap();
    assert_eq!(
        engine.stop_capture().unwrap_err().code,
        EngineErrorCode::NotRecording
    );
    let error = next_snapshot(&events, |s| {
        s.last_error
            .as_ref()
            .is_some_and(|e| e.code == EngineErrorCode::NotRecording)
    });
    assert_eq!(error.status, EngineStatus::Idle);
}

#[test]
fn partial_failure_keeps_healthy_source_and_can_retry() {
    let (engine, _directory, controls) = setup();
    controls.lock().unwrap()[1].fail_start = true;
    engine.set_system_audio_enabled(true).unwrap();
    let snapshot = engine.start_capture().unwrap();
    assert_eq!(snapshot.status, EngineStatus::Recording);
    assert!(snapshot.microphone.active);
    assert!(snapshot.system_audio.enabled && !snapshot.system_audio.active);
    assert_eq!(
        snapshot.system_audio.last_error.unwrap().code,
        EngineErrorCode::PermissionDenied
    );
    controls.lock().unwrap()[1].fail_start = false;
    let snapshot = engine.set_system_audio_enabled(true).unwrap();
    assert!(snapshot.system_audio.active);
    assert!(snapshot.system_audio.last_error.is_none());
    engine.shutdown().unwrap();
}

#[test]
fn all_start_failures_end_in_error_and_next_start_recovers() {
    let (engine, _directory, controls) = setup();
    controls.lock().unwrap()[0].fail_start = true;
    assert_eq!(
        engine.start_capture().unwrap_err().code,
        EngineErrorCode::NoActiveSource
    );
    assert_eq!(engine.snapshot().status, EngineStatus::Error);
    let failed_id = engine.snapshot().session.unwrap().id;
    controls.lock().unwrap()[0].fail_start = false;
    let recovered = engine.start_capture().unwrap();
    assert_eq!(recovered.status, EngineStatus::Recording);
    assert_ne!(recovered.session.unwrap().id, failed_id);
    assert_eq!(recovered.dropped_microphone_blocks, 0);
    engine.shutdown().unwrap();
}

#[test]
fn live_start_failure_is_returned_without_ending_session() {
    let (engine, _directory, controls) = setup();
    engine.start_capture().unwrap();
    controls.lock().unwrap()[1].fail_start = true;
    assert_eq!(
        engine.set_system_audio_enabled(true).unwrap_err().code,
        EngineErrorCode::PermissionDenied
    );
    assert!(engine.snapshot().microphone.active);
    assert_eq!(engine.snapshot().status, EngineStatus::Recording);
    assert!(engine.snapshot().system_audio.enabled);
    engine.shutdown().unwrap();
}

#[test]
fn drop_metrics_accumulate_across_toggles_and_reset_next_session() {
    let (engine, _directory, controls) = setup();
    engine.set_system_audio_enabled(true).unwrap();
    engine.start_capture().unwrap();
    controls.lock().unwrap()[0].drops = 3;
    controls.lock().unwrap()[1].drops = 7;
    let state = engine.send(EngineCommand::GetState).unwrap();
    assert_eq!(
        (state.dropped_microphone_blocks, state.dropped_system_blocks),
        (3, 7)
    );
    engine.set_microphone_enabled(false).unwrap();
    controls.lock().unwrap()[0].drops = 0;
    engine.set_microphone_enabled(true).unwrap();
    controls.lock().unwrap()[0].drops = 2;
    assert_eq!(
        engine
            .send(EngineCommand::GetState)
            .unwrap()
            .dropped_microphone_blocks,
        5
    );
    engine.stop_capture().unwrap();
    controls.lock().unwrap()[0].drops = 0;
    controls.lock().unwrap()[1].drops = 0;
    assert_eq!(engine.start_capture().unwrap().dropped_microphone_blocks, 0);
    engine.shutdown().unwrap();
}

#[test]
fn source_runtime_failure_isolated_then_total_failure_stops_session() {
    let (engine, _directory, controls) = setup();
    let events = engine.subscribe().unwrap();
    engine.set_system_audio_enabled(true).unwrap();
    engine.start_capture().unwrap();
    controls.lock().unwrap()[0].fail_runtime = true;
    let failed = next_snapshot(&events, |s| {
        s.status == EngineStatus::Recording && s.microphone.last_error.is_some()
    });
    assert!(!failed.microphone.active && failed.system_audio.active);
    controls.lock().unwrap()[1].fail_runtime = true;
    let stopped = next_snapshot(&events, |s| s.status == EngineStatus::Error);
    assert!(!stopped.microphone.active && !stopped.system_audio.active);
    assert!(
        std::path::Path::new(&stopped.session.unwrap().recording_directory)
            .join("session.json")
            .exists()
    );
}

#[test]
fn timer_events_and_transcription_availability() {
    let (engine, _directory, _) = setup();
    let events = engine.subscribe().unwrap();
    let state = engine.set_transcription_enabled(true).unwrap();
    assert!(state.transcription.enabled);
    assert!(!state.transcription.available && !state.capabilities.transcription);
    engine.start_capture().unwrap();
    let tick = next_snapshot(&events, |s| {
        s.status == EngineStatus::Recording && s.elapsed_ms >= 500
    });
    assert!(tick.elapsed_ms >= 500);
    engine.stop_capture().unwrap();
}

#[test]
fn missing_parakeet_model_does_not_stop_recording() {
    let directory = tempfile::tempdir().unwrap();
    let controls: Controls = Arc::new(Mutex::new(Default::default()));
    let config = EngineConfig {
        recordings_directory: directory.path().into(),
        transcription_enabled: true,
        transcription: TranscriptionSettings {
            backend: AsrBackendKind::Parakeet,
            model_path: Some(directory.path().join("missing-parakeet-model")),
            ..Default::default()
        },
        ..Default::default()
    };
    let engine = EngineRuntime::with_backend(config, FakeBackend(controls)).unwrap();
    let events = engine.subscribe().unwrap();
    let started = engine.start_capture().unwrap();
    assert_eq!(started.status, EngineStatus::Recording);
    let mut saw_model_error = false;
    for _ in 0..20 {
        if let EngineEvent::TranscriptionError { error } =
            events.recv_timeout(Duration::from_secs(1)).unwrap()
        {
            assert_eq!(error.code, EngineErrorCode::TranscriptionFailed);
            assert!(
                error.message.contains("model path is not configured")
                    || error.message.contains("model")
            );
            saw_model_error = true;
            break;
        }
    }
    assert!(saw_model_error);
    let state = next_snapshot(&events, |state| {
        state.status == EngineStatus::Recording
            && state.transcription.status == TranscriptionStatus::Error
    });
    assert!(!state.transcription.available);
    engine.stop_capture().unwrap();
}

#[test]
fn clone_handles_shutdown_and_final_drop_finalize_recording() {
    let (engine, directory, _) = setup();
    let clone = engine.clone();
    drop(engine);
    let events = clone.subscribe().unwrap();
    let id = clone.start_capture().unwrap().session.unwrap().id;
    drop(clone);
    assert!(directory.path().join(id.0).join("session.json").exists());
    while events.recv().is_ok() {}
    assert_eq!(events.recv(), Err(SubscriptionError::Closed));
}

#[test]
fn shutdown_closes_other_handles_and_is_idempotent() {
    let (engine, _directory, _) = setup();
    let clone = engine.clone();
    engine.shutdown().unwrap();
    clone.shutdown().unwrap();
    assert_eq!(
        clone.send(EngineCommand::GetState).unwrap_err().code,
        EngineErrorCode::RuntimeClosed
    );
    assert!(clone.subscribe().is_err());
}

#[test]
fn configuration_and_session_directory_errors_are_typed() {
    let directory = tempfile::tempdir().unwrap();
    let bad_config = EngineConfig {
        command_capacity: 0,
        ..Default::default()
    };
    assert!(matches!(
        EngineRuntime::new(bad_config),
        Err(EngineError {
            code: EngineErrorCode::InvalidConfiguration,
            ..
        })
    ));
    let file = directory.path().join("not-a-directory");
    std::fs::write(&file, "test").unwrap();
    let config = EngineConfig {
        recordings_directory: file,
        ..Default::default()
    };
    let engine = EngineRuntime::with_backend(
        config,
        FakeBackend(Arc::new(Mutex::new(Default::default()))),
    )
    .unwrap();
    assert_eq!(
        engine.start_capture().unwrap_err().code,
        EngineErrorCode::SessionCreationFailed
    );
    assert_eq!(engine.snapshot().status, EngineStatus::Error);
}

#[test]
fn handles_can_control_same_runtime_from_multiple_threads() {
    let (engine, _directory, _) = setup();
    let a = engine.clone();
    let b = engine.clone();
    let first = std::thread::spawn(move || a.set_transcription_enabled(true).unwrap().revision);
    let second = std::thread::spawn(move || b.set_system_audio_enabled(true).unwrap().revision);
    assert_ne!(first.join().unwrap(), second.join().unwrap());
    assert!(engine.snapshot().transcription.enabled && engine.snapshot().system_audio.enabled);
}
