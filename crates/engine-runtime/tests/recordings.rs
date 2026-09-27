use engine_protocol::AudioSource;
use engine_runtime::{EngineConfig, EngineRuntime, ExportFormat, RecordingLibrary, TranscriptLine};
use std::fs;

fn persisted_recording() -> (tempfile::TempDir, EngineRuntime, RecordingLibrary, String) {
    let root = tempfile::tempdir().unwrap();
    let engine = EngineRuntime::new(EngineConfig {
        recordings_directory: root.path().into(),
        ..Default::default()
    })
    .unwrap();
    let id = "123e4567-e89b-12d3-a456-426614174000".to_owned();
    let directory = root.path().join(&id);
    fs::create_dir(&directory).unwrap();
    let metadata = serde_json::json!({
        "schema_version": 1,
        "snapshot": {"session":{"id":id,"started_at_unix_ms":1234},"elapsed_ms":5600},
        "recordings":[{"source":"microphone","file":"microphone.wav"}]
    });
    fs::write(
        directory.join("session.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let transcript = vec![TranscriptLine {
        source: AudioSource::Microphone,
        start_ms: 10,
        end_ms: 40,
        text: "hello".into(),
    }];
    fs::write(
        directory.join("transcript.json"),
        serde_json::to_vec(&transcript).unwrap(),
    )
    .unwrap();
    let library = RecordingLibrary::new(root.path());
    (root, engine, library, id)
}

#[test]
fn shared_recording_library_lists_renames_and_exports_persisted_sessions() {
    let (_root, engine, library, id) = persisted_recording();
    let listed = library.list(&engine).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].session_id, id);
    let detail = library.get(&engine, &id).unwrap();
    assert_eq!(detail.transcript[0].text, "hello");

    library.rename(&engine, &id, "Team sync").unwrap();
    assert_eq!(
        library.get(&engine, &id).unwrap().summary.title,
        "Team sync"
    );
    assert_eq!(
        library.export(&engine, &id, ExportFormat::Txt).unwrap(),
        "[10–40 ms] Microphone: hello\n".as_bytes()
    );
    assert_eq!(
        library
            .export(&engine, &id, ExportFormat::PlainText)
            .unwrap(),
        b"hello"
    );
    let json: Vec<TranscriptLine> =
        serde_json::from_slice(&library.export(&engine, &id, ExportFormat::Json).unwrap()).unwrap();
    assert_eq!(json, detail.transcript);
    engine.shutdown().unwrap();
}

#[test]
fn shared_recording_library_rejects_path_traversal_and_deletes_completed_sessions() {
    let (_root, engine, library, id) = persisted_recording();
    assert!(library.get(&engine, "../../outside").is_err());
    library.delete(&engine, &id).unwrap();
    assert!(library.list(&engine).unwrap().is_empty());
    engine.shutdown().unwrap();
}
