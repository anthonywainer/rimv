#![cfg(target_os = "macos")]

use audio_capture::CaptureMetrics;
use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use engine_runtime::backend::{CaptureBackend, CaptureStream};
use engine_runtime::{
    AsrBackendKind, EngineCapabilities, EngineConfig, EngineEvent, EngineRuntime, EngineStatus,
    SourceConfig, SourceSettings, TranscriptionSettings,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const FRAME_SAMPLES: usize = 1_024;
const FRAME_MILLIS: u64 = 64;

#[test]
fn apple_speech_locale_capability_diagnostic() {
    let catalog = engine_runtime::apple_speech_locale_catalog();
    let supported = catalog.supported;
    let on_device = catalog.supports_on_device;
    let usable_on_device = catalog.usable_on_device;
    println!("Apple Speech supported locales:");
    for locale in &supported {
        println!("    {locale}");
    }
    println!("Apple Speech on-device locales:");
    for locale in &on_device {
        println!("    {locale}");
    }
    println!("Apple Speech currently usable on-device locales:");
    for locale in &usable_on_device {
        println!("    {locale}");
    }
    println!(
        "Apple Speech locale counts: supported={}, supports-on-device={}, currently-usable-on-device={}",
        supported.len(),
        on_device.len(),
        usable_on_device.len()
    );
    for locale in ["en-US", "es-ES"] {
        println!(
            "Apple Speech on-device {locale}: {}",
            if on_device.iter().any(|candidate| candidate == locale) {
                if usable_on_device.iter().any(|candidate| candidate == locale) {
                    "supported and currently usable"
                } else {
                    "supports on-device, but is currently unavailable"
                }
            } else {
                "does not support on-device recognition"
            }
        );
    }
    assert!(on_device.iter().all(|locale| supported.contains(locale)));
}

#[test]
#[ignore = "requires macOS Apple Speech runtime, on-device en-US support, and Speech permission"]
fn existing_english_fixture_streams_through_native_apple_and_updates_live_state() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest
        .join("../../resources/audio/audio1-16k-mono.wav")
        .canonicalize()
        .expect("locate existing E2E fixture");
    let reference_path = manifest
        .join("../../resources/audio/audio1.txt")
        .canonicalize()
        .expect("locate existing reference transcript");
    eprintln!("Apple Native E2E: resolved fixture and reference paths");
    assert!(
        fixture.is_file(),
        "missing existing E2E fixture: {}",
        fixture.display()
    );
    assert!(
        reference_path.is_file(),
        "missing reference transcript: {}",
        reference_path.display()
    );

    let locale =
        engine_runtime::apple_locale("en").expect("fixture language must map to Apple locale");
    if !engine_runtime::apple_speech_authorized() {
        panic!(
            "UNSUPPORTED E2E CONDITION: this test process has no Speech Recognition permission; grant permission and ensure Apple has downloaded the on-device {locale} Speech asset"
        );
    }
    eprintln!("Apple Native E2E: checking on-device locale support for {locale}");
    if !engine_runtime::apple_speech_available(locale) {
        panic!(
            "UNSUPPORTED E2E CONDITION: Apple Speech does not report on-device support for fixture locale {locale}"
        );
    }
    eprintln!("Apple Native E2E: locale support check passed");

    let mut wav = hound::WavReader::open(&fixture).expect("open existing PCM fixture");
    let spec = wav.spec();
    assert_eq!(spec.sample_rate, 16_000, "fixture format changed");
    assert_eq!(spec.channels, 1, "fixture format changed");
    let samples = wav
        .samples::<i16>()
        .map(|sample| sample.expect("read fixture sample") as f32 / i16::MAX as f32)
        .collect::<Vec<_>>();
    let audio_duration =
        Duration::from_secs_f64(samples.len() as f64 / f64::from(spec.sample_rate));
    let chunk_count = samples.len().div_ceil(FRAME_SAMPLES);
    let format = AudioFormat::new(spec.sample_rate, spec.channels).unwrap();
    let completed = Arc::new(AtomicBool::new(false));
    let recordings = tempfile::tempdir().expect("create temporary recordings directory");

    let config = EngineConfig {
        recordings_directory: recordings.path().to_path_buf(),
        microphone: SourceSettings {
            enabled: true,
            configured: SourceConfig {
                preferred_sample_rate: Some(spec.sample_rate),
                preferred_channels: Some(spec.channels),
                ..Default::default()
            },
        },
        transcription_enabled: true,
        transcription: TranscriptionSettings {
            backend: AsrBackendKind::AppleNative,
            language: Some(locale.into()),
            ..Default::default()
        },
        subscriber_capacity: 1_024,
        ..Default::default()
    };
    let engine = EngineRuntime::with_backend(
        config,
        FixtureBackend {
            samples,
            format,
            completed: completed.clone(),
        },
    )
    .expect("create RimV runtime");
    let events = engine.subscribe().expect("subscribe to RimV events");
    eprintln!("Apple Native E2E: starting LIVE session and Speech provider");
    let started = engine.start_capture().expect("start Native Apple session");
    eprintln!("Apple Native E2E: Native Speech initialized; streaming fixture audio");
    assert_eq!(
        started.status,
        EngineStatus::Recording,
        "session did not become LIVE"
    );
    assert_eq!(
        started.transcription.backend.as_deref(),
        Some("Apple Speech")
    );
    assert_eq!(
        started.transcription.status,
        engine_runtime::TranscriptionStatus::Ready
    );
    assert!(
        started.transcription.supports_partial_results,
        "Apple Native did not report partial-result support"
    );
    assert!(
        started.last_error.is_none(),
        "startup error remained after successful initialization: {:?}",
        started.last_error
    );
    assert!(
        started.microphone.active,
        "fixture input adapter did not become active"
    );

    let playback_started = Instant::now();
    let deadline = playback_started + audio_duration + Duration::from_secs(90);
    let mut observed = Observed::default();
    while !completed.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "timed out while streaming fixture audio"
        );
        collect_events(
            &events,
            &mut observed,
            playback_started,
            Duration::from_millis(100),
        );
        let snapshot = engine.snapshot();
        assert_eq!(
            snapshot.status,
            EngineStatus::Recording,
            "session left LIVE during fixture playback"
        );
        assert!(
            snapshot.last_error.is_none(),
            "stale startup/runtime error during LIVE session: {:?}",
            snapshot.last_error
        );
    }

    let stopped = engine
        .stop_capture()
        .expect("flush and stop Native Apple session");
    collect_events(
        &events,
        &mut observed,
        playback_started,
        Duration::from_millis(100),
    );
    let live_transcript = engine.transcript_snapshot();
    let final_updates = live_transcript
        .updates
        .iter()
        .filter(|update| update.is_final)
        .collect::<Vec<_>>();
    let actual = final_updates
        .iter()
        .map(|update| update.stable_text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let expected = std::fs::read_to_string(&reference_path).expect("read fixture reference");
    let accuracy = score(&expected, &actual);
    let session_directory = stopped
        .session
        .as_ref()
        .map(|session| PathBuf::from(&session.recording_directory));
    let metadata = session_directory
        .as_ref()
        .and_then(|directory| std::fs::read_to_string(directory.join("session.json")).ok())
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok());
    let provider = metadata.as_ref().and_then(|metadata| {
        metadata
            .get("transcription_engine")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    });
    let persisted_locale = metadata.as_ref().and_then(|metadata| {
        metadata
            .get("transcription_language")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    });
    let persisted = session_directory
        .as_ref()
        .and_then(|directory| std::fs::read(directory.join("transcript.json")).ok())
        .and_then(|bytes| {
            serde_json::from_slice::<Vec<engine_runtime::TranscriptLine>>(&bytes).ok()
        })
        .unwrap_or_default();
    let persisted_text = persisted
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    println!(
        "Engine: Native / Apple Speech ({})",
        provider.as_deref().unwrap_or("provider identity missing")
    );
    println!("Apple API/provider: SFSpeechRecognizer, on-device");
    println!("Fixture: {}", fixture.display());
    println!(
        "Locale: {locale} (session metadata: {})",
        persisted_locale.as_deref().unwrap_or("missing")
    );
    println!("Audio duration: {:.3}s", audio_duration.as_secs_f64());
    println!(
        "Audio chunks: {chunk_count} ({} samples, {FRAME_MILLIS}ms cadence)",
        FRAME_SAMPLES
    );
    println!("Partial events: {}", observed.partials.len());
    println!("Final events: {}", observed.finals);
    println!(
        "Time to first Partial: {}",
        format_elapsed(observed.first_partial)
    );
    println!(
        "Time to first Final: {}",
        format_elapsed(observed.first_final)
    );
    println!("Partial event times (ms): {:?}", observed.partial_times_ms);
    println!("Final transcript: {actual}");
    println!("Persisted final transcript: {persisted_text}");
    println!(
        "Expected transcript ({}): {}",
        reference_path.display(),
        expected.trim()
    );
    println!("WER: {:.3}; CER: {:.3}", accuracy.wer, accuracy.cer);
    println!(
        "Stale startup error present: {}",
        stopped.last_error.is_some()
    );
    let mut failures = Vec::new();
    if provider.as_deref() != Some("native_apple") {
        failures.push("transcription identity mismatch or fallback".to_owned());
    }
    if persisted_locale.as_deref() != Some(locale) {
        failures.push("session locale metadata does not match the fixture locale".to_owned());
    }
    if observed.partials.len() < 2 {
        failures.push("fewer than two meaningful Partial events".to_owned());
    }
    if observed.finals == 0 {
        failures.push("no Final events".to_owned());
    }
    if observed.first_partial.is_none() {
        failures.push("no meaningful partial recognition".to_owned());
    }
    if observed.first_final.is_none() {
        failures.push("no Final event timing".to_owned());
    }
    if observed.first_partial >= observed.first_final {
        failures.push("first Partial did not precede first Final".to_owned());
    }
    let mut partial_counts = HashMap::<&str, usize>::new();
    for update in &observed.partials {
        *partial_counts
            .entry(update.utterance_id.as_str())
            .or_default() += 1;
    }
    if !partial_counts.values().any(|count| *count >= 2) {
        failures.push("Partial updates did not replace the same mutable utterance".to_owned());
    }
    let mut ids = HashSet::new();
    if !final_updates
        .iter()
        .all(|update| !update.stable_text.is_empty() && ids.insert(update.utterance_id.as_str()))
    {
        failures.push("final transcript state has empty or duplicate utterances".to_owned());
    }
    if observed.finals != final_updates.len() {
        failures.push("event stream and final transcript state disagree".to_owned());
    }
    if live_transcript
        .updates
        .iter()
        .any(|update| !update.is_final)
    {
        failures.push("partial remained in live transcript state after finalization".to_owned());
    }
    if persisted_text != actual {
        failures.push("persisted transcript differs from RimV final transcript state".to_owned());
    }
    for (partial_position, (utterance_id, is_final)) in observed.event_order.iter().enumerate() {
        if *is_final {
            continue;
        }
        if !observed.event_order[partial_position + 1..]
            .iter()
            .any(|(id, final_result)| id == utterance_id && *final_result)
        {
            failures.push(format!(
                "partial did not get promoted by a later Final for utterance {utterance_id}"
            ));
        }
        if !ids.contains(utterance_id.as_str()) {
            failures.push(format!(
                "partial utterance {utterance_id} is absent from final transcript state"
            ));
        }
    }
    if !normalized(&actual).contains("travel plans") {
        failures.push("existing reference phrase check failed: travel plans".to_owned());
    }
    if !normalized(&actual).contains("san francisco") {
        failures.push("existing reference phrase check failed: san francisco".to_owned());
    }
    if stopped.last_error.is_some() {
        failures.push(format!(
            "stale startup error remained: {:?}",
            stopped.last_error
        ));
    }
    if stopped.session.is_none() {
        failures.push("successful E2E session was not finalized".to_owned());
    }
    engine.shutdown().expect("shutdown RimV runtime");
    println!(
        "Result: {}",
        if failures.is_empty() {
            "PASS".to_owned()
        } else {
            format!("FAIL: {}", failures.join("; "))
        }
    );
    assert!(failures.is_empty(), "Apple Native E2E checks failed");
}

#[derive(Default)]
struct Observed {
    partials: Vec<engine_runtime::TranscriptUpdate>,
    finals: usize,
    first_partial: Option<Duration>,
    first_final: Option<Duration>,
    partial_times_ms: Vec<u128>,
    event_order: Vec<(String, bool)>,
}

fn collect_events(
    subscription: &engine_runtime::Subscription,
    observed: &mut Observed,
    started: Instant,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        match subscription.recv_timeout(remaining) {
            Ok(EngineEvent::TranscriptUpdate { update }) if update.is_final => {
                observed.finals += 1;
                observed.event_order.push((update.utterance_id, true));
                observed
                    .first_final
                    .get_or_insert_with(|| started.elapsed());
            }
            Ok(EngineEvent::TranscriptUpdate { update }) => {
                let text = format!("{}{}", update.stable_text, update.unstable_text);
                if !text.trim().is_empty() {
                    let elapsed = started.elapsed();
                    observed.first_partial.get_or_insert(elapsed);
                    observed.partial_times_ms.push(elapsed.as_millis());
                    observed
                        .event_order
                        .push((update.utterance_id.clone(), false));
                    observed.partials.push(update);
                }
            }
            Ok(_) => {}
            Err(engine_runtime::SubscriptionError::Timeout) => return,
            Err(engine_runtime::SubscriptionError::Lagged { missed }) => {
                panic!("E2E event subscriber lagged and missed {missed} events");
            }
            Err(engine_runtime::SubscriptionError::Closed) => {
                panic!("RimV event stream closed during LIVE E2E")
            }
        }
    }
}

struct FixtureBackend {
    samples: Vec<f32>,
    format: AudioFormat,
    completed: Arc<AtomicBool>,
}

impl CaptureBackend for FixtureBackend {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            microphone_capture: true,
            system_audio_capture: false,
            transcription: true,
        }
    }

    fn open(
        &mut self,
        source: engine_runtime::AudioSource,
        _: &SourceConfig,
    ) -> engine_runtime::Result<Box<dyn CaptureStream>> {
        assert_eq!(source, engine_runtime::AudioSource::Microphone);
        let frames = self
            .samples
            .chunks(FRAME_SAMPLES)
            .enumerate()
            .map(|(index, samples)| {
                AudioFrame::new(
                    AudioSourceKind::Microphone,
                    Duration::from_millis(index as u64 * FRAME_MILLIS),
                    self.format,
                    samples.to_vec(),
                )
                .expect("construct fixture audio frame")
            })
            .collect();
        Ok(Box::new(FixtureStream {
            frames,
            index: 0,
            started: None,
            running: false,
            completed: self.completed.clone(),
        }))
    }
}

struct FixtureStream {
    frames: Vec<AudioFrame>,
    index: usize,
    started: Option<Instant>,
    running: bool,
    completed: Arc<AtomicBool>,
}

impl CaptureStream for FixtureStream {
    fn start(&mut self) -> engine_runtime::Result<()> {
        self.started = Some(Instant::now());
        self.running = true;
        Ok(())
    }
    fn stop(&mut self) -> engine_runtime::Result<()> {
        self.running = false;
        Ok(())
    }
    fn try_recv(&mut self) -> engine_runtime::Result<Option<AudioFrame>> {
        if !self.running || self.index >= self.frames.len() {
            if self.index >= self.frames.len() {
                self.completed.store(true, Ordering::Release);
            }
            return Ok(None);
        }
        let due = Duration::from_millis(self.index as u64 * FRAME_MILLIS);
        if self.started.is_some_and(|started| started.elapsed() >= due) {
            let frame = self.frames[self.index].clone();
            self.index += 1;
            if self.index == self.frames.len() {
                self.completed.store(true, Ordering::Release);
            }
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }
    fn is_running(&self) -> bool {
        self.running
    }
    fn metrics(&self) -> CaptureMetrics {
        CaptureMetrics::default()
    }
}

struct Accuracy {
    wer: f64,
    cer: f64,
}

fn score(reference: &str, hypothesis: &str) -> Accuracy {
    let reference = normalized(reference);
    let hypothesis = normalized(hypothesis);
    let reference_words = reference.split_whitespace().collect::<Vec<_>>();
    let hypothesis_words = hypothesis.split_whitespace().collect::<Vec<_>>();
    let reference_chars = reference
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<Vec<_>>();
    let hypothesis_chars = hypothesis
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<Vec<_>>();
    Accuracy {
        wer: edit_distance(&reference_words, &hypothesis_words) as f64
            / reference_words.len().max(1) as f64,
        cer: edit_distance(&reference_chars, &hypothesis_chars) as f64
            / reference_chars.len().max(1) as f64,
    }
}

fn normalized(text: &str) -> String {
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
    output.trim().to_owned()
}

fn edit_distance<T: Eq>(reference: &[T], hypothesis: &[T]) -> usize {
    let mut previous = (0..=hypothesis.len()).collect::<Vec<_>>();
    for (row, reference_item) in reference.iter().enumerate() {
        let mut current = vec![row + 1; hypothesis.len() + 1];
        for (column, hypothesis_item) in hypothesis.iter().enumerate() {
            current[column + 1] = if reference_item == hypothesis_item {
                previous[column]
            } else {
                1 + previous[column]
                    .min(previous[column + 1])
                    .min(current[column])
            };
        }
        previous = current;
    }
    previous[hypothesis.len()]
}

fn format_elapsed(value: Option<Duration>) -> String {
    value.map_or_else(
        || "not observed".into(),
        |elapsed| format!("{:.3}s", elapsed.as_secs_f64()),
    )
}
