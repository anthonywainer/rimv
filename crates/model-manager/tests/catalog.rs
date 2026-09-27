use model_manager::{ModelManager, ModelState};

#[test]
fn catalog_uses_generic_metadata_and_required_file_validation() {
    let directory = tempfile::tempdir().unwrap();
    let manager = ModelManager::new(directory.path());
    let parakeet = manager.descriptor("parakeet-tdt-0.6b-v3-int8").unwrap();
    assert_eq!(parakeet.backend, "parakeet");
    assert_eq!(manager.state(parakeet), ModelState::Missing);
    let path = manager.path(parakeet, &parakeet.files[0]);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, []).unwrap();
    assert_eq!(manager.state(parakeet), ModelState::Incomplete);
}

#[test]
fn parakeet_v3_catalog_matches_its_multilingual_model_card() {
    let manager = ModelManager::new("/tmp/models");
    let parakeet = manager.descriptor("parakeet-tdt-0.6b-v3-int8").unwrap();
    assert!(parakeet.capabilities.supports_language_detection);
    assert_eq!(parakeet.languages.len(), 25);
    assert!(parakeet.languages.iter().any(|language| language == "uk"));
}

#[test]
fn whisper_catalog_entries_have_verified_artifacts() {
    let manager = ModelManager::new("/tmp/models");
    let tiny = manager.descriptor("whisper-tiny").unwrap();
    assert_eq!(tiny.backend, "whisper");
    assert_eq!(tiny.files[0].filename, "ggml-tiny.bin");
    for id in [
        "whisper-small",
        "whisper-medium",
        "whisper-large",
        "whisper-turbo",
    ] {
        let descriptor = manager.descriptor(id).unwrap();
        assert_eq!(descriptor.backend, "whisper");
        assert!(descriptor.files[0].source_url.is_some());
        assert!(descriptor.files[0].sha256.is_some());
    }
}

#[test]
fn language_validation_uses_catalogued_model_capabilities() {
    let manager = ModelManager::new("/tmp/models");
    let model = manager
        .catalog()
        .iter()
        .find(|model| !model.languages.is_empty())
        .unwrap();
    let supported = model.languages[0].as_str();
    assert!(manager.validate_language(model, Some(supported)).is_ok());
    assert!(matches!(
        manager.validate_language(model, Some("not-a-supported-language")),
        Err(model_manager::ModelError::UnsupportedLanguage(_))
    ));
    assert!(manager.validate_language(model, None).is_ok());
}
