using Microsoft.UI.Xaml;
using System.Text.Json;
using Windows.UI.ViewManagement;

namespace RimV.Windows;

internal sealed class AppCoordinator
{
    private readonly string _dataDirectory = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "RimV");
    private readonly List<WeakReference<FrameworkElement>> _themeRoots = [];
    private readonly CancellationTokenSource _shutdown = new();
    private readonly AccessibilitySettings _accessibilitySettings = new();
    private CoreClient? _core;
    private Task? _eventPump;
    private string? _activeSessionId;
    private long _transcriptRevision;
    private bool _accessibilitySubscribed;

    public event EventHandler? Changed;
    public event Action<CoreEvent>? CoreEventReceived;
    public event Action<ModelProgress>? ModelProgressReceived;
    public CoreSnapshot Snapshot { get; private set; } = new();
    public IReadOnlyList<ModelRecord> Models { get; private set; } = [];
    public IReadOnlyList<RecordingSummary> Recordings { get; private set; } = [];
    public string? SelectedModelId { get; private set; }
    public string? SelectedLanguage { get; private set; }
    public string SelectedSource { get; private set; } = "microphone";
    public string? ErrorMessage { get; private set; }
    public string? LastErrorDetail { get; private set; }
    public string? RecordingsDirectory => _core is null ? null : Path.Combine(_dataDirectory, "recordings");
    public bool IsCoreAvailable => _core is not null;
    public long TranscriptRevision => Interlocked.Read(ref _transcriptRevision);

    public async Task InitializeAsync()
    {
        Directory.CreateDirectory(_dataDirectory);
        await LoadPreferencesAsync();
        string recordings = Path.Combine(_dataDirectory, "recordings");
        string models = Path.Combine(_dataDirectory, "models");
        Directory.CreateDirectory(recordings);
        Directory.CreateDirectory(models);
        _core = CoreClient.Create(recordings, models);
        Snapshot = await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" });
        Models = await _core.RequestAsync<List<ModelRecord>>(new { type = "get_models" });
        Recordings = await _core.RequestAsync<List<RecordingSummary>>(new { type = "get_recordings" });

        SelectedModelId ??= FindSelectedModelFromRuntime();
        if (SelectedModelId is not null)
        {
            ModelRecord? savedModel = Models.FirstOrDefault(model => model.Descriptor.Id == SelectedModelId);
            if (savedModel?.State == "installed")
            {
                await GuardAsync(async () =>
                {
                    await _core.RequestAsync<JsonElement>(new
                    {
                        type = "select_model",
                        model_id = savedModel.Descriptor.Id,
                        language = SelectedLanguage,
                    });
                    Snapshot = await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" });
                });
            }
            else
            {
                SelectedModelId = null;
            }
        }

        await ApplySourcePreferenceAsync();
        StartEventPump();
        NotifyChanged();
    }

    private string? FindSelectedModelFromRuntime()
    {
        string? path = Snapshot.Transcription.ModelPath;
        if (string.IsNullOrWhiteSpace(path)) return null;
        foreach (ModelRecord model in Models)
        {
            string modelPath = model.Descriptor.Backend == "parakeet"
                ? Path.Combine(_dataDirectory, "models", model.Descriptor.StorageDirectory)
                : Path.Combine(_dataDirectory, "models", model.Descriptor.StorageDirectory,
                    model.Descriptor.Files.FirstOrDefault()?.Filename ?? "");
            if (string.Equals(Path.GetFullPath(path), Path.GetFullPath(modelPath), StringComparison.OrdinalIgnoreCase))
                return model.Descriptor.Id;
        }
        return null;
    }

    private async Task ApplySourcePreferenceAsync()
    {
        if (_core is null) return;
        string source = SelectedSource switch
        {
            "system" when Snapshot.Capabilities.SystemAudioCapture => "system",
            "both" when Snapshot.Capabilities.SystemAudioCapture && Snapshot.Capabilities.MicrophoneCapture => "both",
            "microphone" when Snapshot.Capabilities.MicrophoneCapture => "microphone",
            _ => Snapshot.Capabilities.MicrophoneCapture ? "microphone" : "system",
        };
        SelectedSource = source;
        await SetSourceAsync(source, persist: false);
    }

    private async Task LoadPreferencesAsync()
    {
        try
        {
            string path = Path.Combine(_dataDirectory, "preferences.json");
            if (!File.Exists(path)) return;
            UserPreferences preferences = JsonSerializer.Deserialize<UserPreferences>(await File.ReadAllTextAsync(path)) ?? new();
            SelectedModelId = preferences.SelectedModelId;
            SelectedLanguage = preferences.Language;
            SelectedSource = preferences.Source;
            ThemeName = preferences.Theme;
        }
        catch (Exception error)
        {
            LastErrorDetail = error.ToString();
            ErrorMessage = "RimV couldn't load your preferences. It will use the defaults.";
        }
    }

    private async Task SavePreferencesAsync()
    {
        string path = Path.Combine(_dataDirectory, "preferences.json");
        string temporary = path + ".tmp";
        var preferences = new UserPreferences(SelectedModelId, SelectedLanguage, SelectedSource, ThemeName);
        await File.WriteAllTextAsync(temporary, JsonSerializer.Serialize(preferences));
        File.Move(temporary, path, overwrite: true);
    }

    public string ThemeName { get; private set; } = "system";

    public void RegisterThemeRoot(FrameworkElement root)
    {
        if (!_accessibilitySubscribed)
        {
            _accessibilitySettings.HighContrastChanged += AccessibilitySettings_HighContrastChanged;
            _accessibilitySubscribed = true;
        }
        _themeRoots.RemoveAll(item => !item.TryGetTarget(out _));
        if (!_themeRoots.Any(item => item.TryGetTarget(out FrameworkElement? target) && ReferenceEquals(target, root)))
            _themeRoots.Add(new WeakReference<FrameworkElement>(root));
        ApplyTheme(root);
    }

    public void UnregisterThemeRoot(FrameworkElement root) =>
        _themeRoots.RemoveAll(item => !item.TryGetTarget(out FrameworkElement? target) || ReferenceEquals(target, root));

    public void SetTheme(string theme)
    {
        ThemeName = theme is "light" or "dark" ? theme : "system";
        foreach (WeakReference<FrameworkElement> item in _themeRoots.ToArray())
            if (item.TryGetTarget(out FrameworkElement? root)) ApplyTheme(root);
        _ = GuardAsync(SavePreferencesAsync);
    }

    private void ApplyTheme(FrameworkElement root)
    {
        bool highContrast = false;
        try { highContrast = _accessibilitySettings.HighContrast; }
        catch { /* If the system accessibility API is unavailable, use the system theme. */ }
        root.RequestedTheme = highContrast ? ElementTheme.Default : ThemeName switch
        {
            "light" => ElementTheme.Light,
            "dark" => ElementTheme.Dark,
            _ => ElementTheme.Default,
        };
    }

    private void AccessibilitySettings_HighContrastChanged(AccessibilitySettings sender, object args)
    {
        foreach (WeakReference<FrameworkElement> item in _themeRoots.ToArray())
            if (item.TryGetTarget(out FrameworkElement? root)) ApplyTheme(root);
    }

    public async Task RefreshStateAsync()
    {
        if (_core is null) return;
        Snapshot = await _core.RequestAsync<CoreSnapshot>(new { type = "get_state" });
        NotifyChanged();
    }

    public async Task RefreshModelsAsync()
    {
        if (_core is null) return;
        Models = await _core.RequestAsync<List<ModelRecord>>(new { type = "get_models" });
        NotifyChanged();
    }

    public async Task RefreshRecordingsAsync()
    {
        if (_core is null) return;
        Recordings = await _core.RequestAsync<List<RecordingSummary>>(new { type = "get_recordings" });
        NotifyChanged();
    }

    public Task<RecordingDetails> GetRecordingAsync(string id) =>
        RequireCore().RequestAsync<RecordingDetails>(new { type = "get_recording", session_id = id });

    public async Task StartOrStopAsync()
    {
        await GuardAsync(async () =>
        {
            string command = Snapshot.Status is "recording" or "starting" ? "stop_capture" : "start_capture";
            Snapshot = await RequireCore().RequestAsync<CoreSnapshot>(new
            {
                type = "send",
                command = new { type = command },
            });
            await RefreshRecordingsAsync();
            ClearError();
            NotifyChanged();
        });
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
            Snapshot = await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" });
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
            string? language = SelectedLanguage is not null && selected?.Descriptor.Languages.Contains(SelectedLanguage) == true
                ? SelectedLanguage
                : null;
            await RequireCore().RequestAsync<JsonElement>(new
            {
                type = "select_model",
                model_id = modelId,
                language,
            });
            SelectedModelId = modelId;
            SelectedLanguage = language;
            Snapshot = await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" });
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
            });
            SelectedLanguage = language;
            Snapshot = await RequireCore().RequestAsync<CoreSnapshot>(new { type = "get_state" });
            await SavePreferencesAsync();
            ClearError();
            NotifyChanged();
        });
    }

    public async Task InstallModelAsync(string id) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "install_model", model_id = id });
        await RefreshModelsAsync();
        ClearError();
    });

    public async Task CancelModelInstallAsync(string id) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "cancel_model_install", model_id = id });
    });

    public async Task RenameRecordingAsync(string id, string title) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "rename_recording", session_id = id, title });
        await RefreshRecordingsAsync();
        ClearError();
    });

    public async Task DeleteRecordingAsync(string id) => await GuardAsync(async () =>
    {
        await RequireCore().RequestAsync<JsonElement>(new { type = "delete_recording", session_id = id });
        await RefreshRecordingsAsync();
        ClearError();
    });

    public async Task<string> ExportRecordingAsync(string id, string format) =>
        (await RequireCore().RequestAsync<ExportPayload>(new { type = "export", session_id = id, format })).Content;

    private async Task<CoreSnapshot> SendCommandAsync(string command, bool enabled)
    {
        return await RequireCore().RequestAsync<CoreSnapshot>(new
        {
            type = "send",
            command = new { type = command, enabled },
        });
    }

    private void StartEventPump()
    {
        if (_core is null || _eventPump is not null) return;
        _eventPump = Task.Run(async () =>
        {
            while (!_shutdown.IsCancellationRequested)
            {
                try
                {
                    CoreEvent? item = await _core.PollEventAsync(_shutdown.Token);
                    if (item is not null) App.CurrentApp.UiQueue.TryEnqueue(() => ApplyEvent(item));
                }
                catch (OperationCanceledException) { break; }
                catch (Exception error)
                {
                    App.CurrentApp.UiQueue.TryEnqueue(() => SetError("RimV lost contact with the shared engine. Restart the app to reconnect.", error.ToString()));
                    await Task.Delay(500, _shutdown.Token).ContinueWith(_ => { });
                }
            }
        });
    }

    private void ApplyEvent(CoreEvent item)
    {
        try
        {
            switch (item.Type)
            {
                case "snapshot":
                    Snapshot = item.Payload.GetProperty("snapshot").Deserialize<CoreSnapshot>() ?? Snapshot;
                    if (_activeSessionId != Snapshot.Session?.Id)
                    {
                        _activeSessionId = Snapshot.Session?.Id;
                        _ = GuardAsync(RefreshRecordingsAsync);
                    }
                    NotifyChanged();
                    break;
                case "model_progress":
                    ModelProgressReceived?.Invoke(item.Payload.GetProperty("progress").Deserialize<ModelProgress>() ?? new());
                    break;
                case "transcript_update":
                    Interlocked.Increment(ref _transcriptRevision);
                    break;
                case "error":
                case "transcription_error":
                    string message = item.Payload.TryGetProperty("error", out JsonElement error) && error.TryGetProperty("message", out JsonElement text)
                        ? text.GetString() ?? "RimV encountered a recoverable error."
                        : "RimV encountered a recoverable error.";
                    SetError(ToFriendlyError(message), message);
                    break;
            }
            CoreEventReceived?.Invoke(item);
        }
        catch (Exception error)
        {
            SetError("RimV received an update it couldn't display. The recording can continue.", error.ToString());
        }
    }

    private static string ToFriendlyError(string message)
    {
        string lower = message.ToLowerInvariant();
        if (lower.Contains("permission")) return "Windows blocked audio access. Allow microphone access in Windows Settings, then try again.";
        if (lower.Contains("device") || lower.Contains("endpoint")) return "An audio device isn't available. Connect or select an audio input, then try again.";
        return message;
    }

    private async Task GuardAsync(Func<Task> action)
    {
        try { await action(); }
        catch (Exception error)
        {
            string message = error is CoreRequestException ? ToFriendlyError(error.Message) : "RimV couldn't complete that action. Check the shared engine and try again.";
            SetError(message, error.ToString());
        }
    }

    public void ReportError(Exception error) => SetError(
        error is DllNotFoundException or EntryPointNotFoundException
            ? "RimV's shared engine is missing. Rebuild the app with the matching Rust core DLL."
            : "RimV couldn't start the shared engine. Check the app's local data folder and try again.",
        error.ToString());

    private void SetError(string message, string detail)
    {
        ErrorMessage = message;
        LastErrorDetail = detail;
        NotifyChanged();
    }

    public void ClearError()
    {
        ErrorMessage = null;
        LastErrorDetail = null;
    }

    private void NotifyChanged()
    {
        if (App.CurrentApp.UiQueue.HasThreadAccess) Changed?.Invoke(this, EventArgs.Empty);
        else App.CurrentApp.UiQueue.TryEnqueue(() => Changed?.Invoke(this, EventArgs.Empty));
    }

    private CoreClient RequireCore() => _core ?? throw new InvalidOperationException("The shared RimV engine is unavailable. Rebuild the app with rimv_core_ffi.dll.");

    public async Task ShutdownAsync()
    {
        if (_accessibilitySubscribed)
        {
            _accessibilitySettings.HighContrastChanged -= AccessibilitySettings_HighContrastChanged;
            _accessibilitySubscribed = false;
        }
        _shutdown.Cancel();
        if (_eventPump is not null)
        {
            try { await _eventPump; } catch (OperationCanceledException) { }
        }
        if (_core is not null)
        {
            CoreClient core = _core;
            _core = null;
            await Task.Run(core.Shutdown);
        }
    }

    private sealed record UserPreferences(string? SelectedModelId, string? Language, string Source, string Theme)
    {
        public UserPreferences() : this(null, null, "microphone", "system") { }
    }
}
