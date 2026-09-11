use crate::{
    EngineCapabilities, EngineCommand, EngineConfig, EngineError, EngineErrorCode, EngineSnapshot,
    FeatureState, Result, SourceState, Subscription,
    backend::{CaptureBackend, NativeBackend},
    controller::Controller,
    event_bus::EventBus,
    lock,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
};

pub(crate) enum Request {
    Command(EngineCommand, SyncSender<Result<EngineSnapshot>>),
    Shutdown(Option<SyncSender<Result<()>>>),
}
struct Handle {
    sender: SyncSender<Request>,
    bus: Arc<EventBus>,
    shutting_down: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.shutting_down.swap(true, Ordering::AcqRel) {
            let _ = self.sender.send(Request::Shutdown(None));
        }
        if let Some(worker) = lock(&self.worker).take() {
            let _ = worker.join();
        }
    }
}

/// Cloneable control handle. Calls are serialized on a single worker. `send`
/// waits for execution, returns the resulting snapshot, and may wait for OS
/// permissions; UI clients should issue it from a control/background thread.
/// Snapshot reads and subscriptions do not wait for native capture startup.
#[derive(Clone)]
pub struct EngineRuntime {
    inner: Arc<Handle>,
}
impl EngineRuntime {
    pub fn new(config: EngineConfig) -> Result<Self> {
        Self::with_backend(config, NativeBackend)
    }

    pub fn with_backend(config: EngineConfig, backend: impl CaptureBackend) -> Result<Self> {
        config.validate()?;
        let mut capabilities = backend.capabilities();
        capabilities.transcription = false;
        let initial = EngineSnapshot {
            microphone: SourceState {
                configured: config.microphone.configured.clone(),
                enabled: config.microphone.enabled,
                ..Default::default()
            },
            system_audio: SourceState {
                configured: config.system_audio.configured.clone(),
                enabled: config.system_audio.enabled,
                ..Default::default()
            },
            transcription: FeatureState {
                enabled: config.transcription_enabled,
                available: config.transcription.model_path.is_some(),
                status: if config.transcription_enabled {
                    crate::TranscriptionStatus::Loading
                } else {
                    crate::TranscriptionStatus::Disabled
                },
                ..Default::default()
            },
            capabilities,
            ..Default::default()
        };
        let bus = Arc::new(EventBus::new(
            initial,
            config.subscriber_capacity,
            config.max_subscribers,
        ));
        let (sender, requests) = mpsc::sync_channel(config.command_capacity);
        let worker_bus = bus.clone();
        let worker = thread::Builder::new()
            .name("rimv-runtime".into())
            .spawn(move || {
                struct Close(Arc<EventBus>);
                impl Drop for Close {
                    fn drop(&mut self) {
                        self.0.close();
                    }
                }
                let _close = Close(worker_bus.clone());
                Controller::new(config, Box::new(backend), worker_bus).run(requests);
            })
            .map_err(|e| EngineError::new(EngineErrorCode::WorkerFailed, e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Handle {
                sender,
                bus,
                shutting_down: AtomicBool::new(false),
                worker: Mutex::new(Some(worker)),
            }),
        })
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        self.inner.bus.snapshot()
    }
    pub fn capabilities(&self) -> EngineCapabilities {
        self.snapshot().capabilities
    }
    pub fn subscribe(&self) -> Result<Subscription> {
        self.inner.bus.subscribe()
    }
    pub fn send(&self, command: EngineCommand) -> Result<EngineSnapshot> {
        if self.inner.shutting_down.load(Ordering::Acquire) {
            return Err(closed());
        }
        let (reply, result) = mpsc::sync_channel(1);
        self.inner
            .sender
            .try_send(Request::Command(command, reply))
            .map_err(|error| match error {
                TrySendError::Full(_) => EngineError::new(
                    EngineErrorCode::CommandQueueFull,
                    "command queue is full; retry later",
                ),
                TrySendError::Disconnected(_) => closed(),
            })?;
        result.recv().map_err(|_| closed())?
    }
    pub fn start_capture(&self) -> Result<EngineSnapshot> {
        self.send(EngineCommand::StartCapture)
    }
    pub fn stop_capture(&self) -> Result<EngineSnapshot> {
        self.send(EngineCommand::StopCapture)
    }
    pub fn set_microphone_enabled(&self, enabled: bool) -> Result<EngineSnapshot> {
        self.send(EngineCommand::SetMicrophoneEnabled { enabled })
    }
    pub fn set_system_audio_enabled(&self, enabled: bool) -> Result<EngineSnapshot> {
        self.send(EngineCommand::SetSystemAudioEnabled { enabled })
    }
    pub fn set_transcription_enabled(&self, enabled: bool) -> Result<EngineSnapshot> {
        self.send(EngineCommand::SetTranscriptionEnabled { enabled })
    }

    /// Stop/finalize any session, join the worker, and close all subscriptions.
    /// Shuts down all cloned handles. Dropping the final handle also shuts down.
    pub fn shutdown(&self) -> Result<()> {
        let result = if !self.inner.shutting_down.swap(true, Ordering::AcqRel) {
            let (reply, response) = mpsc::sync_channel(1);
            self.inner
                .sender
                .send(Request::Shutdown(Some(reply)))
                .map_err(|_| closed())
                .and_then(|()| response.recv().map_err(|_| closed()))
                .and_then(|result| result)
        } else {
            Ok(())
        };
        if let Some(worker) = lock(&self.inner.worker).take() {
            worker.join().map_err(|_| {
                EngineError::new(EngineErrorCode::WorkerFailed, "runtime worker panicked")
            })?;
        }
        result
    }
}
fn closed() -> EngineError {
    EngineError::new(
        EngineErrorCode::RuntimeClosed,
        "runtime is shut down or worker exited",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioSource, SourceConfig, backend::CaptureStream};

    struct GatedBackend {
        entered: SyncSender<()>,
        release: mpsc::Receiver<()>,
    }
    impl CaptureBackend for GatedBackend {
        fn capabilities(&self) -> EngineCapabilities {
            EngineCapabilities {
                microphone_capture: true,
                ..Default::default()
            }
        }
        fn open(&mut self, _: AudioSource, _: &SourceConfig) -> Result<Box<dyn CaptureStream>> {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
            Err(EngineError::new(
                EngineErrorCode::PermissionDenied,
                "test denied",
            ))
        }
    }

    #[test]
    fn bounded_commands_do_not_block_snapshot_or_subscribe_during_startup() {
        let directory = tempfile::tempdir().unwrap();
        let (entered, entered_rx) = mpsc::sync_channel(0);
        let (release, release_rx) = mpsc::sync_channel(0);
        let config = EngineConfig {
            command_capacity: 1,
            recordings_directory: directory.path().into(),
            ..Default::default()
        };
        let engine = EngineRuntime::with_backend(
            config,
            GatedBackend {
                entered,
                release: release_rx,
            },
        )
        .unwrap();
        let starter = engine.clone();
        let pending = std::thread::spawn(move || starter.start_capture());
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(engine.snapshot().status, crate::EngineStatus::Starting);
        assert!(engine.subscribe().is_ok());
        // Deterministically fill the one-slot queue while native startup waits.
        let (reply, result) = mpsc::sync_channel(1);
        assert!(
            engine
                .inner
                .sender
                .try_send(Request::Command(EngineCommand::GetState, reply))
                .is_ok()
        );
        assert_eq!(
            engine.send(EngineCommand::GetState).unwrap_err().code,
            EngineErrorCode::CommandQueueFull
        );
        release.send(()).unwrap();
        assert_eq!(
            pending.join().unwrap().unwrap_err().code,
            EngineErrorCode::NoActiveSource
        );
        assert_eq!(
            result.recv().unwrap().unwrap().status,
            crate::EngineStatus::Error
        );
        engine.shutdown().unwrap();
    }
}
