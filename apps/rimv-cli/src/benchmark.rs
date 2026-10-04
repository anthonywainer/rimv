use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use clap::ValueEnum;
#[cfg(target_os = "macos")]
use engine_protocol::EngineEvent;
use engine_protocol::{AudioSource, EngineSnapshot, TranscriptUpdate};
use engine_runtime::AsrBackendKind;
#[cfg(target_os = "macos")]
use engine_runtime::{EngineConfig, EngineRuntime};
use serde::{Deserialize, Serialize};
use speech_transcription::{
    AsrBackendInfo, SpeechConfig, SpeechEvent, SpeechToTextEngine, SpeechWorker,
    load_configured_backend,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "macos")]
use std::{
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{Result, media};

const SUITE: &str = concat!("rimv-", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum BenchmarkMode {
    DirectFile,
    RealtimeCapture,
    RealtimeReplay,
    All,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum BenchmarkBackend {
    #[default]
    Parakeet,
    Whisper,
    Enhanced,
    NativeApple,
    NativeWindows,
}

impl From<BenchmarkBackend> for AsrBackendKind {
    fn from(value: BenchmarkBackend) -> Self {
        match value {
            BenchmarkBackend::Parakeet => Self::Parakeet,
            BenchmarkBackend::Whisper => Self::Whisper,
            BenchmarkBackend::Enhanced => Self::Enhanced,
            BenchmarkBackend::NativeApple => Self::AppleNative,
            BenchmarkBackend::NativeWindows => Self::WindowsNative,
        }
    }
}

pub(crate) struct BenchmarkOptions {
    pub corpus: PathBuf,
    pub mode: BenchmarkMode,
    pub output: Option<PathBuf>,
    pub model: Option<PathBuf>,
    pub refinement_model: Option<PathBuf>,
    pub backend: AsrBackendKind,
    pub language: Option<String>,
    pub provider: String,
    pub use_gpu: bool,
    #[cfg(target_os = "macos")]
    pub warmup: Duration,
    #[cfg(target_os = "macos")]
    pub trailing: Duration,
    pub silence_seconds: u64,
    pub include_silence: bool,
    pub max_wer: Option<f64>,
}

#[derive(Debug, Clone)]
struct CorpusCase {
    name: String,
    audio: PathBuf,
    reference: Option<PathBuf>,
    metadata: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct CaseMetadata {
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Accuracy {
    wer: f64,
    cer: f64,
    substitutions: usize,
    insertions: usize,
    deletions: usize,
    reference_words: usize,
    hypothesis_words: usize,
    character_substitutions: usize,
    character_insertions: usize,
    character_deletions: usize,
    reference_characters: usize,
    hypothesis_characters: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CaseMetrics {
    audio_duration_ms: Option<u64>,
    benchmark_wall_ms: u64,
    backend: String,
    model: String,
    language: String,
    model_load_ms: Option<u64>,
    playback_start_ms: Option<u64>,
    first_speech_detection_ms: Option<u64>,
    first_partial_ms: Option<u64>,
    first_stable_text_ms: Option<u64>,
    first_final_ms: Option<u64>,
    partial_count: u64,
    partial_stability: Option<f64>,
    average_partial_latency_ms: Option<u64>,
    finalization_latency_ms: Option<u64>,
    partial_latency_p50_ms: Option<u64>,
    partial_latency_p95_ms: Option<u64>,
    vad_segment_count: u64,
    asr_inference_count: u64,
    average_inference_ms: Option<u64>,
    maximum_inference_ms: Option<u64>,
    rtf: Option<f64>,
    cpu_usage_percent: Option<f64>,
    average_cpu_percent: Option<f64>,
    peak_cpu_percent: Option<f64>,
    average_rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
    microphone_capture_drops: u64,
    system_capture_drops: u64,
    speech_pipeline_drops: u64,
    asr_coalesced_work: u64,
    asr_dropped_work: u64,
    asr_dropped_events: u64,
    deadline_misses: u64,
    parakeet_final: Option<String>,
    whisper_refined_final: Option<String>,
    refinement_latency_ms: Option<u64>,
    refinement_changed: Option<bool>,
    refinement_improved: Option<bool>,
    refinement_worsened: Option<bool>,
    hallucinated_words_during_non_speech: Option<usize>,
    accuracy: Option<Accuracy>,
}

#[derive(Debug, Clone, Serialize)]
struct CaseReport {
    case: String,
    mode: String,
    audio_path: String,
    reference_path: Option<String>,
    status: String,
    failure: Option<String>,
    hypothesis: String,
    metrics: CaseMetrics,
}

#[derive(Debug, Serialize)]
struct Summary<'a> {
    schema_version: u32,
    suite: &'static str,
    benchmark_version: &'static str,
    validation_status: &'static str,
    generated_at_unix_ms: u64,
    git_commit: Option<String>,
    environment: system_profile::SystemProfile,
    normalization: &'static str,
    corpus: String,
    output: String,
    total_cases: usize,
    successful_cases: usize,
    failed_cases: usize,
    aggregate_wer: Option<f64>,
    aggregate_cer: Option<f64>,
    results: &'a [CaseReport],
}

struct DirectRunner {
    backend: Box<dyn SpeechToTextEngine>,
    info: AsrBackendInfo,
    model_load_ms: u64,
}

#[derive(Default)]
struct Observations {
    finals: Vec<(String, String)>,
    first_partial_ms: Option<u64>,
    first_stable_ms: Option<u64>,
    first_final_ms: Option<u64>,
    first_final_audio_end_ms: Option<u64>,
    partial_count: u64,
    partial_arrivals: Vec<(u64, u64)>,
    partial_stabilities: Vec<f64>,
    previous_partials: HashMap<String, String>,
    finalized_audio_ms: Option<u64>,
    initial_finals: Vec<(String, String)>,
    finalized_at: HashMap<String, u64>,
    refinement_latencies: Vec<u64>,
    refined_final_ids: HashSet<String>,
    actual_backend: Option<String>,
    actual_model: Option<String>,
    errors: Vec<String>,
}

pub(crate) fn run(options: BenchmarkOptions) -> Result<()> {
    validate_options(&options)?;
    let cases = discover_corpus(&options.corpus)?;
    if cases.is_empty() {
        return Err(format!(
            "no supported audio files found under {}",
            options.corpus.display()
        )
        .into());
    }
    let timestamp = unix_ms()?;
    let output = options.output.clone().unwrap_or_else(|| {
        PathBuf::from("target/benchmarks")
            .join(SUITE)
            .join(timestamp.to_string())
    });
    fs::create_dir_all(&output)?;
    println!("benchmark output: {}", output.display());

    let mut results = Vec::new();
    if matches!(options.mode, BenchmarkMode::DirectFile | BenchmarkMode::All)
        && options.backend != AsrBackendKind::Enhanced
    {
        let mut runners = HashMap::<Option<String>, DirectRunner>::new();
        let mut loader_failures = HashMap::<Option<String>, String>::new();
        for case in &cases {
            let selected_language = effective_language(case, &options);
            ensure_direct_runner(
                &options,
                &selected_language,
                &mut runners,
                &mut loader_failures,
            );
            let report = if let Some(runner) = runners.get_mut(&selected_language) {
                run_direct_case(case, &options, selected_language.clone(), runner, &output)
            } else {
                failed_report(
                    case,
                    "direct_file",
                    &options,
                    format!(
                        "model initialization failed: {}",
                        loader_failures
                            .get(&selected_language)
                            .map_or("unknown error", String::as_str)
                    ),
                )
            };
            print_case(&report);
            results.push(report);
        }
        if options.include_silence {
            let selected_language = options.language.clone();
            ensure_direct_runner(
                &options,
                &selected_language,
                &mut runners,
                &mut loader_failures,
            );
            let report = if let Some(runner) = runners.get_mut(&selected_language) {
                run_direct_silence(&options, runner, &output)
            } else {
                failed_report(
                    &generated_silence_case(options.silence_seconds),
                    "direct_file",
                    &options,
                    format!(
                        "model initialization failed: {}",
                        loader_failures
                            .get(&selected_language)
                            .map_or("unknown error", String::as_str)
                    ),
                )
            };
            print_case(&report);
            results.push(report);
        }
    }
    if matches!(
        options.mode,
        BenchmarkMode::RealtimeReplay | BenchmarkMode::All
    ) {
        for case in &cases {
            let report = run_realtime_replay_case(case, &options, &output);
            print_case(&report);
            results.push(report);
        }
    }
    if matches!(options.mode, BenchmarkMode::RealtimeCapture)
        || (cfg!(target_os = "macos") && matches!(options.mode, BenchmarkMode::All))
    {
        for case in &cases {
            let report = run_realtime_case(case, &options, &output);
            print_case(&report);
            results.push(report);
        }
        if options.include_silence {
            let report = run_realtime_silence(&options, &output);
            print_case(&report);
            results.push(report);
        }
    }
    apply_wer_gate(&mut results, options.max_wer);
    write_summary(&options, &output, timestamp, &results)?;
    let failures = results
        .iter()
        .filter(|result| result.status == "failed")
        .count();
    println!(
        "benchmark complete: {} succeeded, {} failed; {}",
        results.len() - failures,
        failures,
        output.join("benchmark-results.md").display()
    );
    if failures > 0 {
        return Err(
            format!("{failures} benchmark case(s) failed; see the generated report").into(),
        );
    }
    Ok(())
}

fn validate_options(options: &BenchmarkOptions) -> Result<()> {
    if !options.corpus.is_dir() {
        return Err(format!(
            "corpus directory does not exist: {}",
            options.corpus.display()
        )
        .into());
    }
    if options.silence_seconds == 0 {
        return Err("--silence-seconds must be greater than zero".into());
    }
    if options.backend == AsrBackendKind::Enhanced {
        if options.model.is_none() || options.refinement_model.is_none() {
            return Err(
                "Enhanced requires both a Parakeet --model and a Whisper --refinement-model".into(),
            );
        }
        if matches!(options.mode, BenchmarkMode::DirectFile) {
            return Err("Enhanced requires finalized-segment refinement; use realtime-replay or realtime-capture".into());
        }
    }
    if options.backend == AsrBackendKind::AppleNative && !cfg!(target_os = "macos") {
        return Err("Native Apple benchmarking requires macOS; no fallback engine is used".into());
    }
    if options.backend == AsrBackendKind::WindowsNative && !cfg!(target_os = "windows") {
        return Err(
            "Native Windows benchmarking requires Windows; no fallback engine is used".into(),
        );
    }
    if options
        .max_wer
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("--max-wer must be a finite value greater than or equal to zero".into());
    }
    #[cfg(not(target_os = "macos"))]
    if matches!(options.mode, BenchmarkMode::RealtimeCapture) {
        return Err("realtime-capture benchmarks currently require macOS ScreenCaptureKit".into());
    }
    Ok(())
}

fn apply_wer_gate(results: &mut [CaseReport], max_wer: Option<f64>) {
    let Some(limit) = max_wer else {
        return;
    };
    for report in results {
        let Some(accuracy) = report.metrics.accuracy.as_ref() else {
            continue;
        };
        if accuracy.wer > limit {
            report.status = "failed".into();
            report.failure = Some(format!(
                "WER {:.3} exceeds required maximum {:.3}",
                accuracy.wer, limit
            ));
        }
    }
}

impl DirectRunner {
    fn load(options: &BenchmarkOptions, language: Option<String>) -> Result<Self> {
        let started = Instant::now();
        let backend = load_configured_backend(speech_config(options, language))
            .map_err(|error| benchmark_engine_error(options.backend, error))?;
        let model_load_ms = elapsed_ms(started);
        let info = backend.info();
        validate_engine_identity(options.backend, &info)?;
        Ok(Self {
            backend,
            info,
            model_load_ms,
        })
    }
}

fn ensure_direct_runner(
    options: &BenchmarkOptions,
    language: &Option<String>,
    runners: &mut HashMap<Option<String>, DirectRunner>,
    failures: &mut HashMap<Option<String>, String>,
) {
    if runners.contains_key(language) || failures.contains_key(language) {
        return;
    }
    match DirectRunner::load(options, language.clone()) {
        Ok(runner) => {
            runners.insert(language.clone(), runner);
        }
        Err(error) => {
            failures.insert(language.clone(), error.to_string());
        }
    }
}

fn run_direct_case(
    case: &CorpusCase,
    options: &BenchmarkOptions,
    selected_language: Option<String>,
    runner: &mut DirectRunner,
    output: &Path,
) -> CaseReport {
    let started = Instant::now();
    let language = display_language(&selected_language);
    let result = (|| -> Result<(String, u64, u64, crate::resource::ResourceMetrics)> {
        let decoded = media::decode_mono_16k(&case.audio)?;
        let inference_started = Instant::now();
        let resources = crate::resource::ResourceMonitor::start();
        let segments = runner.backend.transcribe(&decoded.samples, 0)?;
        let resources = resources.finish();
        Ok((
            segments
                .into_iter()
                .map(|segment| segment.text)
                .collect::<Vec<_>>()
                .join(" "),
            decoded.duration_ms,
            elapsed_ms(inference_started),
            resources,
        ))
    })();
    match result {
        Ok((hypothesis, duration_ms, inference_ms, resources)) => finish_scored_case(
            case,
            "direct_file",
            output,
            hypothesis,
            DirectMetrics {
                wall_ms: elapsed_ms(started),
                audio_duration_ms: duration_ms,
                backend: runner.info.backend_name.clone(),
                model: runner.info.model_name.clone(),
                language,
                model_load_ms: Some(runner.model_load_ms),
                inference_ms,
                resources,
            },
            None,
            false,
        ),
        Err(error) => failed_report(case, "direct_file", options, error.to_string()),
    }
}

fn run_direct_silence(
    options: &BenchmarkOptions,
    runner: &mut DirectRunner,
    output: &Path,
) -> CaseReport {
    let started = Instant::now();
    let samples = vec![0.0; options.silence_seconds as usize * 16_000];
    let case = generated_silence_case(options.silence_seconds);
    let inference_started = Instant::now();
    let resources = crate::resource::ResourceMonitor::start();
    match runner.backend.transcribe(&samples, 0) {
        Ok(segments) => finish_scored_case(
            &case,
            "direct_file",
            output,
            segments
                .into_iter()
                .map(|segment| segment.text)
                .collect::<Vec<_>>()
                .join(" "),
            DirectMetrics {
                wall_ms: elapsed_ms(started),
                audio_duration_ms: options.silence_seconds * 1000,
                backend: runner.info.backend_name.clone(),
                model: runner.info.model_name.clone(),
                language: display_language(&options.language),
                model_load_ms: Some(runner.model_load_ms),
                inference_ms: elapsed_ms(inference_started),
                resources: resources.finish(),
            },
            Some(String::new()),
            true,
        ),
        Err(error) => failed_report(&case, "direct_file", options, error.to_string()),
    }
}

struct DirectMetrics {
    wall_ms: u64,
    audio_duration_ms: u64,
    backend: String,
    model: String,
    language: String,
    model_load_ms: Option<u64>,
    inference_ms: u64,
    resources: crate::resource::ResourceMetrics,
}

fn finish_scored_case(
    case: &CorpusCase,
    mode: &str,
    output: &Path,
    hypothesis: String,
    direct: DirectMetrics,
    generated_reference: Option<String>,
    silence: bool,
) -> CaseReport {
    let reference_result = generated_reference.map_or_else(
        || {
            case.reference
                .as_ref()
                .ok_or_else(|| "reference transcript is missing".to_string())
                .and_then(|path| fs::read_to_string(path).map_err(|error| error.to_string()))
        },
        Ok,
    );
    match reference_result {
        Ok(reference) => {
            let accuracy = score(&reference, &hypothesis);
            let normalized_reference = normalize(&reference);
            let normalized_hypothesis = normalize(&hypothesis);
            let metrics = CaseMetrics {
                audio_duration_ms: Some(direct.audio_duration_ms),
                benchmark_wall_ms: direct.wall_ms,
                backend: direct.backend,
                model: direct.model,
                language: direct.language,
                model_load_ms: direct.model_load_ms,
                playback_start_ms: None,
                first_speech_detection_ms: None,
                first_partial_ms: None,
                first_stable_text_ms: None,
                first_final_ms: None,
                partial_count: 0,
                partial_stability: None,
                average_partial_latency_ms: None,
                finalization_latency_ms: None,
                partial_latency_p50_ms: None,
                partial_latency_p95_ms: None,
                vad_segment_count: 0,
                asr_inference_count: 1,
                average_inference_ms: Some(direct.inference_ms),
                maximum_inference_ms: Some(direct.inference_ms),
                rtf: Some(ratio(direct.inference_ms, direct.audio_duration_ms)),
                cpu_usage_percent: direct.resources.average_cpu_percent,
                average_cpu_percent: direct.resources.average_cpu_percent,
                peak_cpu_percent: direct.resources.peak_cpu_percent,
                average_rss_bytes: direct.resources.average_ram_bytes,
                peak_rss_bytes: direct.resources.peak_ram_bytes,
                microphone_capture_drops: 0,
                system_capture_drops: 0,
                speech_pipeline_drops: 0,
                asr_coalesced_work: 0,
                asr_dropped_work: 0,
                asr_dropped_events: 0,
                deadline_misses: 0,
                parakeet_final: None,
                whisper_refined_final: None,
                refinement_latency_ms: None,
                refinement_changed: None,
                refinement_improved: None,
                refinement_worsened: None,
                hallucinated_words_during_non_speech: silence.then_some(accuracy.hypothesis_words),
                accuracy: Some(accuracy),
            };
            let report = CaseReport {
                case: case.name.clone(),
                mode: mode.into(),
                audio_path: case.audio.display().to_string(),
                reference_path: case
                    .reference
                    .as_ref()
                    .map(|path| path.display().to_string()),
                status: "ok".into(),
                failure: None,
                hypothesis,
                metrics,
            };
            if let Err(error) = write_case_artifacts(
                output,
                &report,
                &reference,
                &normalized_reference,
                &normalized_hypothesis,
                None,
            ) {
                return failed_report_with_metrics(case, mode, report.metrics, error.to_string());
            }
            report
        }
        Err(error) => failed_report_with_metrics(
            case,
            mode,
            empty_metrics(options_backend_placeholder(), direct.wall_ms),
            error,
        ),
    }
}

fn run_realtime_case(case: &CorpusCase, options: &BenchmarkOptions, output: &Path) -> CaseReport {
    run_realtime(case, options, output, None)
}

fn run_realtime_replay_case(
    case: &CorpusCase,
    options: &BenchmarkOptions,
    output: &Path,
) -> CaseReport {
    let started = Instant::now();
    let decoded = match media::decode_mono_16k(&case.audio) {
        Ok(audio) => audio,
        Err(error) => {
            return failed_report(
                case,
                "realtime_replay",
                options,
                benchmark_engine_error(options.backend, error),
            );
        }
    };
    let selected_language = effective_language(case, options);
    let load_started = Instant::now();
    let engine = match load_configured_backend(speech_config(options, selected_language.clone())) {
        Ok(engine) => engine,
        Err(error) => return failed_report(case, "realtime_replay", options, error.to_string()),
    };
    let model_load_ms = elapsed_ms(load_started);
    let (event_sender, event_receiver) = mpsc::sync_channel(256);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let observations = Arc::new(Mutex::new(Observations::default()));
    let collector_observations = observations.clone();
    let collector = thread::spawn(move || {
        while let Ok(event) = event_receiver.recv() {
            match event {
                SpeechEvent::Ready(info) => {
                    {
                        let mut state = collector_observations
                            .lock()
                            .unwrap_or_else(|p| p.into_inner());
                        state.actual_backend = Some(info.backend_id.clone());
                        state.actual_model = Some(info.model_name.clone());
                    }
                    let _ = ready_sender.send(info);
                }
                SpeechEvent::Update(update) => {
                    let arrival = elapsed_ms(started);
                    observe_update(
                        update,
                        arrival,
                        &mut collector_observations
                            .lock()
                            .unwrap_or_else(|p| p.into_inner()),
                    );
                }
                SpeechEvent::Error(error) => collector_observations
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .errors
                    .push(error.to_string()),
                SpeechEvent::Partial(segment) => {
                    let _ = segment;
                }
                SpeechEvent::Final(segment) => {
                    let _ = segment;
                }
            }
        }
    });
    let config = speech_config(options, selected_language.clone());
    let mut worker = match SpeechWorker::start(engine, config, event_sender) {
        Ok(worker) => worker,
        Err(error) => {
            let _ = collector.join();
            return failed_report(case, "realtime_replay", options, error.to_string());
        }
    };
    let info = match ready_receiver.recv_timeout(Duration::from_secs(120)) {
        Ok(info) => info,
        Err(error) => {
            worker.shutdown();
            let _ = collector.join();
            return failed_report(
                case,
                "realtime_replay",
                options,
                format!("ASR backend did not become ready: {error}"),
            );
        }
    };
    let playback_start_ms = elapsed_ms(started);
    let resources = crate::resource::ResourceMonitor::start();
    let format = match AudioFormat::new(16_000, 1) {
        Ok(format) => format,
        Err(error) => {
            worker.shutdown();
            let _ = collector.join();
            return failed_report(case, "realtime_replay", options, error.to_string());
        }
    };
    let chunk_samples = 1_600usize;
    let playback_started = Instant::now();
    let mut playback_error = None;
    for (index, samples) in decoded.samples.chunks(chunk_samples).enumerate() {
        let offset = index * chunk_samples;
        let audio_time = Duration::from_secs_f64(offset as f64 / 16_000.0);
        let frame = match AudioFrame::new(
            AudioSourceKind::Microphone,
            audio_time,
            format,
            samples.to_vec(),
        ) {
            Ok(frame) => frame,
            Err(error) => {
                playback_error = Some(error.to_string());
                break;
            }
        };
        if let Err(error) = worker.send_captured(AudioSource::Microphone, frame) {
            playback_error = Some(error.to_string());
            break;
        }
        let target = Duration::from_secs_f64((offset + samples.len()) as f64 / 16_000.0);
        if let Some(wait) = target.checked_sub(playback_started.elapsed()) {
            thread::sleep(wait);
        }
    }
    let _ = worker.flush_source(AudioSource::Microphone);
    worker.shutdown(); // Flushes remaining VAD audio and drains bounded refinement work.
    let speech_metrics = worker.metrics();
    let resources = resources.finish();
    drop(worker);
    let _ = collector.join();
    let observed = observations.lock().unwrap_or_else(|p| p.into_inner());
    let hypothesis = observed
        .finals
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let identity_failure = validate_engine_identity(options.backend, &info)
        .err()
        .map(|error| error.to_string());
    let failure = playback_error
        .or(identity_failure)
        .or_else(|| observed.errors.first().cloned())
        .or_else(|| {
            observed
                .finals
                .is_empty()
                .then(|| "backend produced no finalized transcript".into())
        });
    let mut snapshot = EngineSnapshot::default();
    snapshot.transcription.backend = Some(info.backend_name.clone());
    snapshot.transcription.model = Some(info.model_name.clone());
    snapshot.transcription.dropped_blocks = speech_metrics.dropped_blocks;
    snapshot.transcription.asr.inferences = speech_metrics.inferences;
    snapshot.transcription.asr.queued_work = speech_metrics.queued_work;
    snapshot.transcription.asr.coalesced_work = speech_metrics.coalesced_work;
    snapshot.transcription.asr.dropped_work = speech_metrics.dropped_work;
    snapshot.transcription.asr.dropped_events = speech_metrics.dropped_events;
    snapshot.transcription.asr.vad_segments = speech_metrics.vad_segments;
    snapshot.transcription.asr.model_load_ms = model_load_ms;
    snapshot.transcription.asr.deadline_misses = speech_metrics.deadline_misses;
    snapshot.transcription.asr.average_inference_ms = speech_metrics.inference_average_ms as u64;
    snapshot.transcription.asr.maximum_inference_ms = speech_metrics.inference_max_ms;
    snapshot.transcription.asr.average_rtf_milli =
        (speech_metrics.rtf_total * 1000.0 / speech_metrics.inferences.max(1) as f64) as u64;
    let mut metrics = realtime_metrics(
        options,
        elapsed_ms(started),
        Some(decoded.duration_ms),
        playback_start_ms,
        &snapshot,
        &observed,
        &resources,
    );
    metrics.microphone_capture_drops = speech_metrics.dropped_blocks;
    metrics.speech_pipeline_drops = speech_metrics.dropped_blocks;
    metrics.model_load_ms = Some(model_load_ms);
    let report = match failure {
        Some(error) => return failed_report_with_metrics(case, "realtime_replay", metrics, error),
        None => {
            let reference = match case.reference.as_ref().map(fs::read_to_string).transpose() {
                Ok(Some(reference)) => reference,
                Ok(None) => {
                    return failed_report_with_metrics(
                        case,
                        "realtime_replay",
                        metrics,
                        "reference transcript is missing".into(),
                    );
                }
                Err(error) => {
                    return failed_report_with_metrics(
                        case,
                        "realtime_replay",
                        metrics,
                        error.to_string(),
                    );
                }
            };
            let accuracy = score(&reference, &hypothesis);
            apply_refinement_metrics(&mut metrics, &observed, &reference);
            if reference.is_empty() {
                metrics.hallucinated_words_during_non_speech = Some(accuracy.hypothesis_words);
            }
            metrics.accuracy = Some(accuracy);
            CaseReport {
                case: case.name.clone(),
                mode: "realtime_replay".into(),
                audio_path: case.audio.display().to_string(),
                reference_path: case
                    .reference
                    .as_ref()
                    .map(|path| path.display().to_string()),
                status: "ok".into(),
                failure: None,
                hypothesis,
                metrics,
            }
        }
    };
    let reference = case
        .reference
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default();
    if let Err(error) = write_case_artifacts(
        output,
        &report,
        &reference,
        &normalize(&reference),
        &normalize(&report.hypothesis),
        None,
    ) {
        return failed_report_with_metrics(
            case,
            "realtime_replay",
            report.metrics,
            error.to_string(),
        );
    }
    report
}

fn expected_backend_id(backend: AsrBackendKind) -> &'static str {
    match backend {
        AsrBackendKind::Parakeet => "sherpa-onnx-offline-transducer",
        AsrBackendKind::Whisper => "whisper.cpp",
        AsrBackendKind::Enhanced => "enhanced-parakeet-whisper",
        AsrBackendKind::AppleNative => "native_apple",
        AsrBackendKind::WindowsNative => "native_windows",
    }
}

#[cfg(target_os = "macos")]
fn expected_backend_name(backend: AsrBackendKind) -> &'static str {
    match backend {
        AsrBackendKind::Parakeet => "sherpa-onnx Parakeet",
        AsrBackendKind::Whisper => "Whisper",
        AsrBackendKind::Enhanced => "Enhanced — Parakeet + Whisper",
        AsrBackendKind::AppleNative => "Apple Speech",
        AsrBackendKind::WindowsNative => "Windows Native Speech",
    }
}

fn validate_engine_identity(backend: AsrBackendKind, info: &AsrBackendInfo) -> Result<()> {
    let expected = expected_backend_id(backend);
    if info.backend_id == expected {
        Ok(())
    } else {
        Err(format!(
            "requested {expected}, loaded {} (no fallback allowed)",
            info.backend_id
        )
        .into())
    }
}

fn benchmark_engine_error(backend: AsrBackendKind, error: impl std::fmt::Display) -> String {
    let message = error.to_string();
    if backend == AsrBackendKind::AppleNative
        && message.contains("Speech Recognition permission is denied")
    {
        format!(
            "{message} macOS grants Speech Recognition per app identity. This benchmark was launched from a shell, so enable the launching terminal app (for example Terminal or your IDE) in System Settings > Privacy & Security > Speech Recognition; RimV's app permission does not cover it."
        )
    } else {
        message
    }
}

fn run_realtime_silence(options: &BenchmarkOptions, output: &Path) -> CaseReport {
    let case_dir = output.join("generated-inputs");
    if let Err(error) = fs::create_dir_all(&case_dir) {
        return failed_report(
            &generated_silence_case(options.silence_seconds),
            "realtime_capture",
            options,
            error.to_string(),
        );
    }
    let path = case_dir.join(format!("silence-{}s.wav", options.silence_seconds));
    if let Err(error) = write_silence_wav(&path, options.silence_seconds) {
        return failed_report(
            &generated_silence_case(options.silence_seconds),
            "realtime_capture",
            options,
            error.to_string(),
        );
    }
    let mut case = generated_silence_case(options.silence_seconds);
    case.audio = path;
    run_realtime(&case, options, output, Some(String::new()))
}

fn run_realtime(
    case: &CorpusCase,
    options: &BenchmarkOptions,
    output: &Path,
    generated_reference: Option<String>,
) -> CaseReport {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (case, options, output, generated_reference);
        unreachable!("validated before execution")
    }
    #[cfg(target_os = "macos")]
    {
        let started = Instant::now();
        let decoded_duration = media::decode_mono_16k(&case.audio)
            .map(|audio| audio.duration_ms)
            .ok();
        let recordings = output.join("recordings").join(&case.name);
        let mut config = EngineConfig {
            recordings_directory: recordings,
            transcription_enabled: true,
            ..Default::default()
        };
        config.microphone.enabled = false;
        config.system_audio.enabled = true;
        config.transcription = runtime_transcription(options, case_language(case));
        let engine = match EngineRuntime::new(config) {
            Ok(engine) => engine,
            Err(error) => {
                return failed_report(case, "realtime_capture", options, error.to_string());
            }
        };
        let events = match engine.subscribe() {
            Ok(events) => events,
            Err(error) => {
                return failed_report(case, "realtime_capture", options, error.to_string());
            }
        };
        let observations = Arc::new(Mutex::new(Observations::default()));
        let collector_stop = Arc::new(AtomicBool::new(false));
        let collector = spawn_collector(
            events,
            started,
            observations.clone(),
            collector_stop.clone(),
        );
        if let Err(error) = engine.start_capture() {
            collector_stop.store(true, Ordering::Release);
            let _ = collector.join();
            let _ = engine.shutdown();
            return failed_report(case, "realtime_capture", options, error.to_string());
        }
        let model_wait_started = Instant::now();
        while model_wait_started.elapsed() < Duration::from_secs(120) {
            let snapshot = engine.snapshot();
            if matches!(
                snapshot.transcription.status,
                engine_protocol::TranscriptionStatus::Ready
                    | engine_protocol::TranscriptionStatus::Transcribing
            ) {
                break;
            }
            if snapshot.transcription.status == engine_protocol::TranscriptionStatus::Error {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        thread::sleep(options.warmup);
        let playback_start_ms = elapsed_ms(started);
        let resources = crate::resource::ResourceMonitor::start();
        let playback = Command::new("/usr/bin/afplay").arg(&case.audio).status();
        let playback_failure = match playback {
            Ok(status) if status.success() => None,
            Ok(status) => Some(format!("afplay exited with {status}")),
            Err(error) => Some(format!("could not start afplay: {error}")),
        };
        thread::sleep(options.trailing);
        let stopped = engine.stop_capture();
        let resources = resources.finish();
        let snapshot = engine.snapshot();
        let session_json = snapshot
            .session
            .as_ref()
            .map(|session| PathBuf::from(&session.recording_directory).join("session.json"));
        let _ = engine.shutdown();
        collector_stop.store(true, Ordering::Release);
        let _ = collector.join();
        let observed = observations.lock().unwrap_or_else(|lock| lock.into_inner());
        let hypothesis = observed
            .finals
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let failure = playback_failure
            .or_else(|| stopped.err().map(|error| error.to_string()))
            .or_else(|| {
                let expected = expected_backend_name(options.backend);
                let actual = snapshot
                    .transcription
                    .backend
                    .as_deref()
                    .unwrap_or("unreported");
                (actual != expected)
                    .then(|| format!("requested {expected}, loaded {actual} (no fallback allowed)"))
            })
            .or_else(|| observed.errors.first().cloned());
        if let Some(error) = failure {
            return failed_report_with_metrics(
                case,
                "realtime_capture",
                realtime_metrics(
                    options,
                    elapsed_ms(started),
                    decoded_duration,
                    playback_start_ms,
                    &snapshot,
                    &observed,
                    &resources,
                ),
                error,
            );
        }

        // The reference is deliberately opened only after capture, recognition,
        // finalization, and playback have all completed.
        let reference = match generated_reference {
            Some(reference) => reference,
            None => match case.reference.as_ref() {
                Some(path) => match fs::read_to_string(path) {
                    Ok(reference) => reference,
                    Err(error) => {
                        return failed_report_with_metrics(
                            case,
                            "realtime_capture",
                            realtime_metrics(
                                options,
                                elapsed_ms(started),
                                decoded_duration,
                                playback_start_ms,
                                &snapshot,
                                &observed,
                                &resources,
                            ),
                            error.to_string(),
                        );
                    }
                },
                None => {
                    return failed_report_with_metrics(
                        case,
                        "realtime_capture",
                        realtime_metrics(
                            options,
                            elapsed_ms(started),
                            decoded_duration,
                            playback_start_ms,
                            &snapshot,
                            &observed,
                            &resources,
                        ),
                        "reference transcript is missing".into(),
                    );
                }
            },
        };
        let accuracy = score(&reference, &hypothesis);
        let mut metrics = realtime_metrics(
            options,
            elapsed_ms(started),
            decoded_duration,
            playback_start_ms,
            &snapshot,
            &observed,
            &resources,
        );
        apply_refinement_metrics(&mut metrics, &observed, &reference);
        if reference.is_empty() {
            metrics.hallucinated_words_during_non_speech = Some(accuracy.hypothesis_words);
        }
        metrics.accuracy = Some(accuracy);
        let report = CaseReport {
            case: case.name.clone(),
            mode: "realtime_capture".into(),
            audio_path: case.audio.display().to_string(),
            reference_path: case
                .reference
                .as_ref()
                .map(|path| path.display().to_string()),
            status: "ok".into(),
            failure: None,
            hypothesis,
            metrics,
        };
        let normalized_reference = normalize(&reference);
        let normalized_hypothesis = normalize(&report.hypothesis);
        if let Err(error) = write_case_artifacts(
            output,
            &report,
            &reference,
            &normalized_reference,
            &normalized_hypothesis,
            session_json.as_deref(),
        ) {
            return failed_report_with_metrics(
                case,
                "realtime_capture",
                report.metrics,
                error.to_string(),
            );
        }
        report
    }
}

#[cfg(target_os = "macos")]
fn spawn_collector(
    events: engine_runtime::Subscription,
    started: Instant,
    observations: Arc<Mutex<Observations>>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            match events.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => observe(event, started, &observations),
                Err(engine_runtime::SubscriptionError::Timeout) => {}
                Err(engine_runtime::SubscriptionError::Lagged { missed }) => {
                    observations
                        .lock()
                        .unwrap_or_else(|lock| lock.into_inner())
                        .errors
                        .push(format!(
                            "benchmark event subscriber missed {missed} event(s)"
                        ));
                }
                Err(engine_runtime::SubscriptionError::Closed) => break,
            }
        }
        loop {
            match events.recv_timeout(Duration::ZERO) {
                Ok(event) => observe(event, started, &observations),
                Err(engine_runtime::SubscriptionError::Lagged { missed }) => observations
                    .lock()
                    .unwrap_or_else(|lock| lock.into_inner())
                    .errors
                    .push(format!(
                        "benchmark event subscriber missed {missed} event(s)"
                    )),
                Err(_) => break,
            }
        }
    })
}

#[cfg(target_os = "macos")]
fn observe(event: EngineEvent, started: Instant, observations: &Mutex<Observations>) {
    let arrival = elapsed_ms(started);
    let mut state = observations.lock().unwrap_or_else(|lock| lock.into_inner());
    match event {
        EngineEvent::TranscriptUpdate { update } => observe_update(update, arrival, &mut state),
        EngineEvent::TranscriptionError { error } => state.errors.push(error.to_string()),
        _ => {}
    }
}

fn observe_update(update: TranscriptUpdate, arrival: u64, state: &mut Observations) {
    let text = update.stable_text.trim().to_owned();
    if update.is_final {
        if state.first_final_ms.is_none() {
            state.first_final_ms = Some(arrival);
            state.first_final_audio_end_ms = Some(update.end_ms);
        }
        state.finalized_audio_ms = Some(state.finalized_audio_ms.unwrap_or(0).max(update.end_ms));
        if !text.is_empty() {
            if let Some((_, existing)) = state
                .finals
                .iter_mut()
                .find(|(id, _)| *id == update.utterance_id)
            {
                if let Some(first_arrival) = state.finalized_at.get(&update.utterance_id) {
                    state
                        .refinement_latencies
                        .push(arrival.saturating_sub(*first_arrival));
                    state.refined_final_ids.insert(update.utterance_id.clone());
                }
                *existing = text;
            } else {
                state
                    .initial_finals
                    .push((update.utterance_id.clone(), text.clone()));
                state
                    .finalized_at
                    .insert(update.utterance_id.clone(), arrival);
                state.finals.push((update.utterance_id, text));
            }
        }
    } else {
        state.first_partial_ms.get_or_insert(arrival);
        state.partial_count += 1;
        if !text.is_empty() {
            state.first_stable_ms.get_or_insert(arrival);
        }
        state.partial_arrivals.push((arrival, update.end_ms));
        let combined = format!("{}{}", update.stable_text, update.unstable_text)
            .trim()
            .to_owned();
        if let Some(previous) = state
            .previous_partials
            .insert(update.utterance_id, combined.clone())
        {
            state
                .partial_stabilities
                .push(text_similarity(&previous, &combined));
        }
    }
}

fn text_similarity(left: &str, right: &str) -> f64 {
    let left = normalize(left)
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let right = normalize(right)
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let distance = edits(&left, &right).total();
    1.0 - distance as f64 / left.len().max(right.len()) as f64
}

fn apply_refinement_metrics(metrics: &mut CaseMetrics, observed: &Observations, reference: &str) {
    let (Some(parakeet), Some(refined)) = (&metrics.parakeet_final, &metrics.whisper_refined_final)
    else {
        return;
    };
    metrics.refinement_changed = Some(normalize(parakeet) != normalize(refined));
    let initial = score(reference, parakeet);
    let final_score = score(reference, refined);
    metrics.refinement_improved = Some(final_score.wer < initial.wer);
    metrics.refinement_worsened = Some(final_score.wer > initial.wer);
    metrics.refinement_latency_ms = mean(&observed.refinement_latencies);
}

fn enhanced_final_texts(
    backend: AsrBackendKind,
    observed: &Observations,
) -> (Option<String>, Option<String>) {
    if backend != AsrBackendKind::Enhanced {
        return (None, None);
    }
    let parakeet_final = (!observed.initial_finals.is_empty()).then(|| {
        observed
            .initial_finals
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let whisper_refined_final = (!observed.refined_final_ids.is_empty()).then(|| {
        observed
            .finals
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    (parakeet_final, whisper_refined_final)
}

fn realtime_metrics(
    options: &BenchmarkOptions,
    wall_ms: u64,
    audio_duration_ms: Option<u64>,
    playback_start_ms: u64,
    snapshot: &EngineSnapshot,
    observed: &Observations,
    resources: &crate::resource::ResourceMetrics,
) -> CaseMetrics {
    let latencies = partial_latency_samples(&observed.partial_arrivals, playback_start_ms);
    let (parakeet_final, whisper_refined_final) = enhanced_final_texts(options.backend, observed);
    let finalization_latency = finalization_latency(
        observed.first_final_ms,
        observed.first_final_audio_end_ms,
        playback_start_ms,
    );
    CaseMetrics {
        audio_duration_ms,
        benchmark_wall_ms: wall_ms,
        backend: snapshot
            .transcription
            .backend
            .clone()
            .unwrap_or_else(|| format!("{:?}", options.backend).to_lowercase()),
        model: snapshot
            .transcription
            .model
            .clone()
            .or_else(|| {
                options
                    .model
                    .as_ref()
                    .map(|path| path.display().to_string())
            })
            .unwrap_or_else(|| "not configured".into()),
        language: display_language(&options.language),
        model_load_ms: nonzero(snapshot.transcription.asr.model_load_ms),
        playback_start_ms: Some(playback_start_ms),
        first_speech_detection_ms: None,
        first_partial_ms: observed
            .first_partial_ms
            .map(|value| value.saturating_sub(playback_start_ms)),
        first_stable_text_ms: observed
            .first_stable_ms
            .map(|value| value.saturating_sub(playback_start_ms)),
        first_final_ms: observed
            .first_final_ms
            .map(|value| value.saturating_sub(playback_start_ms)),
        partial_count: observed.partial_count,
        partial_stability: mean_f64(&observed.partial_stabilities),
        average_partial_latency_ms: mean(&latencies),
        finalization_latency_ms: finalization_latency,
        partial_latency_p50_ms: percentile(&latencies, 50),
        partial_latency_p95_ms: percentile(&latencies, 95),
        vad_segment_count: snapshot.transcription.asr.vad_segments,
        asr_inference_count: snapshot.transcription.asr.inferences,
        average_inference_ms: nonzero(snapshot.transcription.asr.average_inference_ms),
        maximum_inference_ms: nonzero(snapshot.transcription.asr.maximum_inference_ms),
        rtf: nonzero(snapshot.transcription.asr.average_rtf_milli)
            .map(|value| value as f64 / 1000.0),
        cpu_usage_percent: resources.average_cpu_percent,
        average_cpu_percent: resources.average_cpu_percent,
        peak_cpu_percent: resources.peak_cpu_percent,
        average_rss_bytes: resources.average_ram_bytes,
        peak_rss_bytes: resources.peak_ram_bytes,
        microphone_capture_drops: snapshot.dropped_microphone_blocks,
        system_capture_drops: snapshot.dropped_system_blocks,
        speech_pipeline_drops: snapshot.transcription.dropped_blocks,
        asr_coalesced_work: snapshot.transcription.asr.coalesced_work,
        asr_dropped_work: snapshot.transcription.asr.dropped_work,
        asr_dropped_events: snapshot.transcription.asr.dropped_events,
        deadline_misses: snapshot.transcription.asr.deadline_misses,
        parakeet_final,
        whisper_refined_final,
        refinement_latency_ms: mean(&observed.refinement_latencies),
        refinement_changed: None,
        refinement_improved: None,
        refinement_worsened: None,
        hallucinated_words_during_non_speech: None,
        accuracy: None,
    }
}

fn partial_latency_samples(partials: &[(u64, u64)], playback_start_ms: u64) -> Vec<u64> {
    partials
        .iter()
        .map(|(arrival, audio_end)| {
            arrival.saturating_sub(playback_start_ms.saturating_add(*audio_end))
        })
        .collect()
}

fn finalization_latency(
    first_final_ms: Option<u64>,
    final_audio_end_ms: Option<u64>,
    playback_start_ms: u64,
) -> Option<u64> {
    first_final_ms
        .zip(final_audio_end_ms)
        .map(|(final_ms, audio_end_ms)| {
            final_ms.saturating_sub(playback_start_ms.saturating_add(audio_end_ms))
        })
}

#[cfg(target_os = "macos")]
fn runtime_transcription(
    options: &BenchmarkOptions,
    case_language: Option<String>,
) -> engine_runtime::TranscriptionSettings {
    engine_runtime::TranscriptionSettings {
        backend: options.backend,
        model_path: options.model.clone(),
        refinement_model_path: options.refinement_model.clone(),
        language: options.language.clone().or(case_language),
        provider: options.provider.clone(),
        use_gpu: options.use_gpu,
        ..Default::default()
    }
}

fn speech_config(options: &BenchmarkOptions, language: Option<String>) -> SpeechConfig {
    let vad = engine_runtime::TranscriptionSettings::default().vad;
    SpeechConfig {
        backend: options.backend,
        model_path: options.model.clone(),
        refinement_model_path: options.refinement_model.clone(),
        language,
        provider: options.provider.clone(),
        use_gpu: options.use_gpu,
        vad,
        preflight_vad: true,
        ..Default::default()
    }
}

fn discover_corpus(root: &Path) -> Result<Vec<CorpusCase>> {
    let mut audio = Vec::new();
    visit(root, &mut audio)?;
    audio.sort();
    Ok(audio
        .into_iter()
        .map(|path| {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("case");
            let parent = path.parent().unwrap_or(root);
            let reference = [
                parent.join(format!("{stem}.transcription.txt")),
                parent.join(format!("{stem}.txt")),
                parent.join("transcription.txt"),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file());
            let metadata = [
                parent.join(format!("{stem}.json")),
                parent.join("metadata.json"),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file());
            let relative = path.strip_prefix(root).unwrap_or(&path);
            CorpusCase {
                name: safe_name(relative),
                audio: path,
                reference,
                metadata,
            }
        })
        .collect())
}

fn visit(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "__MACOSX" {
            continue;
        }
        if path.is_dir() {
            visit(&path, output)?;
        } else if path.is_file() && is_supported_audio(&path) {
            output.push(path);
        }
    }
    Ok(())
}

fn is_supported_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("mp3" | "wav" | "flac")
    )
}

fn case_language(case: &CorpusCase) -> Option<String> {
    case.metadata
        .as_ref()
        .and_then(|path| fs::read(path).ok())
        .and_then(|data| serde_json::from_slice::<CaseMetadata>(&data).ok())
        .and_then(|metadata| metadata.language)
}

fn effective_language(case: &CorpusCase, options: &BenchmarkOptions) -> Option<String> {
    options.language.clone().or_else(|| case_language(case))
}

fn generated_silence_case(seconds: u64) -> CorpusCase {
    CorpusCase {
        name: format!("generated-silence-{seconds}s"),
        audio: PathBuf::from("<generated-silence>"),
        reference: None,
        metadata: None,
    }
}

fn write_silence_wav(path: &Path, seconds: u64) -> Result<()> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for _ in 0..seconds.saturating_mul(16_000) {
        writer.write_sample(0_i16)?;
    }
    writer.finalize()?;
    Ok(())
}

fn score(reference: &str, hypothesis: &str) -> Accuracy {
    let reference_normalized = normalize(reference);
    let hypothesis_normalized = normalize(hypothesis);
    let reference_words: Vec<&str> = reference_normalized.split_whitespace().collect();
    let hypothesis_words: Vec<&str> = hypothesis_normalized.split_whitespace().collect();
    let word_edits = edits(&reference_words, &hypothesis_words);
    let reference_chars: Vec<char> = reference_normalized
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let hypothesis_chars: Vec<char> = hypothesis_normalized
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let char_edits = edits(&reference_chars, &hypothesis_chars);
    Accuracy {
        wer: error_rate(word_edits.total(), reference_words.len()),
        cer: error_rate(char_edits.total(), reference_chars.len()),
        substitutions: word_edits.substitutions,
        insertions: word_edits.insertions,
        deletions: word_edits.deletions,
        reference_words: reference_words.len(),
        hypothesis_words: hypothesis_words.len(),
        character_substitutions: char_edits.substitutions,
        character_insertions: char_edits.insertions,
        character_deletions: char_edits.deletions,
        reference_characters: reference_chars.len(),
        hypothesis_characters: hypothesis_chars.len(),
    }
}

fn normalize(text: &str) -> String {
    let mut output = String::new();
    let mut space = true;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            output.push(character);
            space = false;
        } else if !space {
            output.push(' ');
            space = true;
        }
    }
    output.trim().to_string()
}

#[derive(Default)]
struct Edits {
    substitutions: usize,
    insertions: usize,
    deletions: usize,
}

impl Edits {
    fn total(&self) -> usize {
        self.substitutions + self.insertions + self.deletions
    }
}

fn edits<T: Eq>(reference: &[T], hypothesis: &[T]) -> Edits {
    let rows = reference.len() + 1;
    let columns = hypothesis.len() + 1;
    let mut costs = vec![0_usize; rows * columns];
    for row in 0..rows {
        costs[row * columns] = row;
    }
    for (column, cost) in costs.iter_mut().take(columns).enumerate() {
        *cost = column;
    }
    for row in 1..rows {
        for column in 1..columns {
            costs[row * columns + column] = if reference[row - 1] == hypothesis[column - 1] {
                costs[(row - 1) * columns + column - 1]
            } else {
                1 + costs[(row - 1) * columns + column - 1]
                    .min(costs[(row - 1) * columns + column])
                    .min(costs[row * columns + column - 1])
            };
        }
    }
    let (mut row, mut column) = (reference.len(), hypothesis.len());
    let mut result = Edits::default();
    while row > 0 || column > 0 {
        if row > 0
            && column > 0
            && reference[row - 1] == hypothesis[column - 1]
            && costs[row * columns + column] == costs[(row - 1) * columns + column - 1]
        {
            row -= 1;
            column -= 1;
        } else if row > 0
            && column > 0
            && costs[row * columns + column] == costs[(row - 1) * columns + column - 1] + 1
        {
            result.substitutions += 1;
            row -= 1;
            column -= 1;
        } else if column > 0
            && costs[row * columns + column] == costs[row * columns + column - 1] + 1
        {
            result.insertions += 1;
            column -= 1;
        } else {
            result.deletions += 1;
            row -= 1;
        }
    }
    result
}

fn error_rate(errors: usize, reference_units: usize) -> f64 {
    if reference_units == 0 {
        if errors == 0 { 0.0 } else { 1.0 }
    } else {
        errors as f64 / reference_units as f64
    }
}

fn write_case_artifacts(
    output: &Path,
    report: &CaseReport,
    reference: &str,
    normalized_reference: &str,
    normalized_hypothesis: &str,
    session_json: Option<&Path>,
) -> Result<()> {
    let directory = output.join(format!("{}--{}", report.mode, report.case));
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("reference.txt"), reference)?;
    fs::write(directory.join("hypothesis.txt"), &report.hypothesis)?;
    fs::write(
        directory.join("normalized-reference.txt"),
        normalized_reference,
    )?;
    fs::write(
        directory.join("normalized-hypothesis.txt"),
        normalized_hypothesis,
    )?;
    fs::write(
        directory.join("metrics.json"),
        serde_json::to_vec_pretty(&report.metrics)?,
    )?;
    if let Some(path) = session_json.filter(|path| path.is_file()) {
        fs::copy(path, directory.join("session.json"))?;
    } else {
        fs::write(
            directory.join("session.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "mode": report.mode,
                "available": false
            }))?,
        )?;
    }
    Ok(())
}

fn write_summary(
    options: &BenchmarkOptions,
    output: &Path,
    timestamp: u64,
    results: &[CaseReport],
) -> Result<()> {
    let successful = results
        .iter()
        .filter(|result| result.status == "ok")
        .count();
    let (word_errors, reference_words, char_errors, reference_chars) = results
        .iter()
        .filter_map(|result| result.metrics.accuracy.as_ref())
        .fold((0, 0, 0, 0), |totals, accuracy| {
            (
                totals.0 + accuracy.substitutions + accuracy.insertions + accuracy.deletions,
                totals.1 + accuracy.reference_words,
                totals.2
                    + accuracy.character_substitutions
                    + accuracy.character_insertions
                    + accuracy.character_deletions,
                totals.3 + accuracy.reference_characters,
            )
        });
    let summary = Summary {
        schema_version: 1,
        suite: SUITE,
        benchmark_version: "1",
        validation_status: "pending",
        generated_at_unix_ms: timestamp,
        git_commit: option_env!("RIMV_GIT_COMMIT").map(str::to_owned),
        environment: system_profile::SystemProfile::detect(),
        normalization: "Unicode lowercase; retain Unicode alphanumeric code points; punctuation and symbols become word boundaries; collapse whitespace; contractions split at punctuation; digits remain digits without number-word conversion; no transliteration or canonical Unicode normalization.",
        corpus: options.corpus.display().to_string(),
        output: output.display().to_string(),
        total_cases: results.len(),
        successful_cases: successful,
        failed_cases: results.len() - successful,
        aggregate_wer: (successful > 0).then(|| error_rate(word_errors, reference_words)),
        aggregate_cer: (successful > 0).then(|| error_rate(char_errors, reference_chars)),
        results,
    };
    fs::write(
        output.join("benchmark-results.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    let mut markdown = format!(
        "# RimV benchmark results\n\n- Suite: `{}`\n- Benchmark version: `{}`\n- Validation: `{}`\n- Corpus: `{}`\n- Generated (Unix ms): `{}`\n- Git commit: `{}`\n- Environment: `{:?}` / `{}` / `{:?}`\n- Cases: {} successful, {} failed\n- Aggregate WER: {}\n- Aggregate CER: {}\n- Normalization: {}\n\n| Case | Mode | Status | WER | CER | RTF |\n|---|---|---:|---:|---:|---:|\n",
        SUITE,
        summary.benchmark_version,
        summary.validation_status,
        options.corpus.display(),
        summary.generated_at_unix_ms,
        summary.git_commit.as_deref().unwrap_or("unknown"),
        summary.environment.os,
        summary.environment.os_version,
        summary.environment.architecture,
        summary.successful_cases,
        summary.failed_cases,
        format_rate(summary.aggregate_wer),
        format_rate(summary.aggregate_cer),
        summary.normalization,
    );
    for result in results {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            result.case,
            result.mode,
            result.status,
            format_rate(result.metrics.accuracy.as_ref().map(|value| value.wer)),
            format_rate(result.metrics.accuracy.as_ref().map(|value| value.cer)),
            result
                .metrics
                .rtf
                .map_or_else(|| "n/a".into(), |value| format!("{value:.3}")),
        ));
    }
    fs::write(output.join("benchmark-results.md"), markdown)?;
    Ok(())
}

fn failed_report(
    case: &CorpusCase,
    mode: &str,
    options: &BenchmarkOptions,
    error: String,
) -> CaseReport {
    failed_report_with_metrics(
        case,
        mode,
        empty_metrics(format!("{:?}", options.backend).to_lowercase(), 0),
        error,
    )
}

fn failed_report_with_metrics(
    case: &CorpusCase,
    mode: &str,
    metrics: CaseMetrics,
    error: String,
) -> CaseReport {
    CaseReport {
        case: case.name.clone(),
        mode: mode.into(),
        audio_path: case.audio.display().to_string(),
        reference_path: case
            .reference
            .as_ref()
            .map(|path| path.display().to_string()),
        status: "failed".into(),
        failure: Some(error),
        hypothesis: String::new(),
        metrics,
    }
}

fn empty_metrics(backend: String, wall_ms: u64) -> CaseMetrics {
    CaseMetrics {
        audio_duration_ms: None,
        benchmark_wall_ms: wall_ms,
        backend,
        model: "unavailable".into(),
        language: "auto".into(),
        model_load_ms: None,
        playback_start_ms: None,
        first_speech_detection_ms: None,
        first_partial_ms: None,
        first_stable_text_ms: None,
        first_final_ms: None,
        partial_count: 0,
        partial_stability: None,
        average_partial_latency_ms: None,
        finalization_latency_ms: None,
        partial_latency_p50_ms: None,
        partial_latency_p95_ms: None,
        vad_segment_count: 0,
        asr_inference_count: 0,
        average_inference_ms: None,
        maximum_inference_ms: None,
        rtf: None,
        cpu_usage_percent: None,
        average_cpu_percent: None,
        peak_cpu_percent: None,
        average_rss_bytes: None,
        peak_rss_bytes: None,
        microphone_capture_drops: 0,
        system_capture_drops: 0,
        speech_pipeline_drops: 0,
        asr_coalesced_work: 0,
        asr_dropped_work: 0,
        asr_dropped_events: 0,
        deadline_misses: 0,
        parakeet_final: None,
        whisper_refined_final: None,
        refinement_latency_ms: None,
        refinement_changed: None,
        refinement_improved: None,
        refinement_worsened: None,
        hallucinated_words_during_non_speech: None,
        accuracy: None,
    }
}

fn options_backend_placeholder() -> String {
    "unavailable".into()
}

fn print_case(report: &CaseReport) {
    if let Some(accuracy) = &report.metrics.accuracy {
        println!(
            "{} [{}]: WER {:.3}, CER {:.3}",
            report.case, report.mode, accuracy.wer, accuracy.cer
        );
    } else {
        println!(
            "{} [{}]: failed: {}",
            report.case,
            report.mode,
            report.failure.as_deref().unwrap_or("unknown failure")
        );
    }
}

fn safe_name(path: &Path) -> String {
    let raw = path.with_extension("").to_string_lossy().to_string();
    raw.chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn display_language(language: &Option<String>) -> String {
    language.clone().unwrap_or_else(|| "auto".into())
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn mean(values: &[u64]) -> Option<u64> {
    (!values.is_empty()).then(|| values.iter().sum::<u64>() / values.len() as u64)
}

fn mean_f64(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn percentile(values: &[u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() - 1) * percentile / 100;
    Some(sorted[index])
}

fn nonzero(value: u64) -> Option<u64> {
    (value != 0).then_some(value)
}

fn format_rate(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".into(), |value| format!("{value:.3}"))
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn unix_ms() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_apple_permission_error_identifies_the_launching_app() {
        let error = "Apple Speech unavailable: Speech Recognition permission is denied.";
        let guided = benchmark_engine_error(AsrBackendKind::AppleNative, error);
        assert!(guided.contains("enable the launching terminal app"));
        assert!(guided.contains("RimV's app permission does not cover it"));

        assert_eq!(
            benchmark_engine_error(AsrBackendKind::Parakeet, error),
            error
        );
    }

    #[test]
    fn engine_identity_rejects_any_substitution() {
        let info = AsrBackendInfo {
            backend_id: "whisper.cpp".into(),
            backend_name: "Whisper".into(),
            model_id: "whisper-local".into(),
            model_name: "Whisper model".into(),
            capabilities: speech_transcription::AsrCapabilities {
                supports_incremental_audio: false,
                supports_partial_results: false,
                supports_word_timestamps: false,
                supports_language_detection: false,
                supports_true_streaming: false,
            },
        };
        assert!(validate_engine_identity(AsrBackendKind::Whisper, &info).is_ok());
        assert!(
            validate_engine_identity(AsrBackendKind::Parakeet, &info)
                .unwrap_err()
                .to_string()
                .contains("no fallback allowed")
        );
    }

    #[test]
    fn enhanced_requires_both_explicit_models() {
        let corpus = std::env::temp_dir().join(format!("rimv-benchmark-{}", std::process::id()));
        fs::create_dir_all(&corpus).unwrap();
        let options = BenchmarkOptions {
            corpus: corpus.clone(),
            mode: BenchmarkMode::RealtimeReplay,
            output: None,
            model: None,
            refinement_model: None,
            backend: AsrBackendKind::Enhanced,
            language: None,
            provider: "cpu".into(),
            use_gpu: false,
            #[cfg(target_os = "macos")]
            warmup: Duration::ZERO,
            #[cfg(target_os = "macos")]
            trailing: Duration::ZERO,
            silence_seconds: 1,
            include_silence: false,
            max_wer: None,
        };
        assert!(
            validate_options(&options)
                .unwrap_err()
                .to_string()
                .contains("requires both")
        );
        fs::remove_dir_all(corpus).unwrap();
    }

    #[test]
    fn ratio_is_inference_time_over_audio_duration() {
        assert_eq!(ratio(250, 1_000), 0.25);
        assert_eq!(ratio(10, 0), 0.0);
    }

    #[test]
    fn realtime_latency_uses_playback_and_audio_timestamps() {
        assert_eq!(partial_latency_samples(&[(1_550, 100)], 1_000), vec![450]);
        assert_eq!(
            finalization_latency(Some(2_300), Some(1_000), 1_000),
            Some(300)
        );
        assert_eq!(
            finalization_latency(Some(2_300), Some(800), 1_000),
            Some(500)
        );
        assert_eq!(finalization_latency(Some(900), Some(1_000), 0), Some(0));
    }

    fn update(id: &str, text: &str, is_final: bool) -> TranscriptUpdate {
        TranscriptUpdate {
            source: AudioSource::Microphone,
            utterance_id: id.into(),
            start_ms: 0,
            end_ms: 500,
            stable_text: text.into(),
            unstable_text: String::new(),
            is_final,
            language: Some("en".into()),
            confidence: None,
        }
    }

    #[test]
    fn partial_final_refinement_replaces_same_utterance_without_duplicate_final() {
        let mut observed = Observations::default();
        observe_update(update("utterance-1", "hello", false), 100, &mut observed);
        observe_update(
            update("utterance-1", "hello world", true),
            600,
            &mut observed,
        );
        observe_update(
            update("utterance-1", "hello there", true),
            900,
            &mut observed,
        );
        assert_eq!(observed.partial_count, 1);
        assert_eq!(
            observed.initial_finals,
            vec![("utterance-1".into(), "hello world".into())]
        );
        assert_eq!(
            observed.finals,
            vec![("utterance-1".into(), "hello there".into())]
        );
        assert_eq!(observed.refinement_latencies, vec![300]);
    }

    #[test]
    fn enhanced_report_keeps_parakeet_fallback_and_adds_refined_text_when_available() {
        let mut observed = Observations::default();
        observe_update(
            update("utterance-1", "Parakeet final", true),
            600,
            &mut observed,
        );
        assert_eq!(
            enhanced_final_texts(AsrBackendKind::Enhanced, &observed),
            (Some("Parakeet final".into()), None)
        );

        observe_update(
            update("utterance-1", "Whisper refinement", true),
            900,
            &mut observed,
        );
        assert_eq!(
            enhanced_final_texts(AsrBackendKind::Enhanced, &observed),
            (
                Some("Parakeet final".into()),
                Some("Whisper refinement".into())
            )
        );
        assert_eq!(
            enhanced_final_texts(AsrBackendKind::Parakeet, &observed),
            (None, None)
        );
    }

    #[test]
    fn partial_stability_uses_successive_text_similarity() {
        assert_eq!(text_similarity("hello world", "hello there"), 0.5);
        assert_eq!(text_similarity("", ""), 1.0);
    }

    #[test]
    fn conservative_normalization_preserves_letters_and_digits() {
        assert_eq!(normalize("  ¡Hola, MÜNCHEN! 42. "), "hola münchen 42");
    }

    #[test]
    fn normalization_splits_contractions_and_keeps_unicode_and_numbers() {
        assert_eq!(
            normalize("Don't stop at 21st Straße — 東京!"),
            "don t stop at 21st straße 東京"
        );
        assert_eq!(score("café", "cafe").cer, 0.25);
    }

    #[test]
    fn wer_and_cer_report_edit_types() {
        let result = score("one two three", "one too four extra");
        assert_eq!(result.substitutions, 2);
        assert_eq!(result.insertions, 1);
        assert_eq!(result.deletions, 0);
        assert_eq!(result.reference_words, 3);
        assert_eq!(result.hypothesis_words, 4);
        assert_eq!(result.wer, 1.0);
        assert!(result.cer > 0.0);
    }

    #[test]
    fn silence_score_counts_hallucination_as_error() {
        assert_eq!(score("", "").wer, 0.0);
        assert_eq!(score("", "hello").wer, 1.0);
    }

    #[test]
    fn corpus_discovery_is_recursive_and_ignores_hidden_files() {
        let root = std::env::temp_dir().join(format!("rimv-corpus-{}", std::process::id()));
        let nested = root.join("conversation");
        let hidden = root.join(".hidden");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&hidden).unwrap();
        fs::write(nested.join("audio.mp3"), []).unwrap();
        fs::write(nested.join("transcription.txt"), "hello").unwrap();
        fs::write(hidden.join("ignored.wav"), []).unwrap();
        let cases = discover_corpus(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(cases.len(), 1);
        assert!(
            cases[0]
                .reference
                .as_ref()
                .unwrap()
                .ends_with("transcription.txt")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn percentiles_are_deterministic() {
        assert_eq!(percentile(&[40, 10, 30, 20], 50), Some(20));
        assert_eq!(percentile(&[40, 10, 30, 20], 95), Some(30));
        assert_eq!(percentile(&[], 95), None);
    }

    #[test]
    fn configured_wer_gate_marks_only_excessive_scores_failed() {
        let mut reports = vec![CaseReport {
            case: "case".into(),
            mode: "direct_file".into(),
            audio_path: "audio.mp3".into(),
            reference_path: None,
            status: "ok".into(),
            failure: None,
            hypothesis: "hello".into(),
            metrics: CaseMetrics {
                accuracy: Some(Accuracy {
                    wer: 0.5,
                    cer: 0.5,
                    substitutions: 1,
                    insertions: 0,
                    deletions: 0,
                    reference_words: 2,
                    hypothesis_words: 2,
                    character_substitutions: 1,
                    character_insertions: 0,
                    character_deletions: 0,
                    reference_characters: 2,
                    hypothesis_characters: 2,
                }),
                ..empty_metrics("test".into(), 0)
            },
        }];
        apply_wer_gate(&mut reports, Some(0.4));
        assert_eq!(reports[0].status, "failed");
        assert!(reports[0].failure.as_deref().unwrap().contains("WER 0.500"));
    }

    #[test]
    fn generated_silence_is_a_valid_wav_without_committed_fixture() {
        let path =
            std::env::temp_dir().join(format!("rimv-generated-silence-{}.wav", std::process::id()));
        write_silence_wav(&path, 1).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.duration(), 16_000);
        fs::remove_file(path).unwrap();
    }
}
