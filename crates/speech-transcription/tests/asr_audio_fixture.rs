use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use engine_protocol::AudioSource;
use speech_transcription::{
    AsrBackendKind, SpeechConfig, SpeechEvent, SpeechWorker, load_configured_backend,
};
use std::{path::PathBuf, sync::mpsc, thread, time::Duration};

/// Run explicitly with the matching Parakeet and Silero assets installed.
/// Fixture: 16 kHz mono PCM WAV, 34.99 s, English travel conversation;
/// transcript reference is resources/audio/audio1.txt.
#[test]
#[ignore = "requires local Parakeet and Silero model assets"]
fn prerecorded_english_speech_produces_final_transcript_events() {
    let model = required_path("RIMV_PARAKEET_MODEL_DIR");
    let vad = required_path("RIMV_SILERO_VAD_MODEL");
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/audio/audio1-16k-mono.wav");
    let mut reader = hound::WavReader::open(fixture).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(spec.channels, 1);
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|sample| sample.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let format = AudioFormat::new(spec.sample_rate, spec.channels).unwrap();

    let config = SpeechConfig {
        backend: AsrBackendKind::Parakeet,
        model_path: Some(model),
        use_gpu: false,
        vad: speech_transcription::VadConfig {
            model_path: Some(vad),
            ..Default::default()
        },
        ..Default::default()
    };
    let loader_config = config.clone();
    let (events, receiver) = mpsc::sync_channel(256);
    let mut worker = SpeechWorker::start_with_loader(config, events, move || {
        load_configured_backend(loader_config)
    })
    .unwrap();
    // Feed capture-sized blocks at real-time cadence. Submitting the whole WAV
    // as one frame lets the VAD finalize the utterance before the decoder has a
    // chance to publish any live hypotheses, which does not exercise capture.
    for (index, chunk) in samples.chunks(1_024).enumerate() {
        let frame = AudioFrame::new(
            AudioSourceKind::Microphone,
            Duration::from_millis(index as u64 * 64),
            format,
            chunk.to_vec(),
        )
        .unwrap();
        worker
            .send_captured(AudioSource::Microphone, frame)
            .unwrap();
        thread::sleep(Duration::from_millis(64));
    }
    worker.flush_source(AudioSource::Microphone).unwrap();
    worker.shutdown();

    let updates = receiver
        .try_iter()
        .filter_map(|event| match event {
            SpeechEvent::Update(update) => Some(update),
            SpeechEvent::Final(segment) => Some(speech_transcription::TranscriptUpdate {
                source: segment.source,
                utterance_id: "legacy-final".into(),
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                stable_text: segment.text,
                unstable_text: String::new(),
                is_final: true,
                language: None,
                confidence: None,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        updates.iter().any(|update| !update.is_final
            && (!update.stable_text.is_empty() || !update.unstable_text.is_empty())),
        "the offline recognizer pipeline should publish incremental live drafts while capture continues"
    );
    assert!(
        updates.iter().any(|update| update.is_final),
        "stopping capture should flush a finalized result"
    );
    for (index, partial) in updates
        .iter()
        .enumerate()
        .filter(|(_, update)| !update.is_final)
    {
        assert!(
            updates[index + 1..]
                .iter()
                .any(|later| { later.is_final && later.utterance_id == partial.utterance_id }),
            "every published partial should be superseded by exactly its utterance's final update"
        );
    }
    let mut finalized_ids = std::collections::HashSet::new();
    assert!(
        updates
            .iter()
            .filter(|update| update.is_final)
            .all(|update| finalized_ids.insert(update.utterance_id.as_str())),
        "a VAD utterance should be finalized only once"
    );
    let transcript = updates
        .iter()
        .filter(|update| update.is_final)
        .map(|update| update.stable_text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        transcript.contains("travel plans"),
        "unexpected transcript: {transcript}"
    );
    assert!(
        transcript.contains("san francisco"),
        "unexpected transcript: {transcript}"
    );
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .unwrap_or_else(|| panic!("set {name} to an installed model asset"))
}
