use crate::{
    AsrBackendInfo, AsrBackendKind, Mono16k, Result, SpeechError, SpeechSegment, SpeechSegmenter,
    VadConfig, VoiceActivityGate,
};
use audio_core::AudioFrame;
use engine_protocol::AudioSource;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

/// Backend boundary. Implementations are loaded once and used only by the
/// worker thread, never by capture callbacks.
pub trait SpeechToTextEngine: Send + 'static {
    fn info(&self) -> AsrBackendInfo;

    fn transcribe(
        &mut self,
        audio_16khz_mono: &[f32],
        offset_ms: u64,
    ) -> Result<Vec<SpeechSegment>>;
}

impl<T: SpeechToTextEngine + ?Sized> SpeechToTextEngine for Box<T> {
    fn info(&self) -> AsrBackendInfo {
        (**self).info()
    }

    fn transcribe(
        &mut self,
        audio_16khz_mono: &[f32],
        offset_ms: u64,
    ) -> Result<Vec<SpeechSegment>> {
        (**self).transcribe(audio_16khz_mono, offset_ms)
    }
}

#[derive(Debug, Clone)]
pub struct SpeechConfig {
    pub backend: AsrBackendKind,
    pub model_path: Option<PathBuf>,
    pub language: Option<String>,
    pub threads: usize,
    pub use_gpu: bool,
    pub provider: String,
    /// Retained for configuration compatibility. VAD utterances replaced
    /// fixed rolling windows in v0.5.
    pub window_ms: u64,
    /// Retained for configuration compatibility. VAD utterances do not overlap.
    pub step_ms: u64,
    pub queue_capacity: usize,
    pub vad: VadConfig,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            backend: AsrBackendKind::Parakeet,
            model_path: std::env::var_os("RIMV_PARAKEET_MODEL_DIR").map(PathBuf::from),
            language: None,
            threads: 4,
            use_gpu: cfg!(target_os = "macos"),
            provider: std::env::var("RIMV_ASR_PROVIDER").unwrap_or_else(|_| "cpu".into()),
            window_ms: 6_000,
            step_ms: 3_000,
            queue_capacity: 64,
            vad: VadConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpeechMetrics {
    pub dropped_blocks: u64,
    pub queue_depth: usize,
    pub model_load_ms: u64,
    pub first_latency_ms: Option<u64>,
    pub inferences: u64,
    pub queued_work: usize,
    pub coalesced_work: u64,
    pub dropped_work: u64,
    pub dropped_events: u64,
    pub vad_segments: u64,
    pub inference_average_ms: f64,
    pub inference_total_ms: u64,
    pub inference_max_ms: u64,
    pub rtf_total: f64,
    pub rtf_max: f64,
}

#[derive(Debug, Clone)]
pub enum SpeechEvent {
    Update(crate::TranscriptUpdate),
    Partial(SpeechSegment),
    Final(SpeechSegment),
    Error(SpeechError),
    Ready(AsrBackendInfo),
}

enum Input {
    Frame(AudioSource, AudioFrame),
    FlushSource(AudioSource),
    Stop,
}

pub struct SpeechWorker {
    sender: SyncSender<Input>,
    input_depth: Arc<AtomicUsize>,
    metrics: Arc<Mutex<SpeechMetrics>>,
    join: Option<JoinHandle<()>>,
    final_overflow: Arc<Mutex<std::collections::VecDeque<crate::TranscriptUpdate>>>,
}

impl SpeechWorker {
    pub fn start(
        engine: impl SpeechToTextEngine,
        config: SpeechConfig,
        events: SyncSender<SpeechEvent>,
    ) -> Result<Self> {
        Self::spawn(config, events, move || Ok(Box::new(engine)))
    }

    /// Starts a worker whose model loader runs on the worker thread. Capture
    /// callers only enqueue frames and are never blocked by model or VAD I/O.
    pub fn start_with_loader<E, F>(
        config: SpeechConfig,
        events: SyncSender<SpeechEvent>,
        loader: F,
    ) -> Result<Self>
    where
        E: SpeechToTextEngine,
        F: FnOnce() -> Result<E> + Send + 'static,
    {
        Self::spawn(config, events, move || {
            loader().map(|engine| Box::new(engine) as Box<dyn SpeechToTextEngine>)
        })
    }

    fn spawn<F>(config: SpeechConfig, events: SyncSender<SpeechEvent>, loader: F) -> Result<Self>
    where
        F: FnOnce() -> Result<Box<dyn SpeechToTextEngine>> + Send + 'static,
    {
        if config.queue_capacity == 0 {
            return Err(SpeechError::Preprocess(
                "speech queue capacity must be greater than zero".into(),
            ));
        }
        config.vad.validate()?;
        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let metrics = Arc::new(Mutex::new(SpeechMetrics::default()));
        let worker_metrics = metrics.clone();
        let input_depth = Arc::new(AtomicUsize::new(0));
        let worker_depth = input_depth.clone();
        let final_overflow = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        let worker_overflow = final_overflow.clone();
        let join = thread::Builder::new()
            .name("speech-transcription".into())
            .spawn(move || {
                let started = Instant::now();
                let engine = match loader() {
                    Ok(engine) => engine,
                    Err(error) => {
                        let _ = events.try_send(SpeechEvent::Error(error));
                        return;
                    }
                };
                worker_metrics
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .model_load_ms = started.elapsed().as_millis() as u64;
                let _ = events.try_send(SpeechEvent::Ready(engine.info()));

                let decoder = crate::realtime::Decoder::start(
                    engine,
                    events.clone(),
                    worker_metrics.clone(),
                    worker_overflow,
                );
                let mut preprocessors = HashMap::new();
                let mut segmenters =
                    HashMap::<AudioSource, SpeechSegmenter<Box<dyn VoiceActivityGate>>>::new();
                let mut vad_failures = HashSet::new();

                while let Ok(input) = receiver.recv() {
                    match input {
                        Input::FlushSource(source) => {
                            if let Some(segmenter) = segmenters.get_mut(&source) {
                                match segmenter.flush() {
                                    Ok(Some(utterance)) => decoder.submit(source, utterance, true),
                                    Ok(None) => {}
                                    Err(error) => {
                                        let _ = events.try_send(SpeechEvent::Error(error));
                                    }
                                }
                            }
                        }
                        Input::Stop => {
                            for (source, segmenter) in &mut segmenters {
                                match segmenter.flush() {
                                    Ok(Some(utterance)) => decoder.submit(*source, utterance, true),
                                    Ok(None) => {}
                                    Err(error) => {
                                        let _ = events.try_send(SpeechEvent::Error(error));
                                    }
                                }
                            }
                            break;
                        }
                        Input::Frame(source, frame) => {
                            worker_depth.fetch_sub(1, Ordering::Relaxed);
                            let rate = frame.format().sample_rate();
                            let preprocessor = match preprocessors.entry(source) {
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    match crate::Preprocessor::new(rate) {
                                        Ok(preprocessor) => entry.insert(preprocessor),
                                        Err(error) => {
                                            let _ = events.try_send(SpeechEvent::Error(error));
                                            continue;
                                        }
                                    }
                                }
                                std::collections::hash_map::Entry::Occupied(entry) => {
                                    entry.into_mut()
                                }
                            };
                            let blocks = match preprocessor.push(&frame) {
                                Ok(blocks) => blocks,
                                Err(error) => {
                                    let _ = events.try_send(SpeechEvent::Error(error));
                                    continue;
                                }
                            };

                            if !segmenters.contains_key(&source) && !vad_failures.contains(&source)
                            {
                                match load_source_vad(&config.vad)
                                    .and_then(|vad| SpeechSegmenter::new(vad, config.vad.clone()))
                                {
                                    Ok(segmenter) => {
                                        segmenters.insert(source, segmenter);
                                    }
                                    Err(error) => {
                                        vad_failures.insert(source);
                                        let _ = events.try_send(SpeechEvent::Error(error));
                                    }
                                }
                            }
                            let Some(segmenter) = segmenters.get_mut(&source) else {
                                continue;
                            };
                            for Mono16k { start_ms, samples } in blocks {
                                match segmenter.push(start_ms, &samples) {
                                    Ok(utterances) => {
                                        for utterance in utterances {
                                            decoder.submit(source, utterance, true);
                                        }
                                    }
                                    Err(error) => {
                                        let _ = events.try_send(SpeechEvent::Error(error));
                                    }
                                }
                                if let Some((start, audio)) = segmenter.active_audio() {
                                    decoder.partial(source, start, audio);
                                }
                            }
                        }
                    }
                }
                decoder.finish();
            })
            .map_err(|_| SpeechError::Closed)?;
        Ok(Self {
            sender,
            input_depth,
            metrics,
            join: Some(join),
            final_overflow,
        })
    }

    pub fn try_send(&self, source: AudioSource, frame: AudioFrame) {
        self.input_depth.fetch_add(1, Ordering::Relaxed);
        match self.sender.try_send(Input::Frame(source, frame)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.input_depth.fetch_sub(1, Ordering::Relaxed);
                self.metrics
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .dropped_blocks += 1;
            }
            Err(TrySendError::Disconnected(_)) => {
                self.input_depth.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Ordered delivery for frames already captured while a native source was
    /// stopping. This is called outside capture callbacks after the source has
    /// stopped, so the utterance flush cannot overtake its final audio.
    pub fn send_captured(&self, source: AudioSource, frame: AudioFrame) -> Result<()> {
        self.input_depth.fetch_add(1, Ordering::Relaxed);
        self.sender.send(Input::Frame(source, frame)).map_err(|_| {
            self.input_depth.fetch_sub(1, Ordering::Relaxed);
            SpeechError::Closed
        })
    }

    pub fn flush_source(&self, source: AudioSource) -> Result<()> {
        self.sender
            .send(Input::FlushSource(source))
            .map_err(|_| SpeechError::Closed)
    }

    /// Recover finals retained when the public event channel was full.
    pub fn drain_final_updates(&self) -> Vec<crate::TranscriptUpdate> {
        self.final_overflow.lock().unwrap().drain(..).collect()
    }

    pub fn metrics(&self) -> SpeechMetrics {
        let mut metrics = self
            .metrics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        metrics.queue_depth = self.input_depth.load(Ordering::Relaxed);
        metrics
    }

    pub fn shutdown(&mut self) {
        let _ = self.sender.send(Input::Stop);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for SpeechWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(feature = "silero-vad")]
fn load_source_vad(config: &VadConfig) -> Result<Box<dyn VoiceActivityGate>> {
    crate::SileroVad::load(config).map(|vad| Box::new(vad) as Box<dyn VoiceActivityGate>)
}

#[cfg(not(feature = "silero-vad"))]
fn load_source_vad(_config: &VadConfig) -> Result<Box<dyn VoiceActivityGate>> {
    Err(SpeechError::Vad(
        "Silero VAD support is not compiled".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockAsrBackend;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn backend_loader_runs_once_and_worker_reuses_the_backend() {
        let loads = Arc::new(AtomicUsize::new(0));
        let worker_loads = loads.clone();
        let (events, receiver) = mpsc::sync_channel(4);
        let mut worker =
            SpeechWorker::start_with_loader(SpeechConfig::default(), events, move || {
                worker_loads.fetch_add(1, Ordering::SeqCst);
                Ok(MockAsrBackend::default())
            })
            .unwrap();
        assert!(matches!(receiver.recv().unwrap(), SpeechEvent::Ready(_)));
        worker.shutdown();
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }
}
