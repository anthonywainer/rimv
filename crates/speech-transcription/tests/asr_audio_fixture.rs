use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use engine_protocol::AudioSource;
use speech_transcription::{
    AsrBackendKind, SpeechConfig, SpeechEvent, SpeechWorker, load_configured_backend,
};
use std::{path::PathBuf, sync::mpsc};

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
    let samples = reader
        .samples::<i16>()
        .map(|sample| sample.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let frame = AudioFrame::new(
        AudioSourceKind::Microphone,
        std::time::Duration::ZERO,
        AudioFormat::new(spec.sample_rate, spec.channels).unwrap(),
        samples,
    )
    .unwrap();

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
    worker
        .send_captured(AudioSource::Microphone, frame)
        .unwrap();
    worker.flush_source(AudioSource::Microphone).unwrap();
    worker.shutdown();

    let transcript = receiver
        .try_iter()
        .filter_map(|event| match event {
            SpeechEvent::Update(update) if update.is_final => Some(update.stable_text),
            SpeechEvent::Final(segment) => Some(segment.text),
            _ => None,
        })
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
