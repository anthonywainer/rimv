use clap::ValueEnum;
#[cfg(target_os = "macos")]
use engine_protocol::{AudioSource, EngineEvent, EngineSnapshot};
use engine_runtime::AsrBackendKind;
#[cfg(target_os = "macos")]
use engine_runtime::{EngineConfig, EngineRuntime};
use serde::{Deserialize, Serialize};
use speech_transcription::{
    AsrBackendInfo, SpeechConfig, SpeechToTextEngine, load_configured_backend,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "macos")]
use std::{
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{Result, media};

const SUITE: &str = concat!("rimv-", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum BenchmarkMode {
    DirectFile,
    RealtimeCapture,
    All,
}

pub(crate) struct BenchmarkOptions {
    pub corpus: PathBuf,
    pub mode: BenchmarkMode,
    pub output: Option<PathBuf>,
    pub model: Option<PathBuf>,
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
    peak_rss_bytes: Option<u64>,
    microphone_capture_drops: u64,
    system_capture_drops: u64,
    speech_pipeline_drops: u64,
    asr_coalesced_work: u64,
    asr_dropped_work: u64,
    asr_dropped_events: u64,
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
    generated_at_unix_ms: u64,
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

#[cfg(target_os = "macos")]
#[derive(Default)]
struct Observations {
    finals: Vec<String>,
    first_partial_ms: Option<u64>,
    first_stable_ms: Option<u64>,
    first_final_ms: Option<u64>,
    partial_latencies_ms: Vec<u64>,
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
    if matches!(options.mode, BenchmarkMode::DirectFile | BenchmarkMode::All) {
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
        BenchmarkMode::RealtimeCapture | BenchmarkMode::All
    ) {
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
        output.join("summary.md").display()
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
    if options
        .max_wer
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("--max-wer must be a finite value greater than or equal to zero".into());
    }
    #[cfg(not(target_os = "macos"))]
    if matches!(
        options.mode,
        BenchmarkMode::RealtimeCapture | BenchmarkMode::All
    ) {
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
        let backend = load_configured_backend(speech_config(options, language))?;
        let model_load_ms = elapsed_ms(started);
        let info = backend.info();
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
    let result = (|| -> Result<(String, u64, u64)> {
        let decoded = media::decode_mono_16k(&case.audio)?;
        let inference_started = Instant::now();
        let segments = runner.backend.transcribe(&decoded.samples, 0)?;
        Ok((
            segments
                .into_iter()
                .map(|segment| segment.text)
                .collect::<Vec<_>>()
                .join(" "),
            decoded.duration_ms,
            elapsed_ms(inference_started),
        ))
    })();
    match result {
        Ok((hypothesis, duration_ms, inference_ms)) => finish_scored_case(
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
                average_partial_latency_ms: None,
                finalization_latency_ms: None,
                partial_latency_p50_ms: None,
                partial_latency_p95_ms: None,
                vad_segment_count: 0,
                asr_inference_count: 1,
                average_inference_ms: Some(direct.inference_ms),
                maximum_inference_ms: Some(direct.inference_ms),
                rtf: Some(ratio(direct.inference_ms, direct.audio_duration_ms)),
                cpu_usage_percent: None,
                peak_rss_bytes: None,
                microphone_capture_drops: 0,
                system_capture_drops: 0,
                speech_pipeline_drops: 0,
                asr_coalesced_work: 0,
                asr_dropped_work: 0,
                asr_dropped_events: 0,
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
        let playback = Command::new("/usr/bin/afplay").arg(&case.audio).status();
        let playback_failure = match playback {
            Ok(status) if status.success() => None,
            Ok(status) => Some(format!("afplay exited with {status}")),
            Err(error) => Some(format!("could not start afplay: {error}")),
        };
        thread::sleep(options.trailing);
        let stopped = engine.stop_capture();
        let snapshot = engine.snapshot();
        let session_json = snapshot
            .session
            .as_ref()
            .map(|session| PathBuf::from(&session.recording_directory).join("session.json"));
        let _ = engine.shutdown();
        collector_stop.store(true, Ordering::Release);
        let _ = collector.join();
        let observed = observations.lock().unwrap_or_else(|lock| lock.into_inner());
        let hypothesis = observed.finals.join(" ");
        let failure = playback_failure
            .or_else(|| stopped.err().map(|error| error.to_string()))
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
        );
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
        EngineEvent::TranscriptUpdate { update } if update.source == AudioSource::System => {
            if update.is_final {
                state.first_final_ms.get_or_insert(arrival);
                if !update.stable_text.trim().is_empty() {
                    state.finals.push(update.stable_text);
                }
            } else {
                state.first_partial_ms.get_or_insert(arrival);
                if !update.stable_text.trim().is_empty() {
                    state.first_stable_ms.get_or_insert(arrival);
                }
                state
                    .partial_latencies_ms
                    .push(arrival.saturating_sub(update.end_ms));
            }
        }
        EngineEvent::TranscriptionError { error } => state.errors.push(error.to_string()),
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn realtime_metrics(
    options: &BenchmarkOptions,
    wall_ms: u64,
    audio_duration_ms: Option<u64>,
    playback_start_ms: u64,
    snapshot: &EngineSnapshot,
    observed: &Observations,
) -> CaseMetrics {
    let latencies = &observed.partial_latencies_ms;
    let finalization_latency = observed.first_final_ms.and_then(|final_ms| {
        audio_duration_ms.map(|duration| final_ms.saturating_sub(playback_start_ms + duration))
    });
    CaseMetrics {
        audio_duration_ms,
        benchmark_wall_ms: wall_ms,
        backend: format!("{:?}", options.backend).to_lowercase(),
        model: options.model.as_ref().map_or_else(
            || "not configured".into(),
            |path| path.display().to_string(),
        ),
        language: display_language(&options.language),
        model_load_ms: None,
        playback_start_ms: Some(playback_start_ms),
        first_speech_detection_ms: None,
        first_partial_ms: observed.first_partial_ms,
        first_stable_text_ms: observed.first_stable_ms,
        first_final_ms: observed.first_final_ms,
        average_partial_latency_ms: mean(latencies),
        finalization_latency_ms: finalization_latency,
        partial_latency_p50_ms: percentile(latencies, 50),
        partial_latency_p95_ms: percentile(latencies, 95),
        vad_segment_count: snapshot.transcription.asr.vad_segments,
        asr_inference_count: snapshot.transcription.asr.inferences,
        average_inference_ms: nonzero(snapshot.transcription.asr.average_inference_ms),
        maximum_inference_ms: nonzero(snapshot.transcription.asr.maximum_inference_ms),
        rtf: nonzero(snapshot.transcription.asr.average_rtf_milli)
            .map(|value| value as f64 / 1000.0),
        cpu_usage_percent: None,
        peak_rss_bytes: None,
        microphone_capture_drops: snapshot.dropped_microphone_blocks,
        system_capture_drops: snapshot.dropped_system_blocks,
        speech_pipeline_drops: snapshot.transcription.dropped_blocks,
        asr_coalesced_work: snapshot.transcription.asr.coalesced_work,
        asr_dropped_work: snapshot.transcription.asr.dropped_work,
        asr_dropped_events: snapshot.transcription.asr.dropped_events,
        hallucinated_words_during_non_speech: None,
        accuracy: None,
    }
}

#[cfg(target_os = "macos")]
fn runtime_transcription(
    options: &BenchmarkOptions,
    case_language: Option<String>,
) -> engine_runtime::TranscriptionSettings {
    engine_runtime::TranscriptionSettings {
        backend: options.backend,
        model_path: options.model.clone(),
        language: options.language.clone().or(case_language),
        provider: options.provider.clone(),
        use_gpu: options.use_gpu,
        ..Default::default()
    }
}

fn speech_config(options: &BenchmarkOptions, language: Option<String>) -> SpeechConfig {
    SpeechConfig {
        backend: options.backend,
        model_path: options.model.clone(),
        language,
        provider: options.provider.clone(),
        use_gpu: options.use_gpu,
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
        generated_at_unix_ms: timestamp,
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
        output.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    let mut markdown = format!(
        "# rimv benchmark\n\n- Suite: `{}`\n- Corpus: `{}`\n- Cases: {} successful, {} failed\n- Aggregate WER: {}\n- Aggregate CER: {}\n\n| Case | Mode | Status | WER | CER | RTF |\n|---|---|---:|---:|---:|---:|\n",
        SUITE,
        options.corpus.display(),
        summary.successful_cases,
        summary.failed_cases,
        format_rate(summary.aggregate_wer),
        format_rate(summary.aggregate_cer),
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
    fs::write(output.join("summary.md"), markdown)?;
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
        peak_rss_bytes: None,
        microphone_capture_drops: 0,
        system_capture_drops: 0,
        speech_pipeline_drops: 0,
        asr_coalesced_work: 0,
        asr_dropped_work: 0,
        asr_dropped_events: 0,
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

#[cfg(target_os = "macos")]
fn mean(values: &[u64]) -> Option<u64> {
    (!values.is_empty()).then(|| values.iter().sum::<u64>() / values.len() as u64)
}

#[cfg(target_os = "macos")]
fn percentile(values: &[u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() - 1) * percentile / 100;
    Some(sorted[index])
}

#[cfg(target_os = "macos")]
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
    fn conservative_normalization_preserves_letters_and_digits() {
        assert_eq!(normalize("  ¡Hola, MÜNCHEN! 42. "), "hola münchen 42");
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
