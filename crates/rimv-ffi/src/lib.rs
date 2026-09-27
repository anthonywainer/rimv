//! Versioned JSON-over-C ABI for the native .NET/WinUI host.
//!
//! Every returned C string is UTF-8, owned by the caller, and released with
//! `rimv_string_free`. Calls return JSON envelopes; no Rust panic crosses the
//! ABI. The host must serialize lifecycle and command calls and poll events on
//! a worker thread, never on the WinUI dispatcher.
use engine_protocol::EngineCommand;
use engine_runtime::{EngineConfig, EngineRuntime, ExportFormat, RecordingLibrary, Subscription};
use model_manager::ModelManager;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub const API_VERSION: u32 = 1;

pub struct RimvEngine {
    runtime: EngineRuntime,
    events: Subscription,
    recordings: RecordingLibrary,
    models: ModelManager,
    model_operations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    model_workers: Mutex<Vec<JoinHandle<()>>>,
    model_events: Mutex<Receiver<ModelProgress>>,
    model_event_sender: SyncSender<ModelProgress>,
}

#[derive(Deserialize)]
struct InitConfig {
    recordings_directory: PathBuf,
    models_directory: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    GetState,
    Send {
        command: EngineCommand,
    },
    PollEvent {
        timeout_ms: Option<u64>,
    },
    GetModels,
    SelectModel {
        model_id: String,
        language: Option<String>,
    },
    SetLanguage {
        model_id: String,
        language: Option<String>,
    },
    ClearModel,
    InstallModel {
        model_id: String,
    },
    CancelModelInstall {
        model_id: String,
    },
    GetRecordings,
    GetRecording {
        session_id: String,
    },
    RenameRecording {
        session_id: String,
        title: String,
    },
    DeleteRecording {
        session_id: String,
    },
    Export {
        session_id: String,
        format: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ModelProgress {
    model_id: String,
    phase: String,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    error: Option<String>,
}

fn envelope<T: serde::Serialize>(result: std::result::Result<T, String>) -> String {
    match result {
        Ok(value) => serde_json::json!({"ok":true,"result":value}).to_string(),
        Err(message) => {
            serde_json::json!({"ok":false,"error":{"code":"request_failed","message":message}})
                .to_string()
        }
    }
}

fn owned_string(value: String) -> *mut c_char {
    CString::new(value.replace('\0', "�"))
        .expect("replacement removes NUL")
        .into_raw()
}

fn start_model_install(engine: &RimvEngine, model_id: String) -> Result<(), String> {
    let descriptor = engine
        .models
        .descriptor(&model_id)
        .map_err(|e| e.to_string())?
        .clone();
    if !engine_runtime::supports_asr_backend(&descriptor.backend) {
        return Err("this model backend is not available in the current RimV build".into());
    }
    if engine.models.state(&descriptor) == model_manager::ModelState::Ready {
        return Ok(());
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut operations = engine
            .model_operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if operations.contains_key(&model_id) {
            return Err("model installation is already running".into());
        }
        operations.insert(model_id.clone(), cancelled.clone());
    }
    let manager = engine.models.clone();
    let events = engine.model_event_sender.clone();
    let worker_id = model_id.clone();
    let operations = engine.model_operations.clone();
    let worker = match thread::Builder::new()
        .name("rimv-core-model-install".into())
        .spawn(move || {
            let last_reported = std::cell::Cell::new(0_u64);
            let report = |phase: String,
                          downloaded_bytes: u64,
                          total_bytes: Option<u64>,
                          error: Option<String>| {
                let _ = events.try_send(ModelProgress {
                    model_id: worker_id.clone(),
                    phase,
                    downloaded_bytes,
                    total_bytes,
                    error,
                });
            };
            let result = if descriptor.backend == "parakeet" {
                manager.install_parakeet_with_phase(
                    &cancelled,
                    |phase| report(phase.into(), last_reported.get(), None, None),
                    |downloaded, total| {
                        if downloaded.saturating_sub(last_reported.get()) >= 1024 * 1024
                            || total == Some(downloaded)
                        {
                            last_reported.set(downloaded);
                            report("downloading".into(), downloaded, total, None);
                        }
                    },
                )
            } else {
                manager.install(&descriptor.id, &cancelled, |downloaded, total| {
                    if downloaded.saturating_sub(last_reported.get()) >= 1024 * 1024
                        || total == Some(downloaded)
                    {
                        last_reported.set(downloaded);
                        report("downloading".into(), downloaded, total, None);
                    }
                })
            };
            let (phase, error) = match result {
                Ok(_) => ("complete", None),
                Err(model_manager::ModelError::Cancelled) => ("cancelled", None),
                Err(error) => ("failed", Some(error.to_string())),
            };
            report(phase.into(), last_reported.get(), None, error);
            operations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&worker_id);
        }) {
        Ok(worker) => worker,
        Err(error) => {
            engine
                .model_operations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&model_id);
            return Err(error.to_string());
        }
    };
    let mut workers = engine
        .model_workers
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            let finished = workers.swap_remove(index);
            let _ = finished.join();
        } else {
            index += 1;
        }
    }
    workers.push(worker);
    Ok(())
}

fn stop_model_workers(engine: &RimvEngine) {
    for cancelled in engine
        .model_operations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .values()
    {
        cancelled.store(true, Ordering::Release);
    }
    let workers = std::mem::take(
        &mut *engine
            .model_workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    for worker in workers {
        let _ = worker.join();
    }
    engine
        .model_operations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

fn take_model_event(engine: &RimvEngine) -> Option<ModelProgress> {
    engine
        .model_events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv()
        .ok()
}

unsafe fn read_utf8<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("null string pointer".into());
    }
    // SAFETY: the ABI requires callers to pass a valid NUL-terminated string.
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| "input is not valid UTF-8".into())
}

#[unsafe(no_mangle)]
pub extern "C" fn rimv_api_version() -> u32 {
    API_VERSION
}

/// Creates the shared runtime and returns a JSON status envelope. On success,
/// `out_engine` receives an opaque handle; the host owns it until destroy.
///
/// # Safety
/// `config_json` must point to a valid NUL-terminated UTF-8 string for the
/// duration of this call. `out_engine` must be null or point to a valid,
/// writable, correctly aligned pointer slot for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_engine_create(
    config_json: *const c_char,
    out_engine: *mut *mut RimvEngine,
) -> *mut c_char {
    if !out_engine.is_null() {
        unsafe { *out_engine = ptr::null_mut() };
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        if out_engine.is_null() {
            return Err("out_engine is null".to_owned());
        }
        // SAFETY: caller provides a valid config string as required by this ABI.
        let input = unsafe { read_utf8(config_json) }?;
        let config: InitConfig =
            serde_json::from_str(input).map_err(|e| format!("invalid configuration: {e}"))?;
        if config.recordings_directory.as_os_str().is_empty() {
            return Err("recordings_directory must not be empty".into());
        }
        let runtime = EngineRuntime::new(EngineConfig {
            recordings_directory: config.recordings_directory.clone(),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
        let events = runtime.subscribe().map_err(|e| e.to_string())?;
        let (model_event_sender, model_events) = mpsc::sync_channel(128);
        let model_root = config.models_directory.unwrap_or_else(|| {
            config
                .recordings_directory
                .parent()
                .unwrap_or(&config.recordings_directory)
                .join("models")
        });
        let engine = Box::new(RimvEngine {
            runtime,
            events,
            recordings: RecordingLibrary::new(config.recordings_directory),
            models: ModelManager::new(model_root),
            model_operations: Arc::new(Mutex::new(HashMap::new())),
            model_workers: Mutex::new(Vec::new()),
            model_events: Mutex::new(model_events),
            model_event_sender,
        });
        // SAFETY: checked non-null above; ownership is transferred to caller.
        unsafe { *out_engine = Box::into_raw(engine) };
        Ok(serde_json::json!({"api_version":API_VERSION}))
    }));
    match result {
        Ok(result) => owned_string(envelope(result)),
        Err(_) => owned_string(envelope::<serde_json::Value>(Err(
            "Rust panic caught at FFI boundary".into(),
        ))),
    }
}

/// Executes one bounded request and returns a caller-owned UTF-8 JSON result.
/// Event polling blocks for at most 30 seconds and must run off the UI thread.
///
/// # Safety
/// `engine` must be a live handle returned by `rimv_engine_create` and must
/// not be destroyed while this call is running. `request_json` must point to
/// a valid NUL-terminated UTF-8 string for the duration of this call. The host
/// must serialize lifecycle and command calls as described by this module's
/// ABI contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_engine_request(
    engine: *mut RimvEngine,
    request_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if engine.is_null() {
            return Err("engine handle is null".to_owned());
        }
        // SAFETY: non-null handle originates from rimv_engine_create and remains owned by caller.
        let engine = unsafe { &*engine };
        // SAFETY: caller provides a valid NUL-terminated UTF-8 request.
        let input = unsafe { read_utf8(request_json) }?;
        let request: Request =
            serde_json::from_str(input).map_err(|e| format!("invalid request: {e}"))?;
        let value = match request {
            Request::GetState => {
                serde_json::to_value(engine.runtime.snapshot()).map_err(|e| e.to_string())?
            }
            Request::Send { command } => {
                serde_json::to_value(engine.runtime.send(command).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?
            }
            Request::PollEvent { timeout_ms } => {
                let wait = Duration::from_millis(timeout_ms.unwrap_or(0).min(30_000));
                if let Some(progress) = take_model_event(engine) {
                    serde_json::json!({"type":"model_progress","progress":progress})
                } else {
                    let deadline = std::time::Instant::now() + wait;
                    loop {
                        let remaining =
                            deadline.saturating_duration_since(std::time::Instant::now());
                        if remaining.is_zero() {
                            break serde_json::Value::Null;
                        }
                        match engine
                            .events
                            .recv_timeout(remaining.min(Duration::from_millis(50)))
                        {
                            Ok(event) => {
                                break serde_json::to_value(event).map_err(|e| e.to_string())?;
                            }
                            Err(engine_runtime::SubscriptionError::Timeout) => {
                                if let Some(progress) = take_model_event(engine) {
                                    break serde_json::json!({"type":"model_progress","progress":progress});
                                }
                            }
                            Err(error) => return Err(error.to_string()),
                        }
                    }
                }
            }
            Request::GetModels => {
                let operations = engine
                    .model_operations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let models = engine
                    .models
                    .catalog()
                    .iter()
                    .map(|descriptor| {
                        let state = if !engine_runtime::supports_asr_backend(&descriptor.backend) {
                            "unsupported"
                        } else if operations.contains_key(&descriptor.id) {
                            "downloading"
                        } else {
                            match engine.models.state(descriptor) {
                                model_manager::ModelState::Ready => "installed",
                                model_manager::ModelState::Incomplete => "incomplete",
                                model_manager::ModelState::Missing => "available",
                            }
                        };
                        serde_json::json!({"descriptor":descriptor,"state":state})
                    })
                    .collect::<Vec<_>>();
                serde_json::to_value(models).map_err(|e| e.to_string())?
            }
            Request::SelectModel { model_id, language } => {
                if engine.runtime.snapshot().status != engine_runtime::EngineStatus::Idle {
                    return Err("stop capture before changing the transcription model".into());
                }
                let descriptor = engine
                    .models
                    .descriptor(&model_id)
                    .map_err(|e| e.to_string())?;
                if !engine_runtime::supports_asr_backend(&descriptor.backend) {
                    return Err(
                        "this model backend is not available in the current RimV build".into(),
                    );
                }
                let path = engine
                    .models
                    .runtime_path(descriptor)
                    .map_err(|e| e.to_string())?;
                engine
                    .models
                    .validate_language(descriptor, language.as_deref())
                    .map_err(|e| e.to_string())?;
                engine
                    .runtime
                    .send(EngineCommand::SetTranscriptionBackend {
                        backend: descriptor.backend.clone(),
                    })
                    .map_err(|e| e.to_string())?;
                engine
                    .runtime
                    .send(EngineCommand::SetTranscriptionModel {
                        path: path.to_string_lossy().into_owned(),
                    })
                    .map_err(|e| e.to_string())?;
                engine
                    .runtime
                    .send(EngineCommand::SetTranscriptionLanguage { language })
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    engine
                        .runtime
                        .send(EngineCommand::SetTranscriptionEnabled { enabled: true })
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            }
            Request::SetLanguage { model_id, language } => {
                if engine.runtime.snapshot().status != engine_runtime::EngineStatus::Idle {
                    return Err("stop capture before changing the transcription language".into());
                }
                let descriptor = engine
                    .models
                    .descriptor(&model_id)
                    .map_err(|e| e.to_string())?;
                if !engine_runtime::supports_asr_backend(&descriptor.backend) {
                    return Err(
                        "this model backend is not available in the current RimV build".into(),
                    );
                }
                engine
                    .models
                    .validate_language(descriptor, language.as_deref())
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    engine
                        .runtime
                        .send(EngineCommand::SetTranscriptionLanguage { language })
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            }
            Request::ClearModel => {
                if engine.runtime.snapshot().status != engine_runtime::EngineStatus::Idle {
                    return Err("stop capture before clearing the transcription model".into());
                }
                engine
                    .runtime
                    .send(EngineCommand::ClearTranscriptionModel)
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    engine
                        .runtime
                        .send(EngineCommand::SetTranscriptionLanguage { language: None })
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            }
            Request::InstallModel { model_id } => {
                start_model_install(engine, model_id)?;
                serde_json::Value::Null
            }
            Request::CancelModelInstall { model_id } => {
                let operations = engine
                    .model_operations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let cancelled = operations
                    .get(&model_id)
                    .ok_or_else(|| "model installation is not running".to_owned())?;
                cancelled.store(true, Ordering::Release);
                serde_json::Value::Null
            }
            Request::GetRecordings => serde_json::to_value(
                engine
                    .recordings
                    .list(&engine.runtime)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
            Request::GetRecording { session_id } => serde_json::to_value(
                engine
                    .recordings
                    .get(&engine.runtime, &session_id)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
            Request::RenameRecording { session_id, title } => {
                engine
                    .recordings
                    .rename(&engine.runtime, &session_id, &title)
                    .map_err(|e| e.to_string())?;
                serde_json::Value::Null
            }
            Request::DeleteRecording { session_id } => {
                engine
                    .recordings
                    .delete(&engine.runtime, &session_id)
                    .map_err(|e| e.to_string())?;
                serde_json::Value::Null
            }
            Request::Export { session_id, format } => {
                let format = match format.as_str() {
                    "txt" => ExportFormat::Txt,
                    "json" => ExportFormat::Json,
                    _ => return Err("format must be txt or json".into()),
                };
                let bytes = engine
                    .recordings
                    .export_persisted(&session_id, format)
                    .map_err(|e| e.to_string())?;
                serde_json::json!({"content":String::from_utf8(bytes).map_err(|e| e.to_string())?})
            }
            Request::Shutdown => {
                stop_model_workers(engine);
                engine.runtime.shutdown().map_err(|e| e.to_string())?;
                serde_json::Value::Null
            }
        };
        Ok(value)
    }));
    match result {
        Ok(result) => owned_string(envelope(result)),
        Err(_) => owned_string(envelope::<serde_json::Value>(Err(
            "Rust panic caught at FFI boundary".into(),
        ))),
    }
}

/// Frees a string returned by this library. Null is accepted.
///
/// # Safety
/// A non-null `value` must be a pointer returned by this library and must not
/// have been freed previously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_string_free(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: pointer must have been returned by this library and not freed already.
        drop(unsafe { CString::from_raw(value) });
    }
}

/// Shuts down the runtime and frees the opaque handle. Null is accepted.
///
/// # Safety
/// A non-null `engine` must be a live handle returned by
/// `rimv_engine_create`. It must be uniquely owned by the caller, and no other
/// FFI call may be using it while it is destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rimv_engine_destroy(engine: *mut RimvEngine) {
    if !engine.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: handle must be uniquely owned and returned by create.
            let engine = unsafe { Box::from_raw(engine) };
            stop_model_workers(&engine);
            let _ = engine.runtime.shutdown();
        }));
    }
}
