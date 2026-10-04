use crate::{AsrBackendKind, MockAsrBackend, SpeechSegment, SpeechToTextEngine, supports_backend};
#[cfg(feature = "parakeet")]
use crate::{SpeechConfig, load_configured_backend};
use engine_protocol::AudioSource;

#[test]
fn backend_capabilities_match_compiled_features() {
    assert_eq!(
        supports_backend(AsrBackendKind::Parakeet),
        cfg!(feature = "parakeet")
    );
    assert_eq!(
        supports_backend(AsrBackendKind::Whisper),
        cfg!(feature = "whisper")
    );
    assert_eq!(
        supports_backend(AsrBackendKind::Enhanced),
        cfg!(all(feature = "parakeet", feature = "whisper"))
    );
    assert_eq!(
        supports_backend(AsrBackendKind::AppleNative),
        cfg!(all(target_os = "macos", feature = "apple-speech"))
    );
}

#[test]
fn mock_backend_is_deterministic() {
    let mut backend = MockAsrBackend::default();
    backend.responses.push_back(Ok(vec![SpeechSegment {
        source: AudioSource::Microphone,
        start_ms: 0,
        end_ms: 1,
        text: "ok".into(),
    }]));
    assert_eq!(backend.transcribe(&[], 0).unwrap()[0].text, "ok");
    assert!(backend.transcribe(&[], 0).unwrap().is_empty());
    assert_eq!(backend.info().backend_id, "mock");
}

#[cfg(feature = "parakeet")]
#[test]
fn configured_parakeet_reports_a_missing_model_without_native_loading() {
    let error = match load_configured_backend(SpeechConfig {
        backend: AsrBackendKind::Parakeet,
        model_path: None,
        ..Default::default()
    }) {
        Ok(_) => panic!("missing Parakeet model unexpectedly loaded"),
        Err(error) => error,
    };
    assert!(matches!(error, crate::SpeechError::ModelMissing));
}
