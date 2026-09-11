use engine_runtime::{
    AsrBackendKind, EngineCommand, EngineConfig, EngineEvent, EngineRuntime, SubscriptionError,
};
use model_manager::{ModelManager, ModelState};
use std::{
    ffi::{CString, c_char},
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::Duration,
};

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
}

enum Action {
    Command(EngineCommand),
    Quit,
}
static COMMANDS: OnceLock<SyncSender<Action>> = OnceLock::new();
static MODELS: OnceLock<ModelManager> = OnceLock::new();
static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn command(code: u32, enabled: u8) {
    if let Some(model_id) = match code {
        6 => Some("whisper-tiny"),
        7 => Some("whisper-base"),
        8 => Some("whisper-small"),
        _ => None,
    } {
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
                let result = manager.install(model_id, &cancelled, |done, total| {
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
                });
                match result {
                    Ok(path) => {
                        let command = EngineCommand::SetTranscriptionModel {
                            path: path.to_string_lossy().into_owned(),
                        };
                        let selected = COMMANDS.get().is_some_and(|sender| {
                            sender.try_send(Action::Command(command)).is_ok()
                        });
                        if selected {
                            let _ = COMMANDS.get().map(|sender| {
                                sender.try_send(Action::Command(
                                    EngineCommand::SetTranscriptionEnabled { enabled: true },
                                ))
                            });
                        }
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
            });
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

fn show_error(error: &str) {
    eprintln!("{error}");
    if let Ok(text) = CString::new(error) {
        // Native code copies the string before returning and dispatches to AppKit.
        unsafe { rimv_menu_error(text.as_ptr()) };
    }
}

fn update(snapshot: &engine_runtime::EngineSnapshot) {
    if let Ok(json) = serde_json::to_string(snapshot)
        && let Ok(text) = CString::new(json)
    {
        // Only serialized control state crosses this boundary, never audio data.
        unsafe { rimv_menu_update(text.as_ptr()) };
    }
}

struct Options {
    directory: PathBuf,
    profile: Option<String>,
    seconds: u64,
    self_test: bool,
}
impl Options {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut options = Self {
            directory: PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
                .join("Library/Application Support/rimv/recordings"),
            profile: None,
            seconds: 30,
            self_test: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--self-test" => options.self_test = true,
                "--recordings" => {
                    options.directory = args.next().ok_or("missing recordings path")?.into()
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
    let directory = std::env::current_dir()?.join(&options.directory);
    std::fs::create_dir_all(&directory)?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
    let model_root = std::env::var_os("RIMV_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| ModelManager::default_root(&home));
    let models = ModelManager::new(model_root);
    let selected = models
        .catalog()
        .iter()
        .find(|model| model.backend == "whisper" && models.state(model) == ModelState::Ready)
        .and_then(|model| model.files.first().map(|file| models.path(model, file)));
    let mut config = EngineConfig {
        recordings_directory: directory.clone(),
        ..Default::default()
    };
    // This UI still manages the legacy Whisper catalog. Generic Parakeet
    // model selection is intentionally deferred to the model-manager phase.
    config.transcription.backend = AsrBackendKind::Whisper;
    if let Some(selected) = selected.filter(|path| path.is_file()) {
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
    let root = CString::new(directory.to_string_lossy().as_bytes())?;
    let initial = CString::new(serde_json::to_string(&engine.snapshot())?)?;
    // AppKit is created and run on the process main thread.
    unsafe { rimv_menu_create(root.as_ptr(), initial.as_ptr(), command) };
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
                    Action::Quit => {
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
                    Ok(EngineEvent::TranscriptPartial { .. })
                    | Ok(EngineEvent::TranscriptFinal { .. })
                    | Ok(EngineEvent::TranscriptUpdate { .. }) => {}
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
