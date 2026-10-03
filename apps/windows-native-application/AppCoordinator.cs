using System.Text.Json;

namespace RimV.Windows.Application;

/// <summary>Application-level orchestration of shared-core state and user preferences.</summary>
public sealed class AppCoordinator
{
    private readonly string _dataDirectory;
    private readonly ISharedCoreClientFactory _coreFactory;
    private readonly IUserPreferencesStore _preferences;
    private readonly IUiDispatcher _dispatcher;
    private readonly IAppLog _log;
    private readonly CancellationTokenSource _shutdown = new();
    private readonly SemaphoreSlim _operations = new(1, 1);
    private ISharedCoreClient? _core;
    private Task? _eventPump;
    private string? _activeSessionId;
    private long _transcriptRevision;

    public event EventHandler? Changed;
    public event Action<CoreEvent>? CoreEventReceived;
    public event Action<ModelProgress>? ModelProgressReceived;
    public CoreSnapshot Snapshot { get; private set; } = new();
    public IReadOnlyList<ModelRecord> Models { get; private set; } = [];
    public IReadOnlyList<AudioInputDevice> InputDevices { get; private set; } = [];
    public IReadOnlyList<RecordingSummary> Recordings { get; private set; } = [];
    public string? SelectedModelId { get; private set; }
    public string? SelectedLanguage { get; private set; }
    public string SelectedSource { get; private set; } = "microphone";
    public string? ErrorMessage { get; private set; }
    public string? LastErrorDetail { get; private set; }
    public string ThemeName { get; private set; } = "system";
    public string? SelectedMicrophoneDeviceId { get; private set; }
    public bool IsPreparingToListen { get; private set; }
    public string RecordingsDirectory => Path.Combine(_dataDirectory, "recordings");
    public string ModelsDirectory => Path.Combine(_dataDirectory, "models");
    public bool IsCoreAvailable => _core is not null;
    public long TranscriptRevision => Interlocked.Read(ref _transcriptRevision);

    public AppCoordinator(
        string dataDirectory,
        ISharedCoreClientFactory coreFactory,
        IUserPreferencesStore preferences,
        IUiDispatcher dispatcher,
        IAppLog? log = null)
    {
        _dataDirectory = dataDirectory;
        _coreFactory = coreFactory;
        _preferences = preferences;
        _dispatcher = dispatcher;
        _log = log ?? NullAppLog.Instance;
    }

    public async Task InitializeAsync()
    {
        _log.Info("coordinator.initialize");
        Directory.CreateDirectory(_dataDirectory);
        try
        {
            UserPreferences preferences = await _preferences.LoadAsync(_shutdown.Token) ?? UserPreferences.Default;
            SelectedModelId = preferences.SelectedModelId;
            SelectedLanguage = preferences.Language;
            SelectedSource = preferences.Source;
            SelectedMicrophoneDeviceId = preferences.MicrophoneDeviceId;
            ThemeName = NormalizeTheme(preferences.Theme);
        }
        catch (Exception error) when (error is not OperationCanceledException)
        {
            _log.Error("preferences.load_failed", error);
            LastErrorDetail = error.ToString();
            ErrorMessage = "RimV couldn't load your preferences. It will use the defaults.";
        }

        string recordings = RecordingsDirectory;
        string models = Path.Combine(_dataDirectory, "models");
        Directory.CreateDirectory(recordings);
        Directory.CreateDirectory(models);
        _core = _coreFactory.Create(recordings, models);
        ApplySnapshot(await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
        try
        {
            InputDevices = await _core.RequestAsync<List<AudioInputDevice>>(new { type = "list_input_devices" }, _shutdown.Token);
        }
        catch (Exception error)
        {
            _log.Error("audio.input_devices_unavailable", error);
            InputDevices = [];
        }
        if (SelectedMicrophoneDeviceId is not null && !InputDevices.Any(device => device.Id == SelectedMicrophoneDeviceId))
            SelectedMicrophoneDeviceId = null;
        if (Snapshot.Microphone.Configured.DeviceId != SelectedMicrophoneDeviceId)
        {
            await _core.RequestAsync<CoreSnapshot>(new
            {
                type = "send",
                command = new { type = "set_microphone_device", device_id = SelectedMicrophoneDeviceId },
            }, _shutdown.Token);
            ApplySnapshot(await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
        }
        Models = await _core.RequestAsync<List<ModelRecord>>(new { type = "get_models" }, _shutdown.Token);
        Recordings = await _core.RequestAsync<List<RecordingSummary>>(new { type = "get_recordings" }, _shutdown.Token);

        if (SelectedModelId is not null)
        {
            ModelRecord? savedModel = Models.FirstOrDefault(model => model.Descriptor.Id == SelectedModelId);
            if (savedModel?.State == "installed")
            {
                if (SelectedLanguage is not null && !savedModel.Descriptor.Languages.Contains(SelectedLanguage))
                    SelectedLanguage = null;
                await GuardAsync(async () =>
                {
                    await _core.RequestAsync<JsonElement>(new
                    {
                        type = "select_model",
                        model_id = savedModel.Descriptor.Id,
                        language = SelectedLanguage,
                    }, _shutdown.Token);
                    ApplySnapshot(await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
                    Models = await _core.RequestAsync<List<ModelRecord>>(new { type = "get_models" }, _shutdown.Token);
                });
            }
            else
            {
                SelectedModelId = null;
                SelectedLanguage = null;
            }
        }

        await ApplySourcePreferenceAsync();
        StartEventPump();
        _log.Info("coordinator.ready", $"models={Models.Count}; recordings={Recordings.Count}");
        NotifyChanged();
    }

    private async Task ApplySourcePreferenceAsync()
    {
        if (_core is null) return;
        SelectedSource = NativeWindowsPolicy.ResolveSource(
            SelectedSource,
            Snapshot.Capabilities.MicrophoneCapture,
            Snapshot.Capabilities.SystemAudioCapture);
        await SetSourceAsync(SelectedSource, persist: false);
    }

    public void SetTheme(string theme)
    {
        _log.Info("preferences.theme_changed");
        ThemeName = NormalizeTheme(theme);
        _ = GuardAsync(SavePreferencesAsync);
        NotifyChanged();
    }

    public async Task RefreshStateAsync()
    {
        if (_core is null) return;
        ApplySnapshot(await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
        NotifyChanged();
    }

    public async Task RefreshModelsAsync()
    {
        if (_core is null) return;
        Models = await _core.RequestAsync<List<ModelRecord>>(new { type = "get_models" }, _shutdown.Token);
        NotifyChanged();
    }

    public async Task RefreshRecordingsAsync()
    {
        if (_core is null) return;
        Recordings = await _core.RequestAsync<List<RecordingSummary>>(new { type = "get_recordings" }, _shutdown.Token);
        NotifyChanged();
    }

    public Task<RecordingDetails> GetRecordingAsync(string id) =>
        RequireCore().RequestAsync<RecordingDetails>(new { type = "get_recording", session_id = id }, _shutdown.Token);

    public Task<LiveTranscriptSnapshot> GetLiveTranscriptSnapshotAsync() =>
        RequireCore().RequestAsync<LiveTranscriptSnapshot>(new { type = "get_transcript_state" }, _shutdown.Token);

    public async Task StartOrStopAsync()
    {
        await GuardAsync(async () =>
        {
            string? command = NativeWindowsPolicy.ListeningCommand(Snapshot.Status);
            if (command is null) return;
            if (command == "start_capture" && Snapshot.Transcription.Enabled)
            {
                IsPreparingToListen = true;
                NotifyChanged();
                try { await EnsureVadInstalledAsync(); }
                finally { IsPreparingToListen = false; NotifyChanged(); }
            }
            ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new
            {
                type = "send",
                command = new { type = command },
            }, _shutdown.Token));
            await RefreshRecordingsAsync();
            ClearError();
            NotifyChanged();
        });
    }

    public async Task SelectMicrophoneDeviceAsync(string? deviceId)
    {
        if (deviceId is not null && !InputDevices.Any(device => device.Id == deviceId)) return;
        await GuardAsync(async () =>
        {
            await RequireCore().RequestAsync<CoreSnapshot>(new
            {
                type = "send",
                command = new { type = "set_microphone_device", device_id = deviceId },
            }, _shutdown.Token);
            SelectedMicrophoneDeviceId = deviceId;
            ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
            await SavePreferencesAsync();
            ClearError();
            NotifyChanged();
        });
    }

    private async Task EnsureVadInstalledAsync()
    {
        const string vadId = "silero-vad";
        ModelRecord? vad = Models.FirstOrDefault(model => model.Descriptor.Id == vadId);
        if (vad is null)
            throw new CoreRequestException("RimV can't find its Silero speech detector in the shared model catalog. Reinstall or repair the application.");
        if (vad.State == "installed") return;
        if (vad.State == "unsupported")
            throw new CoreRequestException("This RimV build does not include Silero VAD support. Install a Windows build with the shared speech runtime enabled.");

        var completion = new TaskCompletionSource<ModelProgress>(TaskCreationOptions.RunContinuationsAsynchronously);
        void OnProgress(ModelProgress progress)
        {
            if (progress.ModelId == vadId && progress.Phase is "complete" or "failed" or "cancelled")
                completion.TrySetResult(progress);
        }
        ModelProgressReceived += OnProgress;
        try
        {
            if (vad.State != "downloading")
                await RequireCore().RequestAsync<JsonElement>(new { type = "install_model", model_id = vadId }, _shutdown.Token);
            ModelProgress result = await completion.Task.WaitAsync(TimeSpan.FromMinutes(15), _shutdown.Token);
            if (result.Phase != "complete")
                throw new CoreRequestException(result.Phase == "cancelled"
                    ? "Installing the required speech detector was cancelled. Open Model Manager and retry, then start listening again."
                    : $"RimV couldn't install the required speech detector. Check your internet connection and available disk space, then retry in Model Manager. {result.Error}");
            await RefreshModelsAsync();
            if (Models.FirstOrDefault(model => model.Descriptor.Id == vadId)?.State != "installed")
                throw new CoreRequestException("The speech detector download finished, but its model file is missing. Open Model Manager and retry the installation.");
        }
        finally { ModelProgressReceived -= OnProgress; }
    }

    public async Task SetSourceAsync(string source, bool persist = true)
    {
        if (Snapshot.Status is "recording" or "starting" or "stopping") return;
        if ((source is "system" or "both") && !Snapshot.Capabilities.SystemAudioCapture)
        {
            SetError("System audio capture isn't available on this Windows setup.", "The shared capture backend reports that loopback capture is unavailable.");
            return;
        }
        await GuardAsync(async () =>
        {
            bool microphone = source is "microphone" or "both";
            bool system = source is "system" or "both";
            await SendCommandAsync("set_microphone_enabled", microphone);
            await SendCommandAsync("set_system_audio_enabled", system);
            SelectedSource = source;
            ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
            if (persist) await SavePreferencesAsync();
            ClearError();
            NotifyChanged();
        });
    }

    public async Task SelectModelAsync(string modelId)
    {
        await GuardAsync(async () =>
        {
            ModelRecord? selected = Models.FirstOrDefault(item => item.Descriptor.Id == modelId);
            if (selected is null || !NativeWindowsPolicy.CanSelectModel(selected.State, Snapshot.Status))
                throw new CoreRequestException("Select an installed model while RimV is idle.");
            string? language = SelectedLanguage is not null && selected.Descriptor.Languages.Contains(SelectedLanguage)
                ? SelectedLanguage
                : null;
            await RequireCore().RequestAsync<JsonElement>(new { type = "select_model", model_id = modelId, language }, _shutdown.Token);
            SelectedModelId = modelId;
            SelectedLanguage = language;
            ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
            Models = await RequireCore().RequestAsync<List<ModelRecord>>(new { type = "get_models" }, _shutdown.Token);
            await SavePreferencesAsync();
            ClearError();
            NotifyChanged();
        });
    }

    public async Task SelectLanguageAsync(string? language)
    {
        if (SelectedModelId is null) return;
        await GuardAsync(async () =>
        {
            await RequireCore().RequestAsync<JsonElement>(new
            {
                type = "set_language",
                model_id = SelectedModelId,
                language,
            }, _shutdown.Token);
            SelectedLanguage = language;
            ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
            await SavePreferencesAsync();
            ClearError();
            NotifyChanged();
        });
    }

    public async Task InstallModelAsync(string id) => await GuardAsync(async () =>
    {
        ModelRecord? model = Models.FirstOrDefault(item => item.Descriptor.Id == id);
        if (model is null || !NativeWindowsPolicy.CanInstallModel(model.State))
            throw new CoreRequestException("This model is unavailable for installation.");
        await RequireCore().RequestAsync<JsonElement>(new { type = "install_model", model_id = id }, _shutdown.Token);
        await RefreshModelsAsync();
        ClearError();
    });

    public async Task CancelModelInstallAsync(string id) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "cancel_model_install", model_id = id }, _shutdown.Token);
    });

    public async Task RemoveModelAsync(string id) => await GuardAsync(async () =>
    {
        if (Snapshot.Status is "starting" or "recording" or "stopping")
            throw new CoreRequestException("Stop listening before removing a model.");
        await RequireCore().RequestAsync<JsonElement>(new { type = "remove_model", model_id = id }, _shutdown.Token);
        await RefreshModelsAsync();
        ApplySnapshot(await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" }, _shutdown.Token));
        if (SelectedModelId == id)
        {
            SelectedModelId = null;
            SelectedLanguage = null;
            await SavePreferencesAsync();
        }
        ClearError();
        NotifyChanged();
    });

    public async Task RenameRecordingAsync(string id, string title) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "rename_recording", session_id = id, title }, _shutdown.Token);
        await RefreshRecordingsAsync();
        ClearError();
    });

    public async Task DeleteRecordingAsync(string id) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "delete_recording", session_id = id }, _shutdown.Token);
        await RefreshRecordingsAsync();
        ClearError();
    });

    public async Task<string> ExportRecordingAsync(string id, string format) =>
        (await RequireCore().RequestAsync<ExportPayload>(new { type = "export", session_id = id, format }, _shutdown.Token)).Content;

    private async Task<CoreSnapshot> SendCommandAsync(string command, bool enabled) =>
        await RequireCore().RequestAsync<CoreSnapshot>(new
        {
            type = "send",
            command = new { type = command, enabled },
        }, _shutdown.Token);

    private void StartEventPump()
    {
        if (_core is null || _eventPump is not null) return;
        ISharedCoreClient core = _core;
        _eventPump = Task.Run(async () =>
        {
            while (!_shutdown.IsCancellationRequested)
            {
                try
                {
                    CoreEvent? item = await core.PollEventAsync(_shutdown.Token).ConfigureAwait(false);
                    if (item is not null) _dispatcher.TryEnqueue(() => ApplyEvent(item));
                }
                catch (OperationCanceledException) when (_shutdown.IsCancellationRequested)
                {
                    break;
                }
                catch (Exception error)
                {
                    _log.Error("core.event_pump_failed", error);
                    _dispatcher.TryEnqueue(() => SetError("RimV lost contact with the shared engine. Restart the app to reconnect.", error.ToString()));
                    try { await Task.Delay(500, _shutdown.Token).ConfigureAwait(false); }
                    catch (OperationCanceledException) when (_shutdown.IsCancellationRequested) { break; }
                }
            }
        });
    }

    private void ApplyEvent(CoreEvent item)
    {
        try
        {
            _log.Info("core.event", item.Type);
            switch (item.Type)
            {
                case "snapshot":
                    CoreSnapshot? incoming = item.Payload.GetProperty("snapshot").Deserialize<CoreSnapshot>();
                    if (incoming is not null && ApplySnapshot(incoming) && _activeSessionId != Snapshot.Session?.Id)
                    {
                        _activeSessionId = Snapshot.Session?.Id;
                        _ = GuardAsync(RefreshRecordingsAsync);
                    }
                    if (incoming is not null && incoming.Revision >= Snapshot.Revision) NotifyChanged();
                    break;
                case "model_progress":
                    ModelProgressReceived?.Invoke(item.Payload.GetProperty("progress").Deserialize<ModelProgress>() ?? new());
                    break;
                case "transcript_update":
                    Interlocked.Increment(ref _transcriptRevision);
                    _log.Info("transcript.update_received");
                    break;
                case "error":
                case "transcription_error":
                    JsonElement error = item.Payload.TryGetProperty("error", out JsonElement payloadError)
                        ? payloadError : default;
                    string message = error.ValueKind == JsonValueKind.Object && error.TryGetProperty("message", out JsonElement text)
                        ? text.GetString() ?? "RimV encountered a recoverable error."
                        : "RimV encountered a recoverable error.";
                    string userMessage = error.ValueKind == JsonValueKind.Object && error.TryGetProperty("user_message", out JsonElement userText)
                        ? userText.GetString() ?? "" : "";
                    SetError(string.IsNullOrWhiteSpace(userMessage) ? ToFriendlyError(message) : userMessage, message);
                    break;
            }
            CoreEventReceived?.Invoke(item);
        }
        catch (Exception error)
        {
            _log.Error("core.event_apply_failed", error);
            SetError("RimV received an update it couldn't display. The recording can continue.", error.ToString());
        }
    }

    private static string NormalizeTheme(string theme) => theme is "light" or "dark" ? theme : "system";

    private static string ToFriendlyError(string message)
    {
        string lower = message.ToLowerInvariant();
        if (lower.Contains("silero") || lower.Contains("vad") || lower.Contains("speech detector"))
            return "RimV couldn't prepare its speech detector. Check your internet connection and free disk space, install Silero VAD in Model Manager, then try again.";
        if (lower.Contains("permission")) return "Windows blocked audio access. Allow microphone access in Windows Settings, then try again.";
        if (lower.Contains("device") || lower.Contains("endpoint")) return "An audio device isn't available. Connect or select an audio input, then try again.";
        return message;
    }

    private async Task GuardAsync(Func<Task> action)
    {
        bool acquired = false;
        try
        {
            await _operations.WaitAsync(_shutdown.Token);
            acquired = true;
            await action();
        }
        catch (OperationCanceledException) when (_shutdown.IsCancellationRequested) { }
        catch (Exception error)
        {
            _log.Error("coordinator.operation_failed", error);
            string message = error is CoreRequestException ? ToFriendlyError(error.Message) : "RimV couldn't complete that action. Check the shared engine and try again.";
            SetError(message, error.ToString());
        }
        finally
        {
            if (acquired) _operations.Release();
        }
    }

    public void ReportError(Exception error)
    {
        _log.Error("coordinator.startup_error", error);
        SetError(
            error is DllNotFoundException or EntryPointNotFoundException
            ? "RimV's shared engine is missing. Rebuild the app with the matching Rust core DLL."
            : "RimV couldn't start the shared engine. Check the app's local data folder and try again.",
            error.ToString());
    }

    public void ClearError()
    {
        ErrorMessage = null;
        LastErrorDetail = null;
        NotifyChanged();
    }

    private void SetError(string message, string detail)
    {
        _log.Info("coordinator.error_state_changed");
        ErrorMessage = message;
        LastErrorDetail = detail;
        NotifyChanged();
    }

    private bool ApplySnapshot(CoreSnapshot snapshot)
    {
        if (!NativeWindowsPolicy.ShouldAcceptSnapshot(Snapshot.Revision, snapshot.Revision))
        {
            _log.Info("core.snapshot_stale_ignored", $"current={Snapshot.Revision}; incoming={snapshot.Revision}");
            return false;
        }
        Snapshot = snapshot;
        return true;
    }

    private async Task SavePreferencesAsync() => await _preferences.SaveAsync(
        new UserPreferences(SelectedModelId, SelectedLanguage, SelectedSource, ThemeName)
        {
            MicrophoneDeviceId = SelectedMicrophoneDeviceId,
        }, _shutdown.Token);

    private void NotifyChanged()
    {
        void Raise() => Changed?.Invoke(this, EventArgs.Empty);
        if (_dispatcher.HasThreadAccess) Raise();
        else _dispatcher.TryEnqueue(Raise);
    }

    private ISharedCoreClient RequireCore() => _core ?? throw new InvalidOperationException(
        "The shared RimV engine is unavailable. Rebuild the app with rimv_core_ffi.dll.");

    public async Task ShutdownAsync()
    {
        _log.Info("coordinator.shutdown");
        _shutdown.Cancel();
        if (_eventPump is not null)
        {
            try { await _eventPump.ConfigureAwait(false); }
            catch (OperationCanceledException) { }
        }
        await _operations.WaitAsync().ConfigureAwait(false);
        try
        {
            if (_core is not null)
            {
                ISharedCoreClient core = _core;
                _core = null;
                await Task.Run(core.Shutdown).ConfigureAwait(false);
            }
        }
        finally { _operations.Release(); }
        _shutdown.Dispose();
        _operations.Dispose();
    }
}
