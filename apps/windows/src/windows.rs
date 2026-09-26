use engine_protocol::{
    AudioSource, EngineCommand, EngineEvent, EngineSnapshot, EngineStatus, TranscriptUpdate,
};
use engine_runtime::{AsrBackendKind, EngineConfig, EngineRuntime};
use model_manager::{ModelDescriptor, ModelManager, ModelState};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};
use tauri::{
    Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

const MAIN_WINDOW: &str = "main";
const CAPTURE_SOURCE_FILE: &str = "capture-source.json";
const SELECTED_MODEL_FILE: &str = "selected-model";
const SELECTED_LANGUAGE_FILE: &str = "selected-language";
const WINDOWS_BACKEND: &str = "parakeet";
const SESSION_TITLE_FILE: &str = "rimv-session.json";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CaptureSource {
    #[default]
    Microphone,
    System,
    Both,
}

impl CaptureSource {
    fn enabled(self) -> (bool, bool) {
        match self {
            Self::Microphone => (true, false),
            Self::System => (false, true),
            Self::Both => (true, true),
        }
    }
}

struct DesktopState {
    engine: EngineRuntime,
    data_root: PathBuf,
    models: ModelManager,
    command_gate: Arc<Mutex<()>>,
    selected_model: Arc<Mutex<Option<String>>>,
    selected_language: Arc<Mutex<Option<String>>>,
    model_operations: Arc<Mutex<std::collections::HashMap<String, ModelOperation>>>,
    model_workers: Mutex<Vec<JoinHandle<()>>>,
    live_transcript: Arc<Mutex<LiveTranscript>>,
    _instance_lock: File,
    event_thread: Mutex<Option<JoinHandle<()>>>,
    shutting_down: AtomicBool,
}

#[derive(Default)]
struct LiveTranscript {
    session_id: Option<String>,
    updates: HashMap<(String, String), TranscriptUpdate>,
}

#[derive(Debug, Serialize)]
struct RecordingSummary {
    session_id: String,
    title: String,
    started_at_unix_ms: u64,
    duration_ms: u64,
    state: String,
    sources: Vec<String>,
    has_transcript: bool,
}

#[derive(Debug, Serialize)]
struct RecordingDetails {
    summary: RecordingSummary,
    transcript: Vec<TranscriptLine>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TranscriptLine {
    source: AudioSource,
    start_ms: u64,
    end_ms: u64,
    text: String,
}

#[derive(Debug, Serialize)]
struct LiveTranscriptView {
    session_id: Option<String>,
    updates: Vec<TranscriptUpdate>,
}

struct ModelOperation {
    cancelled: Arc<AtomicBool>,
    phase: String,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    active: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelSummary {
    id: String,
    name: String,
    version: String,
    selected_language: Option<String>,
    state: String,
    selected: bool,
    languages: Vec<String>,
    supports_language_detection: bool,
    install_hint: Option<String>,
    progress_bytes: Option<u64>,
    progress_total_bytes: Option<u64>,
    error: Option<String>,
}

#[derive(Clone, Serialize)]
struct ModelProgress {
    model_id: String,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    phase: String,
    error: Option<String>,
}

impl DesktopState {
    fn shutdown(&self) {
        if self.shutting_down.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut workers = self
            .model_workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        {
            let operations = self
                .model_operations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for operation in operations.values().filter(|operation| operation.active) {
                operation.cancelled.store(true, Ordering::Release);
            }
        }
        for worker in workers.drain(..) {
            if worker.join().is_err() {
                tracing::error!("model install worker panicked");
            }
        }
        if let Err(error) = self.engine.shutdown() {
            tracing::error!(%error, "engine shutdown failed");
        }
        if let Some(thread) = self
            .event_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            if thread.join().is_err() {
                tracing::error!("engine event publisher panicked");
            }
        }
    }
}

fn supported_models(models: &ModelManager) -> impl Iterator<Item = &ModelDescriptor> {
    models
        .catalog()
        .iter()
        .filter(|model| model.backend == WINDOWS_BACKEND)
}

fn model_summaries(state: &DesktopState) -> Vec<ModelSummary> {
    let selected = state
        .selected_model
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let selected_language = state
        .selected_language
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let operations = state
        .model_operations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    supported_models(&state.models)
        .map(|model| {
            let operation = operations.get(&model.id);
            let model_state = state.models.state(model);
            let model_state = match (model_state, operation) {
                (ModelState::Ready, _) => "installed",
                (_, Some(operation)) if operation.active && operation.phase == "extracting" => {
                    "extracting"
                }
                (_, Some(operation)) if operation.active => "downloading",
                (_, Some(operation)) if operation.error.is_some() => "failed",
                (ModelState::Incomplete, _) => "incomplete",
                (ModelState::Missing, _) => "available",
            };
            ModelSummary {
                id: model.id.clone(),
                name: model.display_name.clone(),
                version: model.version.clone(),
                selected_language: (selected.as_deref() == Some(model.id.as_str()))
                    .then(|| selected_language.clone())
                    .flatten(),
                state: model_state.to_owned(),
                selected: selected.as_deref() == Some(model.id.as_str()),
                languages: model.languages.clone(),
                supports_language_detection: model.capabilities.supports_language_detection,
                install_hint: model.install_hint.clone(),
                progress_bytes: operation.map(|op| op.downloaded_bytes),
                progress_total_bytes: operation.and_then(|op| op.total_bytes),
                error: operation.and_then(|op| op.error.clone()),
            }
        })
        .collect()
}

fn persist_preference(root: &Path, file: &str, value: &str) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    fs::write(root.join(file), value).map_err(|error| error.to_string())
}

fn saved_language(root: &Path, model: &ModelDescriptor) -> Option<String> {
    let value = fs::read_to_string(root.join(SELECTED_LANGUAGE_FILE)).ok()?;
    let value = value.trim();
    model
        .languages
        .iter()
        .any(|language| language == value)
        .then(|| value.to_owned())
}

fn is_session_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn recordings_root(root: &Path) -> PathBuf {
    root.join("recordings")
}

fn recording_directory(root: &Path, session_id: &str) -> Result<PathBuf, String> {
    if !is_session_id(session_id) {
        return Err("Invalid recording identifier.".into());
    }
    let canonical_root = recordings_root(root)
        .canonicalize()
        .map_err(|_| "Recordings folder is unavailable.".to_owned())?;
    let directory = canonical_root
        .join(session_id)
        .canonicalize()
        .map_err(|_| "Recording could not be found.".to_owned())?;
    if directory.parent() != Some(canonical_root.as_path()) || !directory.is_dir() {
        return Err("Recording is outside the RimV recordings folder.".into());
    }
    Ok(directory)
}

fn active_session_matches(snapshot: &EngineSnapshot, session_id: &str) -> bool {
    (matches!(
        snapshot.status,
        EngineStatus::Starting | EngineStatus::Recording | EngineStatus::Stopping
    ) || snapshot.microphone.active
        || snapshot.system_audio.active)
        && snapshot
            .session
            .as_ref()
            .is_some_and(|session| session.id.0 == session_id)
}

fn metadata_json(directory: &Path) -> Result<serde_json::Value, String> {
    let bytes = fs::read(directory.join("session.json"))
        .map_err(|_| "Recording metadata is not ready yet.".to_owned())?;
    serde_json::from_slice(&bytes).map_err(|_| "Recording metadata is invalid.".to_owned())
}

fn validate_persisted_identity(
    directory: &Path,
    expected_id: &str,
) -> Result<serde_json::Value, String> {
    let metadata = metadata_json(directory)?;
    let session = metadata
        .pointer("/snapshot/session")
        .ok_or_else(|| "Recording metadata is incomplete.".to_owned())?;
    if session.get("id").and_then(serde_json::Value::as_str) != Some(expected_id)
        || directory.file_name().and_then(|name| name.to_str()) != Some(expected_id)
    {
        return Err("Recording identity does not match its folder.".into());
    }
    Ok(metadata)
}

fn recording_title(directory: &Path, session_id: &str) -> Option<String> {
    let sidecar: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join(SESSION_TITLE_FILE)).ok()?).ok()?;
    (sidecar
        .get("session_id")
        .and_then(serde_json::Value::as_str)
        == Some(session_id))
    .then(|| {
        sidecar
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })
    .flatten()
}

fn recording_summary(
    directory: &Path,
    session_id: &str,
    metadata: &serde_json::Value,
    state: &str,
) -> RecordingSummary {
    let session = metadata.pointer("/snapshot/session");
    let started_at_unix_ms = session
        .and_then(|value| value.get("started_at_unix_ms"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    let duration_ms = metadata
        .pointer("/snapshot/elapsed_ms")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    let mut sources = metadata
        .get("recordings")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|recording| recording.get("source").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if state == "recording" {
        for source in ["microphone", "system"] {
            if directory.join(format!("{source}.wav")).is_file()
                && !sources.iter().any(|s| s == source)
            {
                sources.push(source.to_owned());
            }
        }
    }
    RecordingSummary {
        session_id: session_id.to_owned(),
        title: recording_title(directory, session_id).unwrap_or_else(|| "Recording".into()),
        started_at_unix_ms,
        duration_ms,
        state: state.to_owned(),
        sources,
        has_transcript: directory.join("transcript.json").is_file(),
    }
}

fn discover_recordings(state: &DesktopState) -> Result<Vec<RecordingSummary>, String> {
    let root = recordings_root(&state.data_root);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let active = state.engine.snapshot();
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if (matches!(
        active.status,
        EngineStatus::Starting | EngineStatus::Recording | EngineStatus::Stopping
    ) || active.microphone.active
        || active.system_audio.active)
        && let Some(session) = active.session.as_ref()
        && let Ok(directory) = recording_directory(&state.data_root, &session.id.0)
        && Path::new(&session.recording_directory)
            .canonicalize()
            .ok()
            .as_ref()
            == Some(&directory)
    {
        let metadata = serde_json::json!({
            "snapshot": { "session": { "id": session.id.0, "started_at_unix_ms": session.started_at_unix_ms }, "elapsed_ms": active.elapsed_ms },
            "recordings": []
        });
        items.push(recording_summary(
            &directory,
            &session.id.0,
            &metadata,
            "recording",
        ));
        seen.insert(session.id.0.clone());
    }
    let entries = fs::read_dir(&root).map_err(|error| error.to_string())?;
    for entry in entries.flatten() {
        let Some(session_id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !is_session_id(&session_id) || seen.contains(&session_id) {
            continue;
        }
        let Ok(directory) = recording_directory(&state.data_root, &session_id) else {
            continue;
        };
        let Ok(metadata) = validate_persisted_identity(&directory, &session_id) else {
            continue;
        };
        items.push(recording_summary(
            &directory,
            &session_id,
            &metadata,
            "completed",
        ));
    }
    items.sort_by(|a, b| b.started_at_unix_ms.cmp(&a.started_at_unix_ms));
    Ok(items)
}

fn save_recording_title(
    directory: &Path,
    session_id: &str,
    started_at_unix_ms: u64,
    title: &str,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "session_id": session_id,
        "started_at_unix_ms": started_at_unix_ms,
        "title": title,
    }))
    .map_err(|error| error.to_string())?;
    let destination = directory.join(SESSION_TITLE_FILE);
    let temporary = directory.join(format!("{SESSION_TITLE_FILE}.{}.tmp", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    let backup = directory.join(format!("{SESSION_TITLE_FILE}.bak"));
    let had_destination = destination.exists();
    if had_destination {
        let _ = fs::remove_file(&backup);
        fs::rename(&destination, &backup).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            error.to_string()
        })?;
    }
    if let Err(error) = fs::rename(&temporary, &destination) {
        if had_destination {
            let _ = fs::rename(&backup, &destination);
        }
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    if had_destination {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

#[tauri::command]
async fn get_state(state: State<'_, DesktopState>) -> Result<EngineSnapshot, String> {
    Ok(state.engine.snapshot())
}

#[tauri::command]
fn get_model_catalog(state: State<'_, DesktopState>) -> Vec<ModelSummary> {
    model_summaries(&state)
}

#[tauri::command]
fn get_recordings(state: State<'_, DesktopState>) -> Result<Vec<RecordingSummary>, String> {
    discover_recordings(&state)
}

#[tauri::command]
async fn get_recording(
    session_id: String,
    state: State<'_, DesktopState>,
) -> Result<RecordingDetails, String> {
    let root = state.data_root.clone();
    let engine = state.engine.clone();
    let live_transcript = state.live_transcript.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let proxy = RecordingState {
            root,
            engine,
            live_transcript,
        };
        recording_details_from(&proxy, &session_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

// Narrow, owned data needed by disk readers; no frontend-provided path crosses the boundary.
struct RecordingState {
    root: PathBuf,
    engine: EngineRuntime,
    live_transcript: Arc<Mutex<LiveTranscript>>,
}

fn recording_details_from(
    state: &RecordingState,
    session_id: &str,
) -> Result<RecordingDetails, String> {
    let directory = recording_directory(&state.root, session_id)?;
    let active = state.engine.snapshot();
    let is_active = active_session_matches(&active, session_id);
    let metadata = if is_active {
        let session = active.session.as_ref().expect("checked active session");
        serde_json::json!({
            "snapshot": { "session": { "id": session.id.0, "started_at_unix_ms": session.started_at_unix_ms }, "elapsed_ms": active.elapsed_ms },
            "recordings": []
        })
    } else {
        validate_persisted_identity(&directory, session_id)?
    };
    let mut transcript = Vec::new();
    if is_active {
        let cache = state
            .live_transcript
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cache.session_id.as_deref() == Some(session_id) {
            transcript.extend(
                cache
                    .updates
                    .values()
                    .map(|update| TranscriptLine {
                        source: update.source,
                        start_ms: update.start_ms,
                        end_ms: update.end_ms,
                        text: [&update.stable_text[..], &update.unstable_text[..]]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" "),
                    })
                    .filter(|line| !line.text.trim().is_empty()),
            );
        }
    } else if let Ok(bytes) = fs::read(directory.join("transcript.json")) {
        transcript = serde_json::from_slice(&bytes)
            .map_err(|_| "Saved transcript is invalid.".to_owned())?;
    }
    transcript.sort_by_key(|line| line.start_ms);
    let mut summary = recording_summary(
        &directory,
        session_id,
        &metadata,
        if is_active { "recording" } else { "completed" },
    );
    summary.has_transcript = !transcript.is_empty();
    Ok(RecordingDetails {
        summary,
        transcript,
    })
}

#[tauri::command]
async fn rename_recording(
    session_id: String,
    title: String,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    let root = state.data_root.clone();
    let engine = state.engine.clone();
    let gate = state.command_gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 120 || title.chars().any(char::is_control) {
            return Err("Recording names must contain 1–120 visible characters.".into());
        }
        let directory = recording_directory(&root, &session_id)?;
        let snapshot = engine.snapshot();
        let is_active = active_session_matches(&snapshot, &session_id);
        let started_at = if is_active {
            snapshot
                .session
                .as_ref()
                .expect("checked active session")
                .started_at_unix_ms
        } else {
            validate_persisted_identity(&directory, &session_id)?
                .pointer("/snapshot/session/started_at_unix_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "Recording start time is missing.".to_owned())?
        };
        save_recording_title(&directory, &session_id, started_at, title)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn delete_recording(
    session_id: String,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    let root = state.data_root.clone();
    let engine = state.engine.clone();
    let gate = state.command_gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = recording_directory(&root, &session_id)?;
        let snapshot = engine.snapshot();
        if active_session_matches(&snapshot, &session_id) {
            return Err("An active recording cannot be deleted.".into());
        }
        validate_persisted_identity(&directory, &session_id)?;
        fs::remove_dir_all(directory)
            .map_err(|error| format!("Could not delete recording: {error}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn get_live_transcript(state: State<'_, DesktopState>) -> LiveTranscriptView {
    let cache = state
        .live_transcript
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    LiveTranscriptView {
        session_id: cache.session_id.clone(),
        updates: cache.updates.values().cloned().collect(),
    }
}

fn audio_response(
    root: &Path,
    engine: &EngineRuntime,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode, header};
    const MAX_AUDIO_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;
    let fail = |status| {
        Response::builder()
            .status(status)
            .body(Vec::new())
            .expect("valid response")
    };
    let pieces = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if pieces.len() != 2 || !matches!(pieces[1], "microphone" | "system") {
        return fail(StatusCode::NOT_FOUND);
    }
    let session_id = pieces[0];
    let Ok(directory) = recording_directory(root, session_id) else {
        return fail(StatusCode::NOT_FOUND);
    };
    let snapshot = engine.snapshot();
    let active = active_session_matches(&snapshot, session_id);
    if !active && validate_persisted_identity(&directory, session_id).is_err() {
        return fail(StatusCode::NOT_FOUND);
    }
    let filename = format!("{}.wav", pieces[1]);
    let path = directory.join(&filename);
    let Ok(canonical_file) = path.canonicalize() else {
        return fail(StatusCode::NOT_FOUND);
    };
    if canonical_file.parent() != Some(directory.as_path()) {
        return fail(StatusCode::NOT_FOUND);
    }
    let Ok(mut file) = File::open(canonical_file) else {
        return fail(StatusCode::NOT_FOUND);
    };
    let Ok(total) = file.metadata().map(|metadata| metadata.len()) else {
        return fail(StatusCode::NOT_FOUND);
    };
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let (start, requested_end, partial) = if let Some(value) = range {
        let Some(spec) = value.strip_prefix("bytes=") else {
            return fail(StatusCode::RANGE_NOT_SATISFIABLE);
        };
        if spec.contains(',') {
            return fail(StatusCode::RANGE_NOT_SATISFIABLE);
        }
        let Some((left, right)) = spec.split_once('-') else {
            return fail(StatusCode::RANGE_NOT_SATISFIABLE);
        };
        let parsed = if left.is_empty() {
            right
                .parse::<u64>()
                .ok()
                .filter(|suffix| *suffix > 0)
                .map(|suffix| (total.saturating_sub(suffix), total.saturating_sub(1)))
        } else {
            left.parse::<u64>().ok().and_then(|start| {
                if start >= total {
                    None
                } else {
                    let end = if right.is_empty() {
                        total - 1
                    } else {
                        right.parse::<u64>().ok()?.min(total - 1)
                    };
                    (start <= end).then_some((start, end))
                }
            })
        };
        let Some((start, end)) = parsed else {
            return fail(StatusCode::RANGE_NOT_SATISFIABLE);
        };
        (start, end, true)
    } else if total > 0 {
        if total > MAX_AUDIO_RESPONSE_BYTES {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .body(Vec::new())
                .expect("valid range response");
        }
        (0, total - 1, false)
    } else {
        return fail(StatusCode::NOT_FOUND);
    };
    let end = requested_end.min(start.saturating_add(MAX_AUDIO_RESPONSE_BYTES - 1));
    let partial = partial || end + 1 < total;
    let length = end - start + 1;
    if file.seek(SeekFrom::Start(start)).is_err() {
        return fail(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut body = vec![0; length.min(usize::MAX as u64) as usize];
    if file.read_exact(&mut body).is_err() {
        return fail(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut response = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, "audio/wav")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, body.len().to_string());
    if partial {
        response = response.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        );
    }
    response.body(body).expect("valid audio response")
}

#[tauri::command]
async fn select_model(
    model_id: String,
    state: State<'_, DesktopState>,
) -> Result<EngineSnapshot, String> {
    let engine = state.engine.clone();
    let models = state.models.clone();
    let root = state.data_root.clone();
    let gate = state.command_gate.clone();
    let selected_model = state.selected_model.clone();
    let selected_language = state.selected_language.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if engine.snapshot().status != EngineStatus::Idle {
            return Err("Stop capture before changing the transcription model.".to_owned());
        }
        let model = models
            .descriptor(&model_id)
            .map_err(|error| error.to_string())?;
        if model.backend != WINDOWS_BACKEND {
            return Err("This model backend is not available in the Windows app.".into());
        }
        if models.state(model) != ModelState::Ready {
            return Err("Install the model before selecting it.".into());
        }
        let model_path = models.directory(model);
        let language = saved_language(&root, model);
        engine
            .send(EngineCommand::SetTranscriptionBackend {
                backend: WINDOWS_BACKEND.into(),
            })
            .map_err(|error| error.to_string())?;
        engine
            .send(EngineCommand::SetTranscriptionModel {
                path: model_path.to_string_lossy().into_owned(),
            })
            .map_err(|error| error.to_string())?;
        engine
            .send(EngineCommand::SetTranscriptionLanguage {
                language: language.clone(),
            })
            .map_err(|error| error.to_string())?;
        let snapshot = engine
            .send(EngineCommand::SetTranscriptionEnabled { enabled: true })
            .map_err(|error| error.to_string())?;
        *selected_model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(model_id.clone());
        *selected_language
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = language;
        persist_preference(&root, SELECTED_MODEL_FILE, &model_id)?;
        Ok(snapshot)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn select_language(
    language: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<EngineSnapshot, String> {
    let engine = state.engine.clone();
    let models = state.models.clone();
    let root = state.data_root.clone();
    let gate = state.command_gate.clone();
    let selected_model = state.selected_model.clone();
    let selected_language = state.selected_language.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if engine.snapshot().status != EngineStatus::Idle {
            return Err("Stop capture before changing the transcription language.".to_owned());
        }
        let selected = selected_model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .ok_or_else(|| "Install and select a transcription model first.".to_owned())?;
        let model = models
            .descriptor(&selected)
            .map_err(|error| error.to_string())?;
        if language
            .as_ref()
            .is_some_and(|code| !model.languages.iter().any(|supported| supported == code))
        {
            return Err("The selected model does not support that language.".into());
        }
        let snapshot = engine
            .send(EngineCommand::SetTranscriptionLanguage {
                language: language.clone(),
            })
            .map_err(|error| error.to_string())?;
        let persisted_language = language.as_deref().unwrap_or("auto").to_owned();
        *selected_language
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = language;
        persist_preference(&root, SELECTED_LANGUAGE_FILE, &persisted_language)?;
        Ok(snapshot)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn clear_selected_model(state: State<'_, DesktopState>) -> Result<EngineSnapshot, String> {
    let engine = state.engine.clone();
    let root = state.data_root.clone();
    let gate = state.command_gate.clone();
    let selected_model = state.selected_model.clone();
    let selected_language = state.selected_language.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if engine.snapshot().status != EngineStatus::Idle {
            return Err("Stop capture before changing the transcription model.".to_owned());
        }
        engine
            .send(EngineCommand::ClearTranscriptionModel)
            .map_err(|error| error.to_string())?;
        engine
            .send(EngineCommand::SetTranscriptionLanguage { language: None })
            .map_err(|error| error.to_string())?;
        *selected_model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        *selected_language
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        persist_preference(&root, SELECTED_MODEL_FILE, "")?;
        persist_preference(&root, SELECTED_LANGUAGE_FILE, "auto")?;
        Ok(engine.snapshot())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn install_model(
    model_id: String,
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    let _gate = state
        .command_gate
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let model = state
        .models
        .descriptor(&model_id)
        .map_err(|error| error.to_string())?;
    if model.backend != WINDOWS_BACKEND {
        return Err("This model backend is not available in the Windows app.".into());
    }
    if state.models.state(model) == ModelState::Ready {
        return Ok(());
    }
    let mut workers = state
        .model_workers
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.shutting_down.load(Ordering::Acquire) {
        return Err("RimV is shutting down.".into());
    }
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            let finished = workers.swap_remove(index);
            let _ = finished.join();
        } else {
            index += 1;
        }
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut operations = state
            .model_operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if operations
            .get(&model_id)
            .is_some_and(|operation| operation.active)
        {
            return Err("This model is already downloading.".into());
        }
        operations.insert(
            model_id.clone(),
            ModelOperation {
                cancelled: cancelled.clone(),
                phase: "downloading".into(),
                downloaded_bytes: 0,
                total_bytes: None,
                active: true,
                error: None,
            },
        );
    }
    let manager = state.models.clone();
    let operations = state.model_operations.clone();
    let worker_id = model_id.clone();
    let worker = thread::Builder::new()
        .name("rimv-model-install".into())
        .spawn(move || {
            let mut last_reported = 0_u64;
            let result = manager.install_parakeet_with_phase(
                &cancelled,
                |phase| {
                    let downloaded = if let Some(operation) = operations
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get_mut(&worker_id)
                    {
                        operation.phase = phase.into();
                        operation.downloaded_bytes
                    } else {
                        0
                    };
                    let _ = app.emit(
                        "model-progress",
                        ModelProgress {
                            model_id: worker_id.clone(),
                            downloaded_bytes: downloaded,
                            total_bytes: operations
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .get(&worker_id)
                                .and_then(|operation| operation.total_bytes),
                            phase: phase.into(),
                            error: None,
                        },
                    );
                },
                |downloaded, total| {
                    if let Some(operation) = operations
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get_mut(&worker_id)
                    {
                        operation.downloaded_bytes = downloaded;
                        operation.total_bytes = total;
                    }
                    if downloaded.saturating_sub(last_reported) >= 1024 * 1024 {
                        last_reported = downloaded;
                        let _ = app.emit(
                            "model-progress",
                            ModelProgress {
                                model_id: worker_id.clone(),
                                downloaded_bytes: downloaded,
                                total_bytes: total,
                                phase: "downloading".into(),
                                error: None,
                            },
                        );
                    }
                },
            );
            let (phase, error) = match result {
                Ok(_) => ("complete", None),
                Err(model_manager::ModelError::Cancelled) => ("cancelled", None),
                Err(error) => ("failed", Some(error.to_string())),
            };
            let (downloaded_bytes, total_bytes) = if let Some(operation) = operations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_mut(&worker_id)
            {
                operation.active = false;
                operation.phase = phase.into();
                operation.error = error.clone();
                (operation.downloaded_bytes, operation.total_bytes)
            } else {
                (last_reported, None)
            };
            let _ = app.emit(
                "model-progress",
                ModelProgress {
                    model_id: worker_id,
                    downloaded_bytes,
                    total_bytes,
                    phase: phase.into(),
                    error,
                },
            );
        })
        .map_err(|error| {
            state
                .model_operations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&model_id);
            error.to_string()
        })?;
    workers.push(worker);
    Ok(())
}

#[tauri::command]
fn cancel_model_install(model_id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let operations = state
        .model_operations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let operation = operations
        .get(&model_id)
        .filter(|operation| operation.active)
        .ok_or_else(|| "This model is not currently downloading.".to_owned())?;
    operation.cancelled.store(true, Ordering::Release);
    Ok(())
}

#[tauri::command]
async fn remove_model(model_id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let engine = state.engine.clone();
    let models = state.models.clone();
    let gate = state.command_gate.clone();
    let selected = state.selected_model.clone();
    let operations = state.model_operations.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if engine.snapshot().status != EngineStatus::Idle {
            return Err("Stop capture before removing a model.".to_owned());
        }
        if selected
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_deref()
            == Some(model_id.as_str())
        {
            return Err("Select another model before removing this one.".into());
        }
        if operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&model_id)
            .is_some_and(|operation| operation.active)
        {
            return Err("Cancel the model download before removing it.".into());
        }
        let descriptor = models
            .descriptor(&model_id)
            .map_err(|error| error.to_string())?;
        if descriptor.backend != WINDOWS_BACKEND {
            return Err("This model backend is not available in the Windows app.".into());
        }
        models.remove(&model_id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn set_capture_source(
    source: CaptureSource,
    state: State<'_, DesktopState>,
) -> Result<EngineSnapshot, String> {
    let engine = state.engine.clone();
    let data_root = state.data_root.clone();
    let command_gate = state.command_gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = DesktopCommandState {
            engine,
            data_root,
            command_gate,
        };
        state.set_capture_source(source)
    })
    .await
    .map_err(|error| error.to_string())?
}

// This owned subset lets blocking commands run off the webview thread.
struct DesktopCommandState {
    engine: EngineRuntime,
    data_root: PathBuf,
    command_gate: Arc<Mutex<()>>,
}

impl DesktopCommandState {
    fn set_capture_source(&self, source: CaptureSource) -> Result<EngineSnapshot, String> {
        let _guard = self
            .command_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (microphone, system) = source.enabled();
        self.engine
            .send(EngineCommand::SetMicrophoneEnabled {
                enabled: microphone,
            })
            .map_err(|error| error.to_string())?;
        let snapshot = self
            .engine
            .send(EngineCommand::SetSystemAudioEnabled { enabled: system })
            .map_err(|error| error.to_string())?;
        let bytes = serde_json::to_vec(&source).map_err(|error| error.to_string())?;
        fs::write(self.data_root.join(CAPTURE_SOURCE_FILE), bytes)
            .map_err(|error| error.to_string())?;
        Ok(snapshot)
    }
}

#[tauri::command]
async fn set_listening(
    enabled: bool,
    state: State<'_, DesktopState>,
) -> Result<EngineSnapshot, String> {
    let engine = state.engine.clone();
    let command_gate = state.command_gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = command_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if enabled {
            engine.start_capture()
        } else {
            engine.stop_capture()
        }
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

fn app_data_root() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    app_paths::data_root(&home)
}

fn load_capture_source(root: &std::path::Path) -> CaptureSource {
    fs::read(root.join(CAPTURE_SOURCE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn instance_lock(root: &std::path::Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("rimv.lock"))
        .map_err(|error| error.to_string())?;
    file.try_lock()
        .map_err(|_| "RimV is already running. Open it from the notification area.".to_owned())?;
    Ok(file)
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open RimV").build(app)?;
    let listening = MenuItemBuilder::with_id("listening", "Start / Stop Listening").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit RimV").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &listening, &separator, &quit])
        .build()?;

    TrayIconBuilder::with_id("rimv-tray")
        .icon(tray_icon())
        .menu(&menu)
        .tooltip("RimV — local transcription")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "listening" => {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let Some(state) = app.try_state::<DesktopState>() else {
                        return;
                    };
                    let _guard = state
                        .command_gate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let result = if state.engine.snapshot().status == EngineStatus::Recording {
                        state.engine.stop_capture()
                    } else {
                        state.engine.start_capture()
                    };
                    if let Err(error) = result {
                        let _ = app.emit("desktop-error", error.to_string());
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn tray_icon() -> tauri::image::Image<'static> {
    const SIDE: usize = 32;
    let mut pixels = vec![0; SIDE * SIDE * 4];
    let wave = [4usize, 7, 11, 8, 5, 9, 13, 8];
    for y in 0..SIDE {
        for x in 0..SIDE {
            let index = (y * SIDE + x) * 4;
            pixels[index..index + 4].copy_from_slice(&[109, 77, 230, 255]);
            let margin = x < 2 || x >= SIDE - 2 || y < 2 || y >= SIDE - 2;
            let bar = (x % 4 <= 1) && y.abs_diff(SIDE / 2) <= wave[(x / 4) % wave.len()];
            if margin {
                pixels[index + 3] = 0;
            } else if bar {
                pixels[index..index + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    tauri::image::Image::new_owned(pixels, SIDE as u32, SIDE as u32)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_runtime(
    root: &Path,
    models: &ModelManager,
) -> Result<(EngineRuntime, Option<String>, Option<String>), String> {
    let source = load_capture_source(root);
    let (microphone, system_audio) = source.enabled();
    let selected = fs::read_to_string(root.join(SELECTED_MODEL_FILE))
        .ok()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .and_then(|id| {
            models
                .descriptor(&id)
                .ok()
                .filter(|model| {
                    model.backend == WINDOWS_BACKEND && models.state(model) == ModelState::Ready
                })
                .map(|model| model.id.clone())
        })
        .or_else(|| {
            supported_models(models)
                .find(|model| models.state(model) == ModelState::Ready)
                .map(|model| model.id.clone())
        });
    let selected_descriptor = selected
        .as_deref()
        .and_then(|id| models.descriptor(id).ok());
    let language = selected_descriptor.and_then(|model| saved_language(root, model));
    let mut config = EngineConfig {
        recordings_directory: root.join("recordings"),
        ..Default::default()
    };
    config.microphone.enabled = microphone;
    config.system_audio.enabled = system_audio;
    config.transcription.backend = AsrBackendKind::Parakeet;
    config.transcription.model_path = selected_descriptor.map(|model| models.directory(model));
    config.transcription.language = language.clone();
    config.transcription_enabled = config.transcription.model_path.is_some();
    let engine = EngineRuntime::new(config).map_err(|error| error.to_string())?;
    Ok((engine, selected, language))
}

pub fn run() -> Result<(), String> {
    let root = app_data_root();
    let lock = instance_lock(&root)?;
    let model_root = std::env::var_os("RIMV_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("models"));
    let models = ModelManager::new(model_root);
    let (engine, selected_model, selected_language) = build_runtime(&root, &models)?;
    let subscription = engine.subscribe().map_err(|error| error.to_string())?;
    let live_transcript = Arc::new(Mutex::new(LiveTranscript::default()));
    let audio_root = root.clone();
    let audio_engine = engine.clone();

    tauri::Builder::default()
        .register_uri_scheme_protocol("rimv-audio", move |_context, request| {
            audio_response(&audio_root, &audio_engine, request)
        })
        .manage(DesktopState {
            engine,
            data_root: root,
            models,
            command_gate: Arc::new(Mutex::new(())),
            selected_model: Arc::new(Mutex::new(selected_model)),
            selected_language: Arc::new(Mutex::new(selected_language)),
            model_operations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            model_workers: Mutex::new(Vec::new()),
            live_transcript: live_transcript.clone(),
            _instance_lock: lock,
            event_thread: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
        })
        .setup(move |app| {
            WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
                .title("RimV")
                .inner_size(390.0, 600.0)
                .min_inner_size(340.0, 500.0)
                .icon(tray_icon())?
                .build()?;
            install_tray(app)?;

            let handle = app.handle().clone();
            let live_transcript = live_transcript.clone();
            let event_thread = thread::Builder::new()
                .name("rimv-ui-events".into())
                .spawn(move || {
                    while let Ok(event) = subscription.recv() {
                        let mut cache = live_transcript
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        match &event {
                            EngineEvent::Snapshot { snapshot } => {
                                let next_id = snapshot
                                    .session
                                    .as_ref()
                                    .map(|session| session.id.0.clone());
                                if next_id != cache.session_id {
                                    cache.session_id = next_id;
                                    cache.updates.clear();
                                }
                            }
                            EngineEvent::TranscriptUpdate { update } => {
                                if cache.session_id.is_some() {
                                    cache.updates.insert(
                                        (
                                            format!("{:?}", update.source),
                                            update.utterance_id.clone(),
                                        ),
                                        update.clone(),
                                    );
                                }
                            }
                            _ => {}
                        }
                        drop(cache);
                        if handle.emit("engine-event", event).is_err() {
                            break;
                        }
                    }
                })
                .map_err(std::io::Error::other)?;
            app.state::<DesktopState>()
                .event_thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .replace(event_thread);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_model_catalog,
            get_recordings,
            get_recording,
            rename_recording,
            delete_recording,
            get_live_transcript,
            select_model,
            select_language,
            clear_selected_model,
            install_model,
            cancel_model_install,
            remove_model,
            set_capture_source,
            set_listening,
            quit_app
        ])
        .on_window_event(|window, event| {
            if window.label() == MAIN_WINDOW
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .map_err(|error| error.to_string())?
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<DesktopState>().shutdown();
            }
        });
    Ok(())
}
