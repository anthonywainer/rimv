use clap::{Args, Parser, Subcommand, ValueEnum};
use engine_protocol::{EngineEvent, EngineSnapshot};
use engine_runtime::{AsrBackendKind, EngineConfig, EngineRuntime};
use model_manager::ModelManager;
use speech_transcription::{SpeechConfig, load_configured_backend};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

mod benchmark;
mod media;
mod terminal_output;
mod transcript_renderer;

use terminal_output::{DiagnosticWriter, PendingPartials, REFRESH_INTERVAL, SharedTerminal};
use transcript_renderer::TranscriptRenderer;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(
    name = "rimv",
    version,
    about = "Real-time Intelligent Multilingual Voice"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Listen to microphone, system audio, or both through the runtime.
    Listen(ListenArgs),
    /// Transcribe one local MP3, WAV, or FLAC file with the selected ASR backend.
    Transcribe(TranscribeArgs),
    /// Print local runtime, capture, and model configuration facts.
    Doctor(DoctorArgs),
    /// Inspect configured local ASR model paths.
    Models(ModelsArgs),
    /// List native input/output devices and supported formats.
    Devices,
    /// Start the local runtime WebSocket bridge.
    Serve(ServeArgs),
    /// Evaluate a corpus with direct-file or real system-capture input.
    Benchmark(BenchmarkArgs),
}

#[derive(Args)]
struct ListenArgs {
    #[arg(long, conflicts_with_all = ["system", "both"])]
    mic: bool,
    #[arg(long, conflicts_with_all = ["mic", "both"])]
    system: bool,
    #[arg(long, conflicts_with_all = ["mic", "system"])]
    both: bool,
    #[arg(long)]
    model: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
    #[arg(long, visible_alias = "lang", default_value = "auto")]
    language: String,
    #[arg(long, default_value = "cpu")]
    provider: String,
    #[arg(long)]
    vad_threshold: Option<f32>,
    #[arg(long, value_enum, default_value_t = Profile::Balanced)]
    profile: Profile,
    #[arg(long)]
    show_partials: bool,
    #[arg(long)]
    show_metrics: bool,
    #[arg(long, default_value = "recordings")]
    recordings: PathBuf,
    /// Stop automatically after this many seconds. Zero means run until Ctrl-C.
    #[arg(long, default_value_t = 0)]
    seconds: u64,
}

#[derive(Args)]
struct TranscribeArgs {
    input: PathBuf,
    #[arg(long)]
    model: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
    #[arg(long, visible_alias = "lang", default_value = "auto")]
    language: String,
    #[arg(long, default_value = "cpu")]
    provider: String,
    #[arg(long)]
    cpu: bool,
}

#[derive(Args)]
struct DoctorArgs {
    #[arg(long)]
    model: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
}

#[derive(Args)]
struct ModelsArgs {
    #[command(subcommand)]
    command: Option<ModelsCommand>,
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
    #[arg(long)]
    model: Option<PathBuf>,
}

#[derive(Subcommand)]
enum ModelsCommand {
    /// Download the default Parakeet ASR and Silero VAD models.
    Install,
}

#[derive(Args)]
struct ServeArgs {
    #[arg(long, default_value_t = 7878)]
    port: u16,
    #[arg(long, default_value = "recordings")]
    recordings: PathBuf,
}

#[derive(Args)]
struct BenchmarkArgs {
    #[arg(long, default_value = "resources/audio")]
    corpus: PathBuf,
    #[arg(long, value_enum, default_value_t = benchmark::BenchmarkMode::RealtimeCapture)]
    mode: benchmark::BenchmarkMode,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    model: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
    #[arg(long, visible_alias = "lang", default_value = "auto")]
    language: String,
    #[arg(long, default_value = "cpu")]
    provider: String,
    #[arg(long)]
    cpu: bool,
    #[cfg(target_os = "macos")]
    #[arg(long, default_value_t = 500)]
    warmup_ms: u64,
    #[cfg(target_os = "macos")]
    #[arg(long, default_value_t = 1_500)]
    trailing_ms: u64,
    #[arg(long, default_value_t = 30)]
    silence_seconds: u64,
    #[arg(long)]
    skip_silence: bool,
    /// Fail after report generation when a scored case exceeds this word-error rate.
    #[arg(long)]
    max_wer: Option<f64>,
}

struct RuntimeTranscriptionOptions {
    backend: Backend,
    model: Option<PathBuf>,
    language: Option<String>,
    provider: String,
    profile: Profile,
    vad_threshold: Option<f32>,
    cpu: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Backend {
    Parakeet,
    Whisper,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Profile {
    Realtime,
    Balanced,
    Accurate,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let terminal = match &cli.command {
        Command::Listen(args) => Some(Arc::new(Mutex::new(TranscriptRenderer::new(
            io::stdout(),
            args.show_partials && io::stdout().is_terminal(),
        )))),
        _ => None,
    };
    let log_terminal = terminal.clone();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(move || DiagnosticWriter::new(log_terminal.clone(), io::stderr()))
        .init();

    match run(cli, terminal) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rimv: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, terminal: Option<SharedTerminal<io::Stdout>>) -> Result<()> {
    match cli.command {
        Command::Listen(args) => listen(args, terminal.ok_or("listen terminal missing")?),
        Command::Transcribe(args) => transcribe(args),
        Command::Doctor(args) => doctor(args),
        Command::Models(args) => models(args),
        Command::Devices => devices(),
        Command::Serve(args) => serve(args),
        Command::Benchmark(args) => benchmark(args),
    }
}

fn listen(args: ListenArgs, terminal: SharedTerminal<io::Stdout>) -> Result<()> {
    let stop = stop_flag()?;
    let mut config = EngineConfig {
        recordings_directory: args.recordings,
        transcription_enabled: true,
        ..Default::default()
    };
    config.microphone.enabled = args.mic || args.both || !args.system;
    config.system_audio.enabled = args.system || args.both;
    apply_transcription_options(
        &mut config.transcription,
        RuntimeTranscriptionOptions {
            backend: args.backend,
            model: args.model,
            language: Some(args.language),
            provider: args.provider,
            profile: args.profile,
            vad_threshold: args.vad_threshold,
            cpu: false,
        },
    );

    let engine = EngineRuntime::new(config)?;
    let events = engine.subscribe()?;
    let printer_terminal = terminal.clone();
    let printer = thread::Builder::new()
        .name("rimv-listen-events".into())
        .spawn(move || print_events(|timeout| events.recv_timeout(timeout), printer_terminal))?;

    if let Err(error) = engine.start_capture() {
        let _ = engine.shutdown();
        let _ = printer.join();
        return Err(error.into());
    }
    let started = Instant::now();
    while !stop.load(Ordering::Relaxed)
        && (args.seconds == 0 || started.elapsed() < Duration::from_secs(args.seconds))
    {
        thread::sleep(Duration::from_millis(100));
    }
    let stopped = engine.stop_capture();
    let snapshot = engine.snapshot();
    let shutdown = engine.shutdown();
    let printed = printer
        .join()
        .map_err(|_| "listen event printer panicked")?;
    shutdown?;
    printed?;
    stopped?;
    println!("stopped: {}", status_line(&snapshot));
    if args.show_metrics {
        for line in metrics_lines(&snapshot) {
            println!("{line}");
        }
    }
    Ok(())
}

fn print_events<W: Write>(
    mut receive: impl FnMut(
        Duration,
    ) -> std::result::Result<EngineEvent, engine_runtime::SubscriptionError>,
    terminal: SharedTerminal<W>,
) -> io::Result<()> {
    let mut pending = PendingPartials::default();
    let mut next_refresh = Instant::now() + REFRESH_INTERVAL;
    // The runtime closes the subscription only after publishing its final
    // updates. Ctrl-C stops capture, but must not stop this drain prematurely.
    let result = (|| {
        terminal
            .lock()
            .map_err(|_| io::Error::other("terminal lock poisoned"))?
            .message("listening; press Ctrl-C to stop")?;
        loop {
            match receive(next_refresh.saturating_duration_since(Instant::now())) {
                Ok(EngineEvent::TranscriptUpdate { update }) => {
                    pending.accept(&terminal, update)?;
                }
                Ok(EngineEvent::TranscriptionError { error })
                | Ok(EngineEvent::Error { error }) => {
                    let mut diagnostic =
                        DiagnosticWriter::new(Some(terminal.clone()), io::stderr());
                    writeln!(diagnostic, "error: {error}")?;
                    diagnostic.flush()?;
                }
                Ok(_) => {}
                Err(engine_runtime::SubscriptionError::Timeout) => {}
                Err(engine_runtime::SubscriptionError::Closed) => break,
                Err(engine_runtime::SubscriptionError::Lagged { missed }) => {
                    terminal
                        .lock()
                        .map_err(|_| io::Error::other("terminal lock poisoned"))?
                        .message(&format!("warning: missed {missed} engine events"))?;
                }
            }
            if Instant::now() >= next_refresh {
                pending.refresh(&terminal)?;
                next_refresh = Instant::now() + REFRESH_INTERVAL;
            }
        }
        Ok(())
    })();
    let finished = terminal
        .lock()
        .map_err(|_| io::Error::other("terminal lock poisoned"))?
        .finish();
    result.and(finished)
}

fn transcribe(args: TranscribeArgs) -> Result<()> {
    let audio = media::decode_mono_16k(&args.input)?.samples;
    let mut config = SpeechConfig {
        backend: args.backend.into(),
        model_path: args.model.or_else(default_model_path),
        provider: args.provider,
        use_gpu: !args.cpu,
        ..Default::default()
    };
    config.language = language(args.language);
    let mut engine = load_configured_backend(config)?;
    let info = engine.info();
    eprintln!("backend: {} ({})", info.backend_name, info.model_name);
    let segments = engine.transcribe(&audio, 0)?;
    for segment in segments {
        println!(
            "[{:>7}-{:>7} ms] {}",
            segment.start_ms, segment.end_ms, segment.text
        );
    }
    Ok(())
}

fn doctor(args: DoctorArgs) -> Result<()> {
    let profile = system_profile::SystemProfile::detect();
    println!("rimv doctor");
    println!("os: {} {}", std::env::consts::OS, profile.os_version);
    println!("arch: {:?}", profile.architecture);
    println!(
        "memory: {:.1} GiB",
        profile.physical_memory_bytes as f64 / 1024.0_f64.powi(3)
    );
    println!("cpu cores: {} logical", profile.logical_cpu_count);
    println!("apple silicon: {}", profile.apple_silicon);
    println!("metal: {}", profile.metal_available);
    let capabilities = EngineRuntime::new(EngineConfig::default())?.capabilities();
    println!("microphone capture: {}", capabilities.microphone_capture);
    println!(
        "system audio capture: {}",
        capabilities.system_audio_capture
    );
    let model = args.model.or_else(default_model_path);
    println!("transcription support: true");
    println!("transcription model configured: {}", model.is_some());
    println!("backend: {:?}", args.backend);
    println!(
        "model: {}",
        model.as_ref().map_or_else(
            || "not configured".into(),
            |path| path.display().to_string()
        )
    );
    Ok(())
}

fn models(args: ModelsArgs) -> Result<()> {
    let root = model_root();
    let manager = ModelManager::new(&root);
    if matches!(args.command, Some(ModelsCommand::Install)) {
        install_models(&manager)?;
        return Ok(());
    }
    println!("backend: {:?}", args.backend);
    let configured = args.model.or_else(default_model_path);
    println!(
        "configured model: {}",
        configured.as_ref().map_or_else(
            || "not configured".into(),
            |path| path.display().to_string()
        )
    );
    if let Some(path) = configured {
        println!("exists: {}", path.exists());
        println!("kind: {}", if path.is_dir() { "directory" } else { "file" });
    }
    println!(
        "env RIMV_PARAKEET_MODEL_DIR: {}",
        env_path("RIMV_PARAKEET_MODEL_DIR")
    );
    println!(
        "env RIMV_SILERO_VAD_MODEL: {}",
        env_path("RIMV_SILERO_VAD_MODEL")
    );
    println!("model storage: {}", root.display());
    for descriptor in manager.catalog() {
        println!(
            "{}: backend={} state={:?} languages={}",
            descriptor.id,
            descriptor.backend,
            manager.state(descriptor),
            if descriptor.languages.is_empty() {
                "n/a".into()
            } else {
                descriptor.languages.join(",")
            }
        );
    }
    Ok(())
}

fn install_models(manager: &ModelManager) -> Result<()> {
    let cancelled = AtomicBool::new(false);
    for (id, label) in [
        ("parakeet-tdt-0.6b-v3-int8", "Parakeet ASR (about 640 MB)"),
        ("silero-vad", "Silero VAD"),
    ] {
        let descriptor = manager.descriptor(id)?;
        if manager.state(descriptor) == model_manager::ModelState::Ready {
            println!("{label} is already installed.");
            continue;
        }
        eprintln!("Installing {label}...");
        let mut reported = 0_u64;
        let mut progress = |written: u64, _total: Option<u64>| {
            if written.saturating_sub(reported) >= 8 * 1024 * 1024 {
                eprint!("\rDownloaded {:.0} MiB", written as f64 / 1024.0 / 1024.0);
                reported = written;
            }
        };
        let path = if id == "parakeet-tdt-0.6b-v3-int8" {
            manager.install_parakeet(&cancelled, &mut progress)?
        } else {
            manager.install(id, &cancelled, &mut progress)?
        };
        eprintln!("\rInstalled {label}: {}", path.display());
    }
    println!("Models are ready. Run `rimv doctor` to verify the runtime configuration.");
    Ok(())
}

fn devices() -> Result<()> {
    for (label, devices) in [
        ("Inputs", audio_capture::list_microphones()?),
        ("Outputs", audio_capture::list_output_devices()?),
    ] {
        println!("{label}:");
        for device in devices {
            println!(
                "  {} {} ({})",
                if device.is_default { "*" } else { " " },
                device.name,
                device.id
            );
            let configs = if label == "Inputs" {
                audio_capture::supported_microphone_configs(&device.id)
            } else {
                audio_capture::supported_output_configs(&device.id)
            };
            match configs {
                Ok(configs) => {
                    for config in configs {
                        println!(
                            "      {} ch, {}..={} Hz, {}",
                            config.channels,
                            config.min_sample_rate,
                            config.max_sample_rate,
                            config.sample_format
                        );
                    }
                }
                Err(error) => println!("      configs unavailable: {error}"),
            }
        }
    }
    Ok(())
}

fn serve(args: ServeArgs) -> Result<()> {
    let stop = stop_flag()?;
    let config = EngineConfig {
        recordings_directory: args.recordings,
        ..Default::default()
    };
    let engine = EngineRuntime::new(config)?;
    engine_runtime::websocket::attach_local_websocket(engine.clone(), args.port)
        .map_err(|error| format!("could not start local web bridge: {error}"))?;
    println!("serving ws://127.0.0.1:{}; press Ctrl-C to stop", args.port);
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(250));
    }
    engine.shutdown()?;
    Ok(())
}

fn benchmark(args: BenchmarkArgs) -> Result<()> {
    benchmark::run(benchmark::BenchmarkOptions {
        corpus: args.corpus,
        mode: args.mode,
        output: args.output,
        model: args.model.or_else(default_model_path),
        backend: args.backend.into(),
        language: language(args.language),
        provider: args.provider,
        use_gpu: !args.cpu,
        #[cfg(target_os = "macos")]
        warmup: Duration::from_millis(args.warmup_ms),
        #[cfg(target_os = "macos")]
        trailing: Duration::from_millis(args.trailing_ms),
        silence_seconds: args.silence_seconds,
        include_silence: !args.skip_silence,
        max_wer: args.max_wer,
    })
}

fn apply_transcription_options(
    settings: &mut engine_runtime::TranscriptionSettings,
    options: RuntimeTranscriptionOptions,
) {
    settings.backend = options.backend.into();
    settings.model_path = options.model.or_else(default_model_path);
    settings.language = options.language.and_then(language);
    settings.provider = options.provider;
    settings.use_gpu = !options.cpu;
    match options.profile {
        Profile::Realtime => {
            settings.threads = 2;
            settings.queue_capacity = 32;
        }
        Profile::Balanced => {}
        Profile::Accurate => {
            settings.threads = 6;
            settings.queue_capacity = 96;
        }
    }
    if let Some(threshold) = options.vad_threshold {
        settings.vad.threshold = threshold;
    }
    settings.vad.model_path = default_vad_model_path();
}

fn stop_flag() -> Result<Arc<AtomicBool>> {
    let stop = Arc::new(AtomicBool::new(false));
    let handler_stop = stop.clone();
    ctrlc::set_handler(move || {
        handler_stop.store(true, Ordering::Relaxed);
    })?;
    Ok(stop)
}

fn default_model_path() -> Option<PathBuf> {
    engine_runtime::TranscriptionSettings::default().model_path
}

fn default_vad_model_path() -> Option<PathBuf> {
    engine_runtime::TranscriptionSettings::default()
        .vad
        .model_path
}

fn model_root() -> PathBuf {
    std::env::var_os("RIMV_MODELS_DIR").map_or_else(
        || {
            std::env::var_os("HOME").map_or_else(
                || PathBuf::from("resources/models"),
                |home| ModelManager::default_root(&PathBuf::from(home)),
            )
        },
        PathBuf::from,
    )
}

fn language(value: String) -> Option<String> {
    if value == "auto" { None } else { Some(value) }
}

fn env_path(name: &str) -> String {
    std::env::var_os(name).map_or_else(
        || "not set".into(),
        |value| PathBuf::from(value).display().to_string(),
    )
}

fn status_line(snapshot: &EngineSnapshot) -> String {
    format!(
        "{:?}, elapsed={}ms, session={}",
        snapshot.status,
        snapshot.elapsed_ms,
        snapshot
            .session
            .as_ref()
            .map_or("-", |session| session.id.0.as_str())
    )
}

fn metrics_lines(snapshot: &EngineSnapshot) -> [String; 2] {
    let asr = &snapshot.transcription.asr;
    [
        format!(
            "capture drops: mic={} system={}",
            snapshot.dropped_microphone_blocks, snapshot.dropped_system_blocks
        ),
        format!(
            "asr: status={:?} available={} inferences={} queued={} coalesced={} dropped_work={} dropped_events={} avg_ms={} max_ms={} avg_rtf={:.3}",
            snapshot.transcription.status,
            snapshot.transcription.available,
            asr.inferences,
            asr.queued_work,
            asr.coalesced_work,
            asr.dropped_work,
            asr.dropped_events,
            asr.average_inference_ms,
            asr.maximum_inference_ms,
            asr.average_rtf_milli as f64 / 1000.0
        ),
    ]
}

impl From<Backend> for AsrBackendKind {
    fn from(value: Backend) -> Self {
        match value {
            Backend::Parakeet => Self::Parakeet,
            Backend::Whisper => Self::Whisper,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
    use engine_protocol::AudioSource;
    use speech_transcription::{SpeechEvent, SpeechSegment, SpeechToTextEngine, SpeechWorker};
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    #[derive(Debug, Clone)]
    struct AsrCall {
        start_ms: u64,
        end_ms: u64,
        text: String,
    }

    struct TracingEngine {
        inner: Box<dyn SpeechToTextEngine>,
        calls: Arc<Mutex<Vec<AsrCall>>>,
    }

    #[derive(Clone, Default)]
    struct TerminalBytes(Arc<Mutex<Vec<u8>>>);

    impl Write for TerminalBytes {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl TerminalBytes {
        fn screen(&self) -> String {
            let mut parser = vt100::Parser::new(40, 80, 0);
            parser.process(&self.0.lock().unwrap());
            parser.screen().contents()
        }
    }

    impl SpeechToTextEngine for TracingEngine {
        fn info(&self) -> speech_transcription::AsrBackendInfo {
            self.inner.info()
        }

        fn transcribe(
            &mut self,
            audio_16khz_mono: &[f32],
            offset_ms: u64,
        ) -> speech_transcription::Result<Vec<SpeechSegment>> {
            let segments = self.inner.transcribe(audio_16khz_mono, offset_ms)?;
            self.calls.lock().unwrap().push(AsrCall {
                start_ms: offset_ms,
                end_ms: offset_ms + audio_16khz_mono.len() as u64 * 1000 / 16_000,
                text: segments
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            });
            Ok(segments)
        }
    }

    /// Replays the tracked audio fixture at its original cadence through the
    /// VAD/decoder worker. It is opt-in because it needs local ASR/VAD models.
    #[test]
    #[ignore = "requires local ASR/VAD models and runs at fixture speed"]
    fn trace_recorded_fixture_through_live_worker() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/audio/audio1.mp3");
        let decoded = media::decode_mono_16k(&fixture).unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let config = SpeechConfig {
            backend: AsrBackendKind::Parakeet,
            model_path: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("resources/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"),
            ),
            ..Default::default()
        };
        let inner = load_configured_backend(config.clone()).unwrap();
        let (sender, receiver) = mpsc::sync_channel(256);
        let mut worker = SpeechWorker::start(
            TracingEngine { inner, calls: calls.clone() },
            config,
            sender,
        )
        .unwrap();
        assert!(matches!(receiver.recv().unwrap(), SpeechEvent::Ready(_)));

        let format = AudioFormat::new(16_000, 1).unwrap();
        let frames_per_block = 1_600;
        let started = Instant::now();
        for (index, block) in decoded.samples.chunks(frames_per_block).enumerate() {
            worker
                .send_captured(
                    AudioSource::System,
                    AudioFrame::new(
                        AudioSourceKind::System,
                        Duration::from_millis(index as u64 * 100),
                        format,
                        block.to_vec(),
                    )
                    .unwrap(),
                )
                .unwrap();
            let due = Duration::from_millis((index as u64 + 1) * 100);
            if let Some(wait) = due.checked_sub(started.elapsed()) {
                std::thread::sleep(wait);
            }
        }
        worker.flush_source(AudioSource::System).unwrap();
        worker.shutdown();

        let updates = receiver
            .try_iter()
            .filter_map(|event| match event {
                SpeechEvent::Update(update) => Some(update),
                _ => None,
            })
            .collect::<Vec<_>>();
        for call in calls.lock().unwrap().iter() {
            eprintln!("raw {}-{}ms: {}", call.start_ms, call.end_ms, call.text);
        }
        for update in &updates {
            eprintln!(
                "event {}-{}ms final={} stable={:?} unstable={:?}",
                update.start_ms, update.end_ms, update.is_final, update.stable_text, update.unstable_text
            );
        }
        // Replay the same event stream through an 80-column terminal. This
        // distinguishes a hypothesis revision from a terminal redraw defect.
        let terminal_bytes = TerminalBytes::default();
        let mut renderer = TranscriptRenderer::with_terminal_columns(
            terminal_bytes.clone(),
            true,
            Some(80),
        );
        let mut prior_screen = String::new();
        for update in &updates {
            renderer.update(update.clone()).unwrap();
            let current_screen = terminal_bytes.screen();
            let partial_occurrences = current_screen.matches("partial [system]").count();
            assert!(
                partial_occurrences <= 1,
                "multiple active partial regions after {}-{}ms: {current_screen:?}",
                update.start_ms,
                update.end_ms,
            );
            if !update.is_final && current_screen != prior_screen {
                eprintln!(
                    "screen {}-{}ms changed: {:?}",
                    update.start_ms, update.end_ms, current_screen
                );
            }
            prior_screen = current_screen;
        }
        assert!(calls.lock().unwrap().iter().any(|call| call.start_ms < 10_000));
        let finals = updates.iter().filter(|update| update.is_final).collect::<Vec<_>>();
        assert!(!finals.is_empty());
        assert!(finals.windows(2).all(|pair| pair[0].start_ms <= pair[1].start_ms));
    }

    #[test]
    fn event_printer_drains_finals_until_closed_and_ignores_legacy_duplicates() {
        #[derive(Clone, Default)]
        struct Output(Arc<Mutex<Vec<u8>>>);
        impl Write for Output {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let output = Output::default();
        let terminal = Arc::new(Mutex::new(TranscriptRenderer::new(output.clone(), false)));
        let first = engine_protocol::TranscriptUpdate {
            source: engine_protocol::AudioSource::Microphone,
            utterance_id: "u1".into(),
            start_ms: 0,
            end_ms: 1000,
            stable_text: "first segment".into(),
            unstable_text: String::new(),
            is_final: true,
            language: None,
            confidence: None,
        };
        let mut second = first.clone();
        second.utterance_id = "u2".into();
        second.stable_text = "second segment".into();
        second.start_ms = 4000;
        second.end_ms = 5000;
        let mut events = vec![
            EngineEvent::TranscriptFinal {
                segment: engine_protocol::TranscriptSegment {
                    source: first.source,
                    start_ms: first.start_ms,
                    end_ms: first.end_ms,
                    text: first.stable_text.clone(),
                },
            },
            EngineEvent::TranscriptUpdate {
                update: first.clone(),
            },
            EngineEvent::TranscriptUpdate { update: first },
            EngineEvent::TranscriptUpdate { update: second },
        ]
        .into_iter();
        print_events(
            |wait| {
                assert!(wait <= REFRESH_INTERVAL);
                events
                    .next()
                    .ok_or(engine_runtime::SubscriptionError::Closed)
            },
            terminal,
        )
        .unwrap();
        let bytes = output.0.lock().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(text.matches("first segment").count(), 1);
        assert_eq!(text.matches("second segment").count(), 1);
        assert!(!text.contains('\x1b'));
    }

    #[test]
    fn language_auto_means_backend_default() {
        assert_eq!(language("auto".into()), None);
        assert_eq!(language("es".into()), Some("es".into()));
    }

    #[test]
    fn backend_mapping_is_generic() {
        assert_eq!(
            AsrBackendKind::from(Backend::Parakeet),
            AsrBackendKind::Parakeet
        );
        assert_eq!(
            AsrBackendKind::from(Backend::Whisper),
            AsrBackendKind::Whisper
        );
    }

    #[test]
    fn wav_reader_normalizes_to_asr_rate() {
        let path = std::env::temp_dir().join(format!("rimv-cli-test-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        {
            let mut writer = hound::WavWriter::create(&path, spec).unwrap();
            for _ in 0..4096 {
                writer.write_sample::<i16>(1000).unwrap();
                writer.write_sample::<i16>(-1000).unwrap();
            }
            writer.finalize().unwrap();
        }
        let samples = media::decode_mono_16k(&path).unwrap().samples;
        let _ = std::fs::remove_file(&path);
        assert!(!samples.is_empty());
        assert!(samples.len() < 4096);
    }
}
