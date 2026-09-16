use clap::{Args, Parser, Subcommand, ValueEnum};
use engine_protocol::{EngineEvent, EngineSnapshot};
use engine_runtime::{AsrBackendKind, EngineConfig, EngineRuntime};
use model_manager::ModelManager;
use speech_transcription::{SpeechConfig, load_configured_backend};
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

mod benchmark;
mod media;
mod transcript_renderer;

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
    #[arg(long, value_enum, default_value_t = Backend::Parakeet)]
    backend: Backend,
    #[arg(long)]
    model: Option<PathBuf>,
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(io::stderr)
        .init();

    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rimv: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Listen(args) => listen(args),
        Command::Transcribe(args) => transcribe(args),
        Command::Doctor(args) => doctor(args),
        Command::Models(args) => models(args),
        Command::Devices => devices(),
        Command::Serve(args) => serve(args),
        Command::Benchmark(args) => benchmark(args),
    }
}

fn listen(args: ListenArgs) -> Result<()> {
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
    let printer_stop = stop.clone();
    let show_partials = args.show_partials;
    let printer = thread::Builder::new()
        .name("rimv-listen-events".into())
        .spawn(move || print_events(events, printer_stop, show_partials))?;

    engine.start_capture()?;
    println!("listening; press Ctrl-C to stop");
    let started = Instant::now();
    while !stop.load(Ordering::Relaxed)
        && (args.seconds == 0 || started.elapsed() < Duration::from_secs(args.seconds))
    {
        thread::sleep(Duration::from_millis(100));
    }
    let stopped = engine.stop_capture();
    let snapshot = engine.snapshot();
    engine.shutdown()?;
    stop.store(true, Ordering::Relaxed);
    printer
        .join()
        .map_err(|_| "listen event printer panicked")?
        .ok();
    stopped?;
    println!("stopped: {}", status_line(&snapshot));
    if args.show_metrics {
        print_metrics(&snapshot);
    }
    Ok(())
}

fn print_events(
    events: engine_runtime::Subscription,
    stop: Arc<AtomicBool>,
    show_partials: bool,
) -> std::result::Result<(), engine_runtime::SubscriptionError> {
    let stdout = io::stdout();
    let interactive_partials = show_partials && stdout.is_terminal();
    let mut renderer = TranscriptRenderer::new(stdout, interactive_partials);
    while !stop.load(Ordering::Relaxed) {
        match events.recv_timeout(Duration::from_millis(100)) {
            Ok(EngineEvent::TranscriptUpdate { update }) => {
                let _ = renderer.update(update);
            }
            Ok(EngineEvent::TranscriptionError { error }) | Ok(EngineEvent::Error { error }) => {
                let _ = renderer.suspend();
                eprintln!("error: {error}");
                let _ = renderer.resume();
            }
            Ok(_) => {}
            Err(engine_runtime::SubscriptionError::Timeout) => {}
            Err(error) => {
                let _ = renderer.finish();
                return Err(error);
            }
        }
    }
    let _ = renderer.finish();
    Ok(())
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
    let root = model_root();
    let manager = ModelManager::new(&root);
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
    std::env::var_os("RIMV_PARAKEET_MODEL_DIR").map_or_else(
        || Some(model_root().join("sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8")),
        |path| Some(PathBuf::from(path)),
    )
}

fn default_vad_model_path() -> Option<PathBuf> {
    std::env::var_os("RIMV_SILERO_VAD_MODEL").map_or_else(
        || Some(model_root().join("silero_vad.onnx")),
        |path| Some(PathBuf::from(path)),
    )
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

fn print_metrics(snapshot: &EngineSnapshot) {
    let asr = &snapshot.transcription.asr;
    println!(
        "capture drops: mic={} system={}",
        snapshot.dropped_microphone_blocks, snapshot.dropped_system_blocks
    );
    println!(
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
    );
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
