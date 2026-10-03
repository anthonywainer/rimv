use crate::{
    AudioSource, EngineCommand, EngineConfig, EngineError, EngineErrorCode, EngineSnapshot,
    EngineStatus, Result, SourceState, backend::CaptureBackend, event_bus::EventBus,
    runtime::Request, session::CaptureSession, storage,
};
use speech_transcription::{SpeechEvent, SpeechWorker, load_configured_backend};
use std::sync::mpsc;
use std::{
    sync::{
        Arc,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

const SOURCES: [AudioSource; 2] = [AudioSource::Microphone, AudioSource::System];

pub(crate) struct Controller {
    config: EngineConfig,
    backend: Box<dyn CaptureBackend>,
    bus: Arc<EventBus>,
    state: EngineSnapshot,
    session: Option<CaptureSession>,
    next_tick: Instant,
}
impl Controller {
    pub fn new(config: EngineConfig, backend: Box<dyn CaptureBackend>, bus: Arc<EventBus>) -> Self {
        Self {
            state: bus.snapshot(),
            config,
            backend,
            bus,
            session: None,
            next_tick: Instant::now(),
        }
    }
    pub fn run(mut self, requests: Receiver<Request>) {
        loop {
            let request = if let Some(session) = &self.session {
                let until_tick = self.next_tick.saturating_duration_since(Instant::now());
                let wait = if session.microphone.active() || session.system.active() {
                    Duration::from_millis(5).min(until_tick)
                } else {
                    until_tick
                };
                match requests.recv_timeout(wait) {
                    Ok(request) => Some(request),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match requests.recv() {
                    Ok(request) => Some(request),
                    Err(_) => break,
                }
            };
            if let Some(request) = request {
                match request {
                    Request::Command(command, reply) => {
                        let result = self.command(command);
                        if let Err(error) = &result {
                            self.report(error.clone());
                            self.publish();
                        }
                        let _ = reply.send(result.map(|()| self.state.clone()));
                    }
                    Request::Shutdown(reply) => {
                        let result = if self.session.is_some() {
                            self.finish(EngineStatus::Idle)
                        } else {
                            Ok(())
                        };
                        if let Some(reply) = reply {
                            let _ = reply.send(result);
                        }
                        break;
                    }
                }
            }
            self.poll();
        }
        if self.session.is_some() {
            let _ = self.finish(EngineStatus::Idle);
        }
    }

    fn source(&mut self, source: AudioSource) -> &mut SourceState {
        match source {
            AudioSource::Microphone => &mut self.state.microphone,
            AudioSource::System => &mut self.state.system_audio,
        }
    }
    fn report(&mut self, error: EngineError) {
        if let Some(source) = error.source {
            self.source(source).last_error = Some(error.clone());
        }
        self.state.last_error = Some(error.clone());
        tracing::warn!(%error, source=?error.source, "engine error");
        self.bus.error(error);
    }
    fn refresh(&mut self) {
        if let Some(session) = &self.session {
            self.state.elapsed_ms =
                session.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            self.state.microphone.active = session.microphone.active();
            self.state.system_audio.active = session.system.active();
            self.state.dropped_microphone_blocks = session.microphone.drops();
            self.state.dropped_system_blocks = session.system.drops();
        }
    }
    fn publish(&mut self) {
        self.refresh();
        self.bus.update(&mut self.state);
    }

    fn command(&mut self, command: EngineCommand) -> Result<()> {
        match command {
            EngineCommand::StartCapture => return self.start(),
            EngineCommand::StopCapture => {
                if self.session.is_none() {
                    return Err(EngineError::new(
                        EngineErrorCode::NotRecording,
                        "there is no active session",
                    ));
                }
                return self.finish(EngineStatus::Idle);
            }
            EngineCommand::SetMicrophoneEnabled { enabled } => {
                return self.toggle(AudioSource::Microphone, enabled);
            }
            EngineCommand::SetMicrophoneDevice { device_id } => {
                if self.session.is_some() {
                    return Err(EngineError::new(
                        EngineErrorCode::AlreadyRecording,
                        "change the microphone device after capture stops",
                    ));
                }
                self.config.microphone.configured.device_id = device_id.clone();
                self.state.microphone.configured.device_id = device_id;
            }
            EngineCommand::SetSystemAudioEnabled { enabled } => {
                return self.toggle(AudioSource::System, enabled);
            }
            EngineCommand::SetTranscriptionEnabled { enabled } => {
                self.state.transcription.enabled = enabled;
                self.state.transcription.available =
                    speech_transcription::supports_backend(self.config.transcription.backend)
                        && (self.config.transcription.backend
                            == crate::AsrBackendKind::AppleNative
                            || self.config.transcription.model_path.is_some());
                self.state.capabilities.transcription = self.state.transcription.available;
                // Enabling is a preference change. A worker is only loaded
                // during an active capture session.
                self.state.transcription.status = crate::TranscriptionStatus::Disabled;
                if self.session.is_some() {
                    if enabled {
                        self.start_transcription();
                    } else if let Some(mut session) = self.session.take() {
                        if let Some((mut worker, receiver)) = session.transcription.take() {
                            worker.shutdown();
                            for event in receiver.try_iter() {
                                self.handle_speech_event(&mut session, event);
                            }
                            for update in worker.drain_final_updates() {
                                self.handle_speech_event(&mut session, SpeechEvent::Update(update));
                            }
                        }
                        self.session = Some(session);
                        self.state.transcription.status = crate::TranscriptionStatus::Disabled;
                    }
                }
            }
            EngineCommand::SetTranscriptionModel { path } => {
                let path = std::path::PathBuf::from(path);
                if !path.exists() {
                    return Err(EngineError::new(
                        EngineErrorCode::TranscriptionModelMissing,
                        "selected ASR model path does not exist",
                    ));
                }
                self.config.transcription.model_path = Some(path);
                self.state.transcription.available = true;
                self.state.capabilities.transcription = true;
            }
            EngineCommand::ClearTranscriptionModel => {
                if self.session.is_some() {
                    return Err(EngineError::new(
                        EngineErrorCode::AlreadyRecording,
                        "clear the transcription model after capture stops",
                    ));
                }
                self.config.transcription.model_path = None;
                self.config.transcription_enabled = false;
                self.state.transcription.enabled = false;
                self.state.transcription.available = false;
                self.state.transcription.status = crate::TranscriptionStatus::Disabled;
                self.state.capabilities.transcription = false;
            }
            EngineCommand::SetTranscriptionBackend { backend } => {
                if self.session.is_some() {
                    return Err(EngineError::new(
                        EngineErrorCode::AlreadyRecording,
                        "change the transcription model after capture stops",
                    ));
                }
                self.config.transcription.backend = match backend.as_str() {
                    "parakeet" => crate::AsrBackendKind::Parakeet,
                    "whisper" => crate::AsrBackendKind::Whisper,
                    "native_apple" => crate::AsrBackendKind::AppleNative,
                    _ => {
                        return Err(EngineError::new(
                            EngineErrorCode::InvalidConfiguration,
                            "unknown transcription backend",
                        ));
                    }
                };
                self.state.transcription.available =
                    speech_transcription::supports_backend(self.config.transcription.backend)
                        && (self.config.transcription.backend
                            == crate::AsrBackendKind::AppleNative
                            || self.config.transcription.model_path.is_some());
                self.state.capabilities.transcription = self.state.transcription.available;
            }
            EngineCommand::SetTranscriptionLanguage { language } => {
                if self.session.is_some() {
                    return Err(EngineError::new(
                        EngineErrorCode::AlreadyRecording,
                        "change the transcription language after capture stops",
                    ));
                }
                self.config.transcription.language = language;
            }
            EngineCommand::GetState => {}
        }
        self.publish();
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        if self.session.is_some() {
            return Err(EngineError::new(
                EngineErrorCode::AlreadyRecording,
                "session already running",
            ));
        }
        if !self.state.microphone.enabled && !self.state.system_audio.enabled {
            return Err(EngineError::new(
                EngineErrorCode::NoSourceEnabled,
                "enable a source before starting",
            ));
        }
        self.state.status = EngineStatus::Starting;
        tracing::info!(
            transcription_enabled = self.state.transcription.enabled,
            microphone = self.state.microphone.enabled,
            system_audio = self.state.system_audio.enabled,
            "capture session starting"
        );
        self.state.last_error = None;
        self.state.microphone.last_error = None;
        self.state.system_audio.last_error = None;
        self.state.session = None;
        self.state.elapsed_ms = 0;
        self.state.dropped_microphone_blocks = 0;
        self.state.dropped_system_blocks = 0;
        self.publish();
        let (info, directory) = match storage::create_session(&self.config.recordings_directory) {
            Ok(value) => value,
            Err(error) => {
                self.state.status = EngineStatus::Error;
                return Err(error);
            }
        };
        self.state.session = Some(info);
        self.session = Some(CaptureSession::new(directory));
        if self.state.transcription.enabled {
            self.start_transcription();
            // Do not activate a source until its ASR worker has either loaded
            // or reported a load error. Otherwise native frames arrive while
            // the bounded worker input queue has no consumer, silently losing
            // the beginning of a recording before VAD can see it.
            if let Err(error) = self.wait_for_transcription_initialization()
                && self.config.transcription.preflight_vad
            {
                self.rollback_failed_start();
                return Err(error);
            }
        }
        let mut start_error = None;
        for source in SOURCES {
            if self.source(source).enabled
                && let Err(error) = self.activate(source)
            {
                self.report(error.clone());
                start_error.get_or_insert(error);
            }
        }
        if start_error.is_some() {
            self.rollback_failed_start();
            return Err(EngineError::new(
                EngineErrorCode::NoActiveSource,
                "one or more selected sources failed to start; see source errors",
            ));
        }
        // The session timer starts when best-effort startup finishes. Source
        // clocks remain independent; this is not hardware synchronization.
        if let Some(session) = &mut self.session {
            session.begin_recording();
        }
        self.state.status = EngineStatus::Recording;
        tracing::info!(session_id=?self.state.session.as_ref().map(|session| &session.id), "capture session recording");
        self.next_tick = Instant::now() + self.config.snapshot_interval;
        self.publish();
        Ok(())
    }

    fn rollback_failed_start(&mut self) {
        tracing::info!("rolling back failed capture session");
        let Some(mut session) = self.session.take() else {
            self.state.status = EngineStatus::Error;
            self.state.session = None;
            self.state.elapsed_ms = 0;
            return;
        };
        if let Some((mut worker, _)) = session.transcription.take() {
            worker.shutdown();
        }
        for source in SOURCES {
            if let Err(error) = session.slot(source).rollback() {
                tracing::warn!(%error, source=?error.source, "failed-start source rollback reported an error");
            }
        }
        let directory = session.directory.clone();
        drop(session);
        if let Err(error) = std::fs::remove_dir_all(&directory) {
            tracing::error!(%error, path=%directory.display(), "failed to remove attempted session directory");
        } else {
            tracing::info!(path=%directory.display(), "temporary capture session removed");
        }
        self.state.microphone.active = false;
        self.state.system_audio.active = false;
        self.state.session = None;
        self.state.elapsed_ms = 0;
        self.state.dropped_microphone_blocks = 0;
        self.state.dropped_system_blocks = 0;
        self.state.status = EngineStatus::Error;
        tracing::info!("engine returned to error state after startup rollback");
    }

    fn start_transcription(&mut self) {
        let config = self.config.transcription.to_speech();
        let (events, receiver) = mpsc::sync_channel(config.queue_capacity);
        let loader_config = config.clone();
        match SpeechWorker::start_with_loader(config, events, move || {
            load_configured_backend(loader_config)
        }) {
            Ok(worker) => {
                if let Some(session) = &mut self.session {
                    session.transcription = Some((worker, receiver));
                }
                self.state.transcription.available = true;
                self.state.transcription.status = crate::TranscriptionStatus::Loading;
            }
            Err(error) => {
                self.report(EngineError::new(
                    EngineErrorCode::TranscriptionModelLoadFailed,
                    error.to_string(),
                ));
                self.state.transcription.status = crate::TranscriptionStatus::Error;
            }
        }
    }

    fn wait_for_transcription_initialization(&mut self) -> Result<()> {
        loop {
            let event = self.session.as_ref().and_then(|session| {
                session
                    .transcription
                    .as_ref()
                    .and_then(|(_, receiver)| receiver.recv().ok())
            });
            let Some(event) = event else {
                return Err(EngineError::new(
                    EngineErrorCode::TranscriptionFailed,
                    "transcription worker exited before initialization completed",
                ));
            };
            let failure = match &event {
                SpeechEvent::Error(error) => {
                    let detail = error.to_string();
                    let mut mapped =
                        EngineError::new(EngineErrorCode::TranscriptionFailed, &detail);
                    if self.config.transcription.backend == crate::AsrBackendKind::AppleNative {
                        let normalized = detail.to_ascii_lowercase();
                        mapped.user_message = if normalized.contains("permission") {
                            "RimV does not have permission to use Speech Recognition.".into()
                        } else if normalized.contains("locale") || normalized.contains("on-device")
                        {
                            "Native speech recognition is unavailable for this language on this Mac.".into()
                        } else {
                            "Apple Speech could not start. Check Speech Recognition access and try again.".into()
                        };
                    }
                    Some(mapped)
                }
                _ => None,
            };
            let initialized = matches!(event, SpeechEvent::Ready(_) | SpeechEvent::Error(_));
            if let Some(mut session) = self.session.take() {
                self.handle_speech_event(&mut session, event);
                self.session = Some(session);
            }
            if initialized {
                return if self.config.transcription.preflight_vad {
                    failure.map_or(Ok(()), Err)
                } else {
                    Ok(())
                };
            }
        }
    }

    fn activate(&mut self, source: AudioSource) -> Result<()> {
        let config = self.source(source).configured.clone();
        let supported = match source {
            AudioSource::Microphone => self.state.capabilities.microphone_capture,
            AudioSource::System => self.state.capabilities.system_audio_capture,
        };
        if !supported {
            return Err(EngineError::new(
                EngineErrorCode::Unsupported,
                "source backend is unavailable on this platform",
            )
            .for_source(source));
        }
        if let Some(session) = &mut self.session {
            let started = session.started;
            session
                .slot(source)
                .activate(&mut *self.backend, &config, started)
                .map_err(|mut error| {
                    error.source = Some(source);
                    if error.code == EngineErrorCode::CaptureFailed {
                        error.code = match source {
                            AudioSource::Microphone => EngineErrorCode::MicrophoneStartFailed,
                            AudioSource::System => EngineErrorCode::SystemAudioStartFailed,
                        };
                    }
                    error
                })?;
        }
        self.source(source).last_error = None;
        Ok(())
    }

    fn toggle(&mut self, source: AudioSource, enabled: bool) -> Result<()> {
        self.source(source).enabled = enabled;
        if self.session.is_some() {
            let active = self
                .session
                .as_mut()
                .is_some_and(|s| s.slot(source).active());
            if enabled && !active {
                self.activate(source)?;
            }
            if !enabled
                && active
                && let Some(session) = &mut self.session
            {
                let tail = session.slot(source).deactivate()?;
                Self::finish_speech_source(session, source, tail);
            }
        }
        // Even with both sources deliberately disabled, session and timer remain.
        self.publish();
        Ok(())
    }

    fn finish(&mut self, desired_status: EngineStatus) -> Result<()> {
        self.state.status = EngineStatus::Stopping;
        tracing::info!(desired_status=?desired_status, "capture session stopping and flushing pending speech");
        self.publish();
        let Some(mut session) = self.session.take() else {
            return Ok(());
        };
        let duration = session.started.elapsed();
        self.state.elapsed_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        let mut first_error = None;
        for source in SOURCES {
            match session.slot(source).deactivate() {
                Ok(tail) => Self::finish_speech_source(&mut session, source, tail),
                Err(error) => {
                    first_error.get_or_insert(error.clone());
                    self.report(error);
                }
            }
        }
        self.state.microphone.active = false;
        self.state.system_audio.active = false;
        self.state.dropped_microphone_blocks = session.microphone.drops();
        self.state.dropped_system_blocks = session.system.drops();
        let mut files = Vec::new();
        if let Some((mut worker, receiver)) = session.transcription.take() {
            worker.shutdown();
            while let Ok(event) = receiver.try_recv() {
                self.handle_speech_event(&mut session, event);
            }
            for update in worker.drain_final_updates() {
                self.handle_speech_event(&mut session, SpeechEvent::Update(update));
            }
        }
        if self.state.transcription.status != crate::TranscriptionStatus::Error {
            // The transcription worker (and its loaded model) has now been
            // joined and released. The next capture will load it again.
            self.state.transcription.status = crate::TranscriptionStatus::Disabled;
        }
        for source in SOURCES {
            match session.slot(source).finish(duration) {
                Ok(Some(file)) => files.push(file),
                Ok(None) => {}
                Err(error) => {
                    first_error.get_or_insert(error.clone());
                    self.report(error);
                }
            }
        }
        self.state.status = if first_error.is_some() {
            EngineStatus::Error
        } else {
            desired_status
        };
        // Reserve the final publication revision so metadata equals the final
        // snapshot for a successful stop. A metadata failure adds a new error.
        self.state.revision = self.bus.snapshot().revision.saturating_add(1);
        let transcription_engine =
            (!session.transcript_segments.is_empty()).then_some(
                match self.config.transcription.backend {
                    crate::AsrBackendKind::Parakeet => "parakeet",
                    crate::AsrBackendKind::Whisper => "whisper",
                    crate::AsrBackendKind::AppleNative => "native_apple",
                },
            );
        if let Err(error) = storage::write_metadata(
            &session.directory,
            &self.state,
            &files,
            transcription_engine,
            self.config.transcription.language.as_deref(),
        ) {
            first_error.get_or_insert(error.clone());
            self.report(error);
            self.state.status = EngineStatus::Error;
        }
        if let Err(error) =
            storage::write_transcript(&session.directory, &session.transcript_segments)
        {
            first_error.get_or_insert(error.clone());
            self.report(error);
            self.state.status = EngineStatus::Error;
        }
        self.publish();
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn poll(&mut self) {
        let mut failed = false;
        for source in SOURCES {
            let result = match &mut self.session {
                Some(session) => session.slot(source).poll(),
                None => return,
            };
            match result {
                Ok(frames) => {
                    if let Some(session) = &mut self.session
                        && let Some((worker, _)) = &session.transcription
                    {
                        for frame in frames {
                            worker.try_send(source, frame);
                        }
                        let metrics = worker.metrics();
                        self.state.transcription.dropped_blocks = metrics.dropped_blocks;
                        self.state.transcription.asr = crate::AsrMetrics {
                            input_queue_depth: metrics.queue_depth,
                            inferences: metrics.inferences,
                            queued_work: metrics.queued_work,
                            coalesced_work: metrics.coalesced_work,
                            dropped_work: metrics.dropped_work,
                            dropped_events: metrics.dropped_events,
                            vad_segments: metrics.vad_segments,
                            average_inference_ms: metrics.inference_average_ms as u64,
                            maximum_inference_ms: metrics.inference_max_ms,
                            average_rtf_milli: (metrics.rtf_total * 1000.0
                                / metrics.inferences.max(1) as f64)
                                as u64,
                        };
                    }
                }
                Err(error) => {
                    failed = true;
                    self.report(error.for_source(source));
                    if let Some(session) = &mut self.session {
                        match session.slot(source).deactivate() {
                            Ok(tail) => Self::finish_speech_source(session, source, tail),
                            Err(error) => self.report(error),
                        }
                    }
                }
            }
        }
        self.poll_speech();
        if failed {
            let no_active = self
                .session
                .as_ref()
                .is_some_and(|s| !s.microphone.active() && !s.system.active());
            if no_active {
                let _ = self.finish(EngineStatus::Error);
                return;
            }
            self.publish();
        }
        if self.session.is_some() && Instant::now() >= self.next_tick {
            self.publish();
            self.next_tick = Instant::now() + self.config.snapshot_interval;
        }
    }

    fn finish_speech_source(
        session: &mut CaptureSession,
        source: AudioSource,
        tail: Vec<audio_core::AudioFrame>,
    ) {
        let Some((worker, _)) = &session.transcription else {
            return;
        };
        for frame in tail {
            if worker.send_captured(source, frame).is_err() {
                return;
            }
        }
        let _ = worker.flush_source(source);
    }

    fn poll_speech(&mut self) {
        let mut events = Vec::new();
        if let Some(session) = &mut self.session
            && let Some((worker, receiver)) = &session.transcription
        {
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
            events.extend(
                worker
                    .drain_final_updates()
                    .into_iter()
                    .map(SpeechEvent::Update),
            );
        }
        for event in events {
            if let Some(mut session) = self.session.take() {
                self.handle_speech_event(&mut session, event);
                self.session = Some(session);
            }
        }
    }
    fn handle_speech_event(&mut self, session: &mut CaptureSession, event: SpeechEvent) {
        match event {
            SpeechEvent::Update(update) => {
                tracing::debug!(
                    source=?update.source,
                    utterance_id=%update.utterance_id,
                    start_ms=update.start_ms,
                    end_ms=update.end_ms,
                    final_result=update.is_final,
                    stable_chars=update.stable_text.chars().count(),
                    unstable_chars=update.unstable_text.chars().count(),
                    "shared transcript update accepted"
                );
                if update.is_final {
                    session
                        .transcript_segments
                        .push(speech_transcription::SpeechSegment {
                            source: update.source,
                            start_ms: update.start_ms,
                            end_ms: update.end_ms,
                            text: update.stable_text.clone(),
                        });
                }
                let legacy = crate::TranscriptSegment {
                    source: update.source,
                    start_ms: update.start_ms,
                    end_ms: update.end_ms,
                    text: [&update.stable_text[..], &update.unstable_text[..]]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                if update.is_final {
                    self.bus.transcript_final(legacy);
                } else {
                    self.bus.transcript_partial(legacy);
                }
                self.bus.transcript_update(update);
                self.state.transcription.status = crate::TranscriptionStatus::Transcribing;
            }
            SpeechEvent::Ready(info) => {
                self.state.transcription.available = true;
                self.state.transcription.status = crate::TranscriptionStatus::Ready;
                self.state.transcription.backend = Some(info.backend_name);
                self.state.transcription.model = Some(info.model_name);
                // The shared worker produces provisional results by decoding
                // bounded prefixes even when the backend API itself is offline.
                self.state.transcription.supports_partial_results = true;
                self.state.transcription.supports_true_streaming =
                    info.capabilities.supports_true_streaming;
            }
            SpeechEvent::Partial(segment) => {
                self.bus.transcript_partial(crate::TranscriptSegment {
                    source: segment.source,
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    text: segment.text,
                })
            }
            SpeechEvent::Final(segment) => {
                let protocol = crate::TranscriptSegment {
                    source: segment.source,
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    text: segment.text.clone(),
                };
                session.transcript_segments.push(segment);
                self.bus.transcript_final(protocol);
                self.state.transcription.status = crate::TranscriptionStatus::Transcribing;
            }
            SpeechEvent::Error(error) => {
                let error =
                    EngineError::new(EngineErrorCode::TranscriptionFailed, error.to_string());
                self.bus.transcription_error(error.clone());
                self.report(error);
                self.state.transcription.available = false;
                self.state.transcription.status = crate::TranscriptionStatus::Error;
            }
        }
    }
}
