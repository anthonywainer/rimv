use engine_runtime::{
    AsrBackendKind, EngineCommand, EngineConfig, EngineEvent, EngineRuntime, SubscriptionError,
};
use model_manager::{ModelDescriptor, ModelManager, ModelState};
use std::{
    ffi::{CStr, CString, c_char},
    path::PathBuf,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::Duration,
};

#[derive(Debug, Clone, Default)]
struct LiveTranscriptCache {
    session_id: Option<String>,
    revision: u64,
    finalized: Vec<engine_runtime::TranscriptUpdate>,
    partials: Vec<engine_runtime::TranscriptUpdate>,
}

impl LiveTranscriptCache {
    fn observe_snapshot(&mut self, snapshot: &engine_runtime::EngineSnapshot) {
        let Some(session) = &snapshot.session else {
            return;
        };
        let session_id = session.id.to_string();
        if self.session_id.as_deref() != Some(session_id.as_str()) {
            self.session_id = Some(session_id);
            self.finalized.clear();
            self.partials.clear();
            self.revision = self.revision.saturating_add(1);
        }
    }

    fn update(&mut self, update: engine_runtime::TranscriptUpdate) {
        self.revision = self.revision.saturating_add(1);
        if update.is_final {
            self.partials
                .retain(|partial| partial.utterance_id != update.utterance_id);
            if let Some(existing) = self
                .finalized
                .iter_mut()
                .find(|finalized| finalized.utterance_id == update.utterance_id)
            {
                *existing = update;
            } else {
                self.finalized.push(update);
            }
            self.finalized.sort_by(|a, b| {
                a.start_ms
                    .cmp(&b.start_ms)
                    .then_with(|| a.utterance_id.cmp(&b.utterance_id))
            });
        } else if update.stable_text.is_empty() && update.unstable_text.is_empty() {
            self.partials
                .retain(|partial| partial.utterance_id != update.utterance_id);
        } else if let Some(existing) = self
            .partials
            .iter_mut()
            .find(|partial| partial.utterance_id == update.utterance_id)
        {
            *existing = update;
        } else {
            self.partials.push(update);
        }
    }

    fn json(&self) -> String {
        serde_json::json!({
            "session_id": self.session_id,
            "revision": self.revision,
            "finalized": self.finalized,
            "partials": self.partials,
        })
        .to_string()
    }
}

#[cfg(test)]
mod live_transcript_cache_tests {
    use super::LiveTranscriptCache;
    use engine_runtime::{AudioSource, SessionId, SessionInfo, TranscriptUpdate};

    fn update(id: &str, text: &str, final_result: bool) -> TranscriptUpdate {
        TranscriptUpdate {
            source: AudioSource::Microphone,
            utterance_id: id.into(),
            start_ms: 10,
            end_ms: 20,
            stable_text: text.into(),
            unstable_text: String::new(),
            is_final: final_result,
            language: Some("en".into()),
            confidence: None,
        }
    }

    #[test]
    fn partials_replace_and_finalized_utterances_are_deduplicated() {
        let mut cache = LiveTranscriptCache::default();
        cache.update(update("utterance-1", "hello", false));
        cache.update(update("utterance-1", "hello there", false));
        assert_eq!(cache.partials.len(), 1);
        assert_eq!(cache.partials[0].stable_text, "hello there");

        cache.update(update("utterance-1", "Hello there.", true));
        cache.update(update("utterance-1", "Hello there!", true));
        assert!(cache.partials.is_empty());
        assert_eq!(cache.finalized.len(), 1);
        assert_eq!(cache.finalized[0].stable_text, "Hello there!");
    }

    #[test]
    fn session_change_clears_the_previous_session_transcript() {
        let mut cache = LiveTranscriptCache::default();
        cache.update(update("utterance-1", "old session", true));
        cache.observe_snapshot(&engine_runtime::EngineSnapshot {
            session: Some(SessionInfo {
                id: SessionId("new-session".into()),
                started_at_unix_ms: 0,
                recording_directory: "/tmp/new-session".into(),
            }),
            ..Default::default()
        });
        assert!(cache.finalized.is_empty());
        assert!(cache.partials.is_empty());
    }
}

unsafe extern "C" {
    fn rimv_menu_create(
        root: *const c_char,
        snapshot: *const c_char,
        command: extern "C" fn(u32, u8),
    );
    fn rimv_menu_run();
    fn rimv_menu_update(snapshot: *const c_char);
    fn rimv_menu_error(message: *const c_char);
    fn rimv_menu_exit();
    fn rimv_menu_self_test() -> bool;
    fn rimv_menu_set_model_summary(summary: *const c_char);
    fn rimv_menu_set_language(language: *const c_char);
    fn rimv_menu_set_selector_state(state: *const c_char);
    fn rimv_model_manager_configure(
        catalog_json: *const c_char,
        storage_path: *const c_char,
        command: extern "C" fn(u32, u8),
    );
    fn rimv_model_manager_update(catalog_json: *const c_char);
    fn rimv_transcription_window_configure(command: extern "C" fn(u32, u8));
    fn rimv_transcription_window_snapshot(snapshot_json: *const c_char);
    fn rimv_transcription_window_update(update_json: *const c_char);
    fn rimv_transcription_window_open_live(
        snapshot_json: *const c_char,
        transcript_state_json: *const c_char,
    );
    fn rimv_transcription_window_shutdown_playback();
    fn rimv_transcription_window_recording_metadata(path: *const c_char, title: *const c_char, deleted: u8);
    fn rimv_transcription_window_close_session(path: *const c_char);
    fn rimv_recordings_selector_update(json: *const c_char);
}

enum Action {
    Command(EngineCommand),
    SetLanguage(Option<String>),
    SelectModel(String),
    RemoveModel(String),
    RenameRecording { path: PathBuf, title: String },
    DeleteRecording(PathBuf),
    OpenLiveViewer,
    Quit,
}
static COMMANDS: OnceLock<SyncSender<Action>> = OnceLock::new();
static MODELS: OnceLock<ModelManager> = OnceLock::new();
static SETTINGS_ROOT: OnceLock<PathBuf> = OnceLock::new();
static RECORDINGS_ROOT: OnceLock<PathBuf> = OnceLock::new();
static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static SELECTED_MODEL: OnceLock<Mutex<String>> = OnceLock::new();
static DOWNLOADING: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
static DOWNLOAD_PROGRESS: OnceLock<Mutex<std::collections::HashMap<String, (u64, Option<u64>)>>> =
    OnceLock::new();
static LIVE_TRANSCRIPT: OnceLock<Mutex<LiveTranscriptCache>> = OnceLock::new();

const RIMV_SESSION_METADATA: &str = "rimv-session.json";

fn validated_recording_directory(path: &std::path::Path) -> Result<PathBuf, String> {
    let root = RECORDINGS_ROOT
        .get()
        .ok_or_else(|| "recordings are not initialized".to_owned())?
        .canonicalize()
        .map_err(|error| format!("could not access recordings folder: {error}"))?;
    let directory = path
        .canonicalize()
        .map_err(|error| format!("could not access recording: {error}"))?;
    if directory.parent() != Some(root.as_path()) || directory == root {
        return Err("the selected folder is not a RimV recording".into());
    }
    Ok(directory)
}

fn session_metadata(path: &std::path::Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(path.join(RIMV_SESSION_METADATA)).ok()?).ok()
}

fn persisted_session_identity(path: &std::path::Path) -> Result<(String, u64), String> {
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(path.join("session.json")).map_err(|error| format!("could not read session metadata: {error}"))?,
    )
    .map_err(|error| format!("could not read session metadata: {error}"))?;
    let session = metadata.get("snapshot").and_then(|snapshot| snapshot.get("session")).ok_or("recording metadata is incomplete")?;
    let id = session.get("id").and_then(serde_json::Value::as_str).ok_or("recording identifier is missing")?;
    let started_at = session.get("started_at_unix_ms").and_then(serde_json::Value::as_u64).ok_or("recording start time is missing")?;
    if path.file_name().and_then(|name| name.to_str()) != Some(id) {
        return Err("recording metadata does not match its session folder".into());
    }
    Ok((id.to_owned(), started_at))
}

fn write_sidecar(path: &std::path::Path, session_id: &str, started_at_unix_ms: u64, title: &str) -> Result<(), String> {
    let metadata = serde_json::json!({
        "schema_version": 1,
        "session_id": session_id,
        "started_at_unix_ms": started_at_unix_ms,
        "title": title,
    });
    let temporary = path.join(format!("{RIMV_SESSION_METADATA}.tmp"));
    let destination = path.join(RIMV_SESSION_METADATA);
    let bytes = serde_json::to_vec_pretty(&metadata).map_err(|error| error.to_string())?;
    std::fs::write(&temporary, bytes).map_err(|error| format!("could not save recording name: {error}"))?;
    std::fs::rename(&temporary, &destination).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("could not save recording name: {error}")
    })
}

fn enqueue_recording_action(action: Action) {
    if COMMANDS.get().is_none_or(|sender| sender.try_send(action).is_err()) {
        show_error("RimV is busy. Try the recording action again.");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_recording_rename(path: *const c_char, title: *const c_char) {
    if path.is_null() || title.is_null() { return; }
    let (Ok(path), Ok(title)) = (unsafe { CStr::from_ptr(path) }.to_str(), unsafe { CStr::from_ptr(title) }.to_str()) else { return; };
    enqueue_recording_action(Action::RenameRecording { path: PathBuf::from(path), title: title.to_owned() });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_recording_delete(path: *const c_char) {
    if path.is_null() { return; }
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else { return; };
    enqueue_recording_action(Action::DeleteRecording(PathBuf::from(path)));
}

extern "C" fn command(code: u32, enabled: u8) {
    if let Some(model_id) = match code {
        40 => Some("parakeet-tdt-0.6b-v3-int8"),
        41 => Some("whisper-tiny"),
        42 => Some("whisper-base"),
        43 => Some("whisper-small"),
        44 => Some("whisper-medium"),
        45 => Some("whisper-large"),
        46 => Some("whisper-turbo"),
        _ => None,
    } {
        let downloads = DOWNLOADING.get_or_init(|| Mutex::new(Default::default()));
        if !downloads
            .lock()
            .expect("download lock poisoned")
            .insert(model_id.into())
        {
            return;
        }
        let _ = thread::Builder::new()
            .name("model-download".into())
            .spawn(move || {
                let Some(manager) = MODELS.get().cloned() else {
                    show_error("model manager is not initialized");
                    return;
                };
                let chosen = match manager.descriptor(model_id) {
                    Ok(model) => model.clone(),
                    Err(error) => {
                        show_error(&error.to_string());
                        return;
                    }
                };
                let cancelled = std::sync::atomic::AtomicBool::new(false);
                publish_model_catalog();
                let result = if chosen.backend == "parakeet" {
                    manager.install_parakeet(&cancelled, |done, total| {
                        publish_download_progress(&chosen, done, total);
                    })
                } else {
                    manager.install(model_id, &cancelled, |done, total| {
                        let text = format!(
                            "Downloading {}\n{:.0}%\n{:.0} MB / {:.0} MB",
                            chosen.display_name,
                            total.map_or(0.0, |size| done as f64 * 100.0 / size as f64),
                            done as f64 / 1_000_000.0,
                            total.unwrap_or(0) as f64 / 1_000_000.0
                        );
                        if let Ok(text) = CString::new(text) {
                            unsafe { rimv_menu_set_model_summary(text.as_ptr()) };
                        }
                        publish_download_progress(&chosen, done, total);
                    })
                };
                match result {
                    Ok(path) => {
                        let _ = path;
                        let _ = COMMANDS
                            .get()
                            .map(|sender| sender.try_send(Action::SelectModel(model_id.into())));
                        if let Ok(text) = CString::new(format!(
                            "{} is installed and selected.\nTranscription is enabled.",
                            chosen.display_name,
                        )) {
                            unsafe { rimv_menu_set_model_summary(text.as_ptr()) };
                        }
                    }
                    Err(error) => show_error(&format!(
                        "{} installation failed: {error}",
                        chosen.display_name
                    )),
                }
                DOWNLOADING
                    .get()
                    .expect("download state initialized")
                    .lock()
                    .expect("download lock poisoned")
                    .remove(model_id);
                if let Some(progress) = DOWNLOAD_PROGRESS.get() {
                    progress.lock().expect("download progress lock poisoned").remove(model_id);
                }
                publish_model_catalog();
            });
        return;
    }
    if let Some(model_id) = model_for_code(code, 60) {
        let _ = COMMANDS
            .get()
            .map(|sender| sender.try_send(Action::SelectModel(model_id.into())));
        return;
    }
    if let Some(model_id) = model_for_code(code, 80) {
        let _ = COMMANDS
            .get()
            .map(|sender| sender.try_send(Action::RemoveModel(model_id.into())));
        return;
    }
    if code == 5 {
        // A full control queue must never lose Quit. A queued item wakes the
        // worker, which sees this flag before executing any further command.
        QUIT_REQUESTED.store(true, Ordering::Release);
        if let Some(sender) = COMMANDS.get() {
            let _ = sender.try_send(Action::Quit);
        }
        return;
    }
    if code == 6 {
        if COMMANDS
            .get()
            .is_none_or(|sender| sender.try_send(Action::OpenLiveViewer).is_err())
        {
            show_error("RimV is busy. Try opening the live viewer again.");
        }
        return;
    }
    let action = match code {
        1 => Action::Command(EngineCommand::StartCapture),
        2 => Action::Command(EngineCommand::StopCapture),
        3 => Action::Command(EngineCommand::SetMicrophoneEnabled {
            enabled: enabled != 0,
        }),
        4 => Action::Command(EngineCommand::SetSystemAudioEnabled {
            enabled: enabled != 0,
        }),
        9 => Action::Command(EngineCommand::SetTranscriptionEnabled {
            enabled: enabled != 0,
        }),
        10 => Action::SetLanguage(None),
        11 => Action::SetLanguage(Some("en".into())),
        12 => Action::SetLanguage(Some("es".into())),
        13 => Action::SetLanguage(Some("fr".into())),
        14 => Action::SetLanguage(Some("de".into())),
        15 => Action::SetLanguage(Some("pt".into())),
        16 => Action::SetLanguage(Some("it".into())),
        17 => Action::SetLanguage(Some("ja".into())),
        18 => Action::SetLanguage(Some("zh".into())),
        19 => Action::SetLanguage(Some("hi".into())),
        20 => Action::SetLanguage(Some("ar".into())),
        21 => Action::SetLanguage(Some("ru".into())),
        22 => Action::SetLanguage(Some("bg".into())),
        23 => Action::SetLanguage(Some("hr".into())),
        24 => Action::SetLanguage(Some("cs".into())),
        25 => Action::SetLanguage(Some("da".into())),
        26 => Action::SetLanguage(Some("nl".into())),
        27 => Action::SetLanguage(Some("et".into())),
        28 => Action::SetLanguage(Some("fi".into())),
        29 => Action::SetLanguage(Some("el".into())),
        30 => Action::SetLanguage(Some("hu".into())),
        31 => Action::SetLanguage(Some("lv".into())),
        32 => Action::SetLanguage(Some("lt".into())),
        33 => Action::SetLanguage(Some("mt".into())),
        34 => Action::SetLanguage(Some("pl".into())),
        35 => Action::SetLanguage(Some("ro".into())),
        36 => Action::SetLanguage(Some("sk".into())),
        37 => Action::SetLanguage(Some("sl".into())),
        38 => Action::SetLanguage(Some("sv".into())),
        39 => Action::SetLanguage(Some("uk".into())),
        5 => Action::Quit,
        _ => return,
    };
    if COMMANDS
        .get()
        .is_none_or(|sender| sender.try_send(action).is_err())
    {
        show_error("Controls are busy. Wait for the current operation, then try again.");
    }
}

fn model_for_code(code: u32, base: u32) -> Option<&'static str> {
    match code.checked_sub(base)? {
        0 => Some("parakeet-tdt-0.6b-v3-int8"),
        1 => Some("whisper-tiny"),
        2 => Some("whisper-base"),
        3 => Some("whisper-small"),
        4 => Some("whisper-medium"),
        5 => Some("whisper-large"),
        6 => Some("whisper-turbo"),
        _ => None,
    }
}

fn model_settings_path(support: &std::path::Path) -> PathBuf {
    support.join("menu-model")
}
fn saved_model(support: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(model_settings_path(support))
        .ok()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
}
fn persist_model(support: &std::path::Path, id: &str) -> std::io::Result<()> {
    std::fs::write(model_settings_path(support), id)
}

fn publish_download_progress(model: &ModelDescriptor, done: u64, total: Option<u64>) {
    DOWNLOAD_PROGRESS
        .get_or_init(|| Mutex::new(Default::default()))
        .lock()
        .expect("download progress lock poisoned")
        .insert(model.id.clone(), (done, total));
    publish_model_catalog();
}
fn publish_model_catalog() {
    let (Some(models), Some(selected)) = (MODELS.get(), SELECTED_MODEL.get()) else {
        return;
    };
    let selected = selected
        .lock()
        .expect("selected model lock poisoned")
        .clone();
    let downloading = DOWNLOADING
        .get()
        .map(|state| state.lock().expect("download lock poisoned").clone())
        .unwrap_or_default();
    let progress = DOWNLOAD_PROGRESS
        .get()
        .map(|state| state.lock().expect("download progress lock poisoned").clone())
        .unwrap_or_default();
    let items: Vec<_> = models.catalog().iter().filter(|model| model.backend == "parakeet" || model.backend == "whisper").map(|model| {
        let size = model.files.first().and_then(|file| file.expected_size_bytes).map(|bytes| format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)).unwrap_or_else(|| match model.id.as_str() { "whisper-medium" => "1.5 GB".into(), "whisper-large" => "3.1 GB".into(), _ => "Managed package".into() });
        let progress_text = progress.get(&model.id).map(|(done, total)| match total { Some(total) => format!("{:.0}% · {:.1} MB / {:.1} MB", *done as f64 * 100.0 / *total as f64, *done as f64 / 1_000_000.0, *total as f64 / 1_000_000.0), None => format!("{:.1} MB downloaded", *done as f64 / 1_000_000.0) });
        serde_json::json!({"id":model.id,"name":model.display_name,"description":model.install_hint.as_deref().unwrap_or("Local transcription model."),"size":size,"progress":progress_text,"state":if models.state(model)==ModelState::Ready {"installed"} else if downloading.contains(&model.id) {"downloading"} else {"available"},"selected":model.id==selected})
    }).collect();
    if let Ok(json) = CString::new(serde_json::to_string(&items).unwrap_or_default()) {
        unsafe { rimv_model_manager_update(json.as_ptr()) };
    }
}

fn language_settings_path(support: &std::path::Path) -> PathBuf {
    support.join("menu-language")
}

fn saved_language(support: &std::path::Path) -> Option<String> {
    let value = std::fs::read_to_string(language_settings_path(support)).ok()?;
    let value = value.trim();
    match value {
        "auto" | "ar" | "bg" | "cs" | "da" | "de" | "el" | "en" | "es" | "et" | "fi" | "fr"
        | "hi" | "hr" | "hu" | "it" | "ja" | "lt" | "lv" | "mt" | "nl" | "pl" | "pt" | "ro"
        | "ru" | "sk" | "sl" | "sv" | "uk" | "zh" => Some(value.into()),
        _ => None,
    }
}

fn selected_language_for_model(saved: Option<String>, model: Option<&ModelDescriptor>) -> String {
    let Some(saved) = saved else {
        return "auto".into();
    };
    if saved == "auto" {
        return "auto".into();
    }
    model
        .is_some_and(|model| model.languages.iter().any(|language| language == &saved))
        .then_some(saved)
        .unwrap_or_else(|| "auto".into())
}

fn update_selector_state(model: Option<&ModelDescriptor>) {
    let state = serde_json::json!({
        "model": model.map(|model| model.display_name.as_str()).unwrap_or("No model"),
        "languages": model.map(|model| &model.languages).cloned().unwrap_or_default(),
        "auto_detect": model.is_some_and(|model| model.capabilities.supports_language_detection),
    });
    if let Ok(text) = CString::new(state.to_string()) {
        unsafe { rimv_menu_set_selector_state(text.as_ptr()) };
    }
}

fn persist_language(support: &std::path::Path, language: Option<&str>) -> std::io::Result<()> {
    std::fs::write(language_settings_path(support), language.unwrap_or("auto"))
}

fn show_error(error: &str) {
    eprintln!("{error}");
    if let Ok(text) = CString::new(error) {
        // Native code copies the string before returning and dispatches to AppKit.
        unsafe { rimv_menu_error(text.as_ptr()) };
    }
}

fn update(snapshot: &engine_runtime::EngineSnapshot) {
    LIVE_TRANSCRIPT
        .get_or_init(|| Mutex::new(LiveTranscriptCache::default()))
        .lock()
        .expect("live transcript lock poisoned")
        .observe_snapshot(snapshot);
    if let Ok(json) = serde_json::to_string(snapshot)
        && let Ok(text) = CString::new(json)
    {
        // Only serialized control state crosses this boundary, never audio data.
        unsafe { rimv_menu_update(text.as_ptr()) };
        unsafe { rimv_transcription_window_snapshot(text.as_ptr()) };
    }
    publish_recordings(snapshot);
}

fn publish_recordings(snapshot: &engine_runtime::EngineSnapshot) {
    let Some(root) = RECORDINGS_ROOT.get() else { return; };
    let mut items = Vec::new();
    if let Some(session) = &snapshot.session
        && matches!(snapshot.status, engine_runtime::EngineStatus::Recording | engine_runtime::EngineStatus::Starting | engine_runtime::EngineStatus::Stopping)
    {
        let path = PathBuf::from(&session.recording_directory);
        let name = session_metadata(&path)
            .filter(|metadata| metadata["session_id"].as_str() == Some(session.id.0.as_str()))
            .and_then(|metadata| metadata["title"].as_str().map(str::to_owned));
        items.push(serde_json::json!({"state":"recording","name":name,"session_id":session.id.0,"started_at_unix_ms":session.started_at_unix_ms,"duration_ms":snapshot.elapsed_ms,"path":session.recording_directory,"root":root}));
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let metadata_path = path.join("session.json");
            let Ok(metadata) = std::fs::read(&metadata_path).and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).map_err(std::io::Error::other)) else { continue; };
            let Some(session) = metadata.pointer("/snapshot/session") else { continue; };
            let session_id = session.get("id").and_then(serde_json::Value::as_str).unwrap_or_default();
            let started_at_unix_ms = session.get("started_at_unix_ms").and_then(serde_json::Value::as_u64).unwrap_or_default();
            let duration_ms = metadata.pointer("/snapshot/elapsed_ms").and_then(serde_json::Value::as_u64).unwrap_or_default();
            let name = session_metadata(&path)
                .filter(|title| title["session_id"].as_str() == Some(session_id))
                .and_then(|title| title["title"].as_str().map(str::to_owned));
            items.push(serde_json::json!({"state":"completed","name":name,"session_id":session_id,"started_at_unix_ms":started_at_unix_ms,"duration_ms":duration_ms,"path":path,"root":root}));
        }
    }
    items.sort_by(|a, b| {
        let active_a = a["state"] == "recording";
        let active_b = b["state"] == "recording";
        active_b.cmp(&active_a).then_with(|| b["started_at_unix_ms"].as_u64().cmp(&a["started_at_unix_ms"].as_u64()))
    });
    if let Ok(text)=CString::new(serde_json::to_string(&items).unwrap_or_default()) { unsafe { rimv_recordings_selector_update(text.as_ptr()) }; }
}

fn active_recording_matches(snapshot: &engine_runtime::EngineSnapshot, directory: &PathBuf) -> bool {
    (matches!(snapshot.status, engine_runtime::EngineStatus::Starting | engine_runtime::EngineStatus::Recording | engine_runtime::EngineStatus::Stopping)
        || snapshot.microphone.active
        || snapshot.system_audio.active)
        && snapshot.session.as_ref().is_some_and(|session| {
            PathBuf::from(&session.recording_directory).canonicalize().ok().as_ref() == Some(directory)
        })
}

fn rename_recording(snapshot: &engine_runtime::EngineSnapshot, requested_path: &PathBuf, title: &str) -> Result<(PathBuf, String), String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 120 || title.chars().any(char::is_control) {
        return Err("recording names must contain 1–120 visible characters".into());
    }
    let path = validated_recording_directory(requested_path)?;
    let (session_id, started_at) = if active_recording_matches(snapshot, &path) {
        let session = snapshot.session.as_ref().ok_or("active session is unavailable")?;
        if path.file_name().and_then(|name| name.to_str()) != Some(session.id.0.as_str()) {
            return Err("recording identifier does not match its session folder".into());
        }
        (session.id.0.clone(), session.started_at_unix_ms)
    } else {
        persisted_session_identity(&path)?
    };
    write_sidecar(&path, &session_id, started_at, title)?;
    Ok((path, title.to_owned()))
}

fn delete_recording(snapshot: &engine_runtime::EngineSnapshot, requested_path: &PathBuf) -> Result<PathBuf, String> {
    let path = validated_recording_directory(requested_path)?;
    if active_recording_matches(snapshot, &path) {
        return Err("an active or finalizing recording cannot be deleted".into());
    }
    if !path.join("session.json").is_file() {
        return Err("only completed recordings can be deleted".into());
    }
    persisted_session_identity(&path)?;
    let path_text = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "invalid recording path")?;
    unsafe { rimv_transcription_window_close_session(path_text.as_ptr()) };
    std::fs::remove_dir_all(&path).map_err(|error| format!("could not delete recording: {error}"))?;
    Ok(path)
}

#[cfg(test)]
mod recording_metadata_tests {
    use super::*;
    use engine_runtime::{EngineSnapshot, EngineStatus, SessionId, SessionInfo};
    use std::sync::atomic::AtomicU64;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_session_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("rimv-session-metadata-{}-{nonce}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn recording_title_sidecar_round_trips_session_identity() {
        let path = temporary_session_path();
        write_sidecar(&path, "stable-session-id", 1_790_000_000_000, "Planning Notes").unwrap();
        let metadata = session_metadata(&path).unwrap();
        assert_eq!(metadata["session_id"], "stable-session-id");
        assert_eq!(metadata["started_at_unix_ms"], 1_790_000_000_000u64);
        assert_eq!(metadata["title"], "Planning Notes");
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn active_and_finalizing_session_states_are_protected_from_deletion() {
        let path = temporary_session_path();
        let session = SessionInfo {
            id: SessionId("active-id".into()),
            started_at_unix_ms: 1,
            recording_directory: path.to_string_lossy().into_owned(),
        };
        for status in [EngineStatus::Starting, EngineStatus::Recording, EngineStatus::Stopping] {
            let snapshot = EngineSnapshot { status, session: Some(session.clone()), ..Default::default() };
            assert!(active_recording_matches(&snapshot, &path.canonicalize().unwrap()));
        }
        let completed = EngineSnapshot { status: EngineStatus::Idle, session: Some(session), ..Default::default() };
        assert!(!active_recording_matches(&completed, &path.canonicalize().unwrap()));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn explicit_recordings_root_is_restored_when_launch_argument_is_omitted() {
        let support = temporary_session_path();
        let recordings = temporary_session_path();
        let canonical_recordings = recordings.canonicalize().unwrap();
        let explicit = Options {
            directory: recordings.clone(),
            recordings_override: true,
            profile: None,
            seconds: 30,
            self_test: false,
        };
        assert_eq!(resolve_recordings_directory(&support, &explicit).unwrap(), canonical_recordings);
        let later_launch = Options {
            directory: PathBuf::from("default-recordings"),
            recordings_override: false,
            profile: None,
            seconds: 30,
            self_test: false,
        };
        assert_eq!(resolve_recordings_directory(&support, &later_launch).unwrap(), canonical_recordings);
        std::fs::remove_dir_all(support).unwrap();
        std::fs::remove_dir_all(recordings).unwrap();
    }
}

fn update_transcription_window(update: &engine_runtime::TranscriptUpdate) {
    let cache = LIVE_TRANSCRIPT.get_or_init(|| Mutex::new(LiveTranscriptCache::default()));
    let json = {
        let mut cache = cache.lock().expect("live transcript lock poisoned");
        cache.update(update.clone());
        serde_json::json!({ "revision": cache.revision, "update": update }).to_string()
    };
    if let Ok(text) = CString::new(json) {
        unsafe { rimv_transcription_window_update(text.as_ptr()) };
    }
}

struct Options {
    directory: PathBuf,
    recordings_override: bool,
    profile: Option<String>,
    seconds: u64,
    self_test: bool,
}
impl Options {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut options = Self {
            directory: PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
                .join("Library/Application Support/rimv/recordings"),
            recordings_override: false,
            profile: None,
            seconds: 30,
            self_test: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--self-test" => options.self_test = true,
                "--recordings" => {
                    options.directory = args.next().ok_or("missing recordings path")?.into();
                    options.recordings_override = true;
                }
                "--profile" => {
                    let mode = args.next().ok_or("missing profile mode")?;
                    if !["idle", "mic", "both", "toggles"].contains(&mode.as_str()) {
                        return Err("profile mode must be idle, mic, both, or toggles".into());
                    }
                    options.profile = Some(mode);
                }
                "--seconds" => {
                    options.seconds = args.next().ok_or("missing seconds")?.parse()?;
                    if !(5..=3600).contains(&options.seconds) {
                        return Err("seconds must be between 5 and 3600".into());
                    }
                }
                _ => return Err(format!("unknown argument: {arg}").into()),
            }
        }
        Ok(options)
    }
}

fn resolve_recordings_directory(support: &std::path::Path, options: &Options) -> Result<PathBuf, std::io::Error> {
    let configured = support.join("recordings-directory.txt");
    if options.recordings_override {
        let requested = if options.directory.is_absolute() {
            options.directory.clone()
        } else {
            std::env::current_dir()?.join(&options.directory)
        };
        let requested = requested.canonicalize().unwrap_or(requested);
        let temporary = support.join("recordings-directory.txt.tmp");
        std::fs::write(&temporary, requested.to_string_lossy().as_bytes())?;
        std::fs::rename(temporary, configured)?;
        return Ok(requested);
    }
    if let Ok(saved) = std::fs::read_to_string(&configured) {
        let saved = PathBuf::from(saved.trim());
        if saved.is_absolute() {
            return Ok(saved);
        }
    }
    if options.directory.is_absolute() {
        Ok(options.directory.clone())
    } else {
        Ok(std::env::current_dir()?.join(&options.directory))
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse()?;
    let support = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
        .join("Library/Application Support/rimv");
    std::fs::create_dir_all(&support)?;
    let instance = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(support.join("menu.lock"))?;
    // Advisory lock is released by the OS even after a crash. This prevents
    // duplicate icons and simultaneous menu-app sessions across launch paths.
    instance
        .try_lock()
        .map_err(|_| "rimv is already running; use its menu-bar icon")?;
    let directory = resolve_recordings_directory(&support, &options)?;
    std::fs::create_dir_all(&directory)?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
    let model_root = std::env::var_os("RIMV_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| ModelManager::default_root(&home));
    let models = ModelManager::new(model_root);
    let catalog = models.catalog();
    let selected_model = saved_model(&support)
        .and_then(|id| {
            catalog
                .iter()
                .find(|model| model.id == id && models.state(model) == ModelState::Ready)
                .cloned()
        })
        .or_else(|| {
            catalog
                .iter()
                .find(|model| {
                    model.backend == "parakeet" && models.state(model) == ModelState::Ready
                })
                .or_else(|| {
                    catalog.iter().find(|model| {
                        model.backend == "whisper" && models.state(model) == ModelState::Ready
                    })
                })
                .cloned()
        });
    let selected = selected_model.as_ref().map(|model| {
        if model.backend == "parakeet" {
            models.directory(model)
        } else {
            models.path(model, &model.files[0])
        }
    });
    let mut config = EngineConfig {
        recordings_directory: directory.clone(),
        ..Default::default()
    };
    config.transcription.backend = if selected_model
        .as_ref()
        .is_some_and(|model| model.backend == "whisper")
    {
        AsrBackendKind::Whisper
    } else {
        AsrBackendKind::Parakeet
    };
    let selected_language =
        selected_language_for_model(saved_language(&support), selected_model.as_ref());
    config.transcription.language =
        (selected_language != "auto").then_some(selected_language.clone());
    config.transcription_enabled = selected_model.is_some();
    if let Some(selected) = selected.filter(|path| path.exists()) {
        config.transcription.model_path = Some(selected);
    }
    let engine = EngineRuntime::new(config)?;
    engine_runtime::websocket::attach_local_websocket(engine.clone(), 7878)
        .map_err(|error| format!("could not start local web bridge: {error}"))?;
    let events = engine.subscribe()?;
    let (sender, actions) = mpsc::sync_channel(8);
    COMMANDS
        .set(sender)
        .map_err(|_| "menu already initialized")?;
    MODELS
        .set(models.clone())
        .map_err(|_| "model manager already initialized")?;
    SETTINGS_ROOT
        .set(support.clone())
        .map_err(|_| "menu settings already initialized")?;
    RECORDINGS_ROOT.set(directory.clone()).map_err(|_| "recordings root already initialized")?;
    SELECTED_MODEL
        .set(Mutex::new(
            selected_model
                .as_ref()
                .map(|model| model.id.clone())
                .unwrap_or_default(),
        ))
        .map_err(|_| "selected model already initialized")?;
    let root = CString::new(directory.to_string_lossy().as_bytes())?;
    let initial = CString::new(serde_json::to_string(&engine.snapshot())?)?;
    // AppKit is created and run on the process main thread.
    unsafe { rimv_menu_create(root.as_ptr(), initial.as_ptr(), command) };
    unsafe { rimv_transcription_window_configure(command) };
    update(&engine.snapshot());
    let model_catalog = CString::new("[]")?;
    let model_root = CString::new(models.root().to_string_lossy().as_bytes())?;
    unsafe { rimv_model_manager_configure(model_catalog.as_ptr(), model_root.as_ptr(), command) };
    publish_model_catalog();
    let selected_language = CString::new(selected_language)?;
    unsafe { rimv_menu_set_language(selected_language.as_ptr()) };
    update_selector_state(selected_model.as_ref());
    let primary = models.descriptor("parakeet-tdt-0.6b-v3-int8")?;
    let summary = CString::new(format!(
        "Primary ASR: {}\nStatus: {:?}\n{}\n\nLegacy Whisper downloads remain optional.",
        primary.display_name,
        models.state(primary),
        primary
            .install_hint
            .as_deref()
            .unwrap_or("Ready for explicit installation.")
    ))?;
    unsafe { rimv_menu_set_model_summary(summary.as_ptr()) };
    if options.self_test {
        let passed = unsafe { rimv_menu_self_test() };
        engine.shutdown()?;
        return if passed {
            Ok(())
        } else {
            Err("native menu self-test failed".into())
        };
    }
    let controller_engine = engine.clone();
    let controller = thread::Builder::new()
        .name("menu-controls".into())
        .spawn(move || {
            while let Ok(action) = actions.recv() {
                let action = if QUIT_REQUESTED.load(Ordering::Acquire) {
                    Action::Quit
                } else {
                    action
                };
                match action {
                    Action::Command(command) => {
                        if let Err(error) = controller_engine.send(command) {
                            show_error(&error.to_string());
                        }
                    }
                    Action::SetLanguage(language) => {
                        if let Err(error) =
                            controller_engine.send(EngineCommand::SetTranscriptionLanguage {
                                language: language.clone(),
                            })
                        {
                            show_error(&error.to_string());
                            continue;
                        }
                        if let Err(error) = persist_language(&support, language.as_deref()) {
                            show_error(&format!("could not save transcription language: {error}"));
                        }
                    }
                    Action::SelectModel(id) => {
                        let Some(manager) = MODELS.get() else {
                            show_error("model manager is not initialized");
                            continue;
                        };
                        let Ok(model) = manager.descriptor(&id) else {
                            show_error("unknown model");
                            continue;
                        };
                        if manager.state(model) != ModelState::Ready {
                            show_error("download the model before selecting it");
                            continue;
                        }
                        let path = if model.backend == "parakeet" {
                            manager.directory(model)
                        } else {
                            manager.path(model, &model.files[0])
                        };
                        let backend = model.backend.clone();
                        if let Err(error) = controller_engine
                            .send(EngineCommand::SetTranscriptionBackend { backend })
                            .and_then(|_| {
                                controller_engine.send(EngineCommand::SetTranscriptionModel {
                                    path: path.to_string_lossy().into_owned(),
                                })
                            })
                        {
                            show_error(&error.to_string());
                            continue;
                        }
                        let saved = saved_language(&support);
                        let language = selected_language_for_model(saved, Some(model));
                        if let Err(error) =
                            controller_engine.send(EngineCommand::SetTranscriptionLanguage {
                                language: (language != "auto").then_some(language),
                            })
                        {
                            show_error(&error.to_string());
                            continue;
                        }
                        if let Some(selected) = SELECTED_MODEL.get() {
                            *selected.lock().expect("selected model lock poisoned") = id.clone();
                        }
                        if let Err(error) = persist_model(&support, &id) {
                            show_error(&format!("could not save selected model: {error}"));
                        }
                        update_selector_state(Some(model));
                        publish_model_catalog();
                    }
                    Action::RemoveModel(id) => {
                        let Some(manager) = MODELS.get() else {
                            show_error("model manager is not initialized");
                            continue;
                        };
                        let selected = SELECTED_MODEL.get().is_some_and(|current| {
                            *current.lock().expect("selected model lock poisoned") == id
                        });
                        if selected {
                            show_error("select another model before removing the current model");
                            continue;
                        }
                        if let Err(error) = manager.remove(&id) {
                            show_error(&format!("could not remove model: {error}"));
                        } else {
                            publish_model_catalog();
                        }
                    }
                    Action::RenameRecording { path, title } => {
                        match rename_recording(&controller_engine.snapshot(), &path, &title) {
                            Ok((path, title)) => {
                                publish_recordings(&controller_engine.snapshot());
                                if let (Ok(path), Ok(title)) = (
                                    CString::new(path.to_string_lossy().as_bytes()),
                                    CString::new(title),
                                ) {
                                    unsafe { rimv_transcription_window_recording_metadata(path.as_ptr(), title.as_ptr(), 0) };
                                }
                            }
                            Err(error) => show_error(&error),
                        }
                    }
                    Action::DeleteRecording(path) => {
                        match delete_recording(&controller_engine.snapshot(), &path) {
                            Ok(_) => publish_recordings(&controller_engine.snapshot()),
                            Err(error) => show_error(&error),
                        }
                    }
                    Action::OpenLiveViewer => {
                        let snapshot = controller_engine.snapshot();
                        let transcript_json = {
                            let mut transcript = LIVE_TRANSCRIPT
                                .get_or_init(|| Mutex::new(LiveTranscriptCache::default()))
                                .lock()
                                .expect("live transcript lock poisoned");
                            transcript.observe_snapshot(&snapshot);
                            transcript.json()
                        };
                        match serde_json::to_string(&snapshot) {
                            Ok(snapshot_json) => match (
                                CString::new(snapshot_json),
                                CString::new(transcript_json),
                            ) {
                                (Ok(snapshot), Ok(transcript)) => unsafe {
                                    rimv_transcription_window_open_live(
                                        snapshot.as_ptr(),
                                        transcript.as_ptr(),
                                    );
                                },
                                _ => show_error("Could not restore the current transcription view."),
                            },
                            Err(_) => show_error("Could not restore the current transcription view."),
                        }
                    }
                    Action::Quit => {
                        unsafe { rimv_transcription_window_shutdown_playback() };
                        if let Err(error) = controller_engine.shutdown() {
                            show_error(&error.to_string());
                        }
                        unsafe { rimv_menu_exit() };
                        break;
                    }
                }
            }
        })?;
    let listener_engine = engine.clone();
    let listener = thread::Builder::new()
        .name("menu-events".into())
        .spawn(move || {
            loop {
                match events.recv() {
                    Ok(EngineEvent::Snapshot { snapshot }) => update(&snapshot),
                    Ok(EngineEvent::Error { error }) => show_error(&error.to_string()),
                    Ok(EngineEvent::TranscriptUpdate { update }) => update_transcription_window(&update),
                    // TranscriptUpdate is the authoritative stable/unstable
                    // representation. Do not append the legacy partial/final
                    // events here, or the live window could duplicate text.
                    Ok(EngineEvent::TranscriptPartial { .. }) | Ok(EngineEvent::TranscriptFinal { .. }) => {}
                    Ok(EngineEvent::TranscriptionError { error }) => show_error(&error.to_string()),
                    Err(SubscriptionError::Lagged { .. }) => update(&listener_engine.snapshot()),
                    Err(SubscriptionError::Closed) => break,
                    Err(SubscriptionError::Timeout) => {}
                }
            }
        })?;
    if let Some(mode) = options.profile {
        let profiling_engine = engine.clone();
        thread::Builder::new()
            .name("menu-profile".into())
            .spawn(move || {
                // Let AppKit settle before starting an explicit profiling scenario.
                thread::sleep(Duration::from_secs(2));
                let outcome = (|| -> engine_runtime::Result<()> {
                    if mode != "idle" {
                        if mode == "both" {
                            profiling_engine.set_system_audio_enabled(true)?;
                        }
                        profiling_engine.start_capture()?;
                    }
                    println!(
                        "PROFILE_READY {}",
                        serde_json::to_string(&profiling_engine.snapshot()).unwrap_or_default()
                    );
                    if mode == "toggles" {
                        for _ in 0..3 {
                            thread::sleep(Duration::from_secs(options.seconds / 6));
                            profiling_engine.set_microphone_enabled(false)?;
                            thread::sleep(Duration::from_secs(options.seconds / 6));
                            profiling_engine.set_microphone_enabled(true)?;
                        }
                    } else {
                        thread::sleep(Duration::from_secs(options.seconds));
                    }
                    if mode != "idle" {
                        profiling_engine.stop_capture()?;
                    }
                    Ok(())
                })();
                println!(
                    "PROFILE_RESULT {}",
                    serde_json::json!({
                        "mode": mode, "error": outcome.err().map(|e| e.to_string()),
                        "snapshot": profiling_engine.snapshot(),
                    })
                );
                // This helper is opt-in and has no thread/timer in normal app use.
                command(5, 0);
            })?;
    }
    unsafe { rimv_menu_run() };
    engine.shutdown()?;
    controller.join().map_err(|_| "menu controller panicked")?;
    listener.join().map_err(|_| "menu listener panicked")?;
    Ok(())
}
