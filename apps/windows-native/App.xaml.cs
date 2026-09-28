using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using RimV.Windows.Application;
using System.Diagnostics;
using System.Threading;

namespace RimV.Windows;

public partial class App : Microsoft.UI.Xaml.Application
{
    private const string InstanceMutexName = "Local\\RimV.Native.Windows.v01";
    private const string ActivationEventName = "Local\\RimV.Native.Windows.v01.Activate";
    private Mutex? _instanceMutex;
    private EventWaitHandle? _activationEvent;
    private CancellationTokenSource? _activationCancellation;
    private Task? _activationListener;
    private TrayIcon? _tray;
    private ShellWindow? _shell;
    private FileAppLog? _log;

    public static App CurrentApp => (App)Current;
    internal IAppLog Log => (IAppLog?)_log ?? NullAppLog.Instance;
    public AppCoordinator Coordinator { get; private set; } = null!;
    internal ThemeManager ThemeManager { get; private set; } = null!;
    public DispatcherQueue UiQueue { get; private set; } = null!;
    public bool IsShuttingDown { get; private set; }

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, args) => _log?.Error("app.unhandled_exception", args.Exception);
        AppDomain.CurrentDomain.UnhandledException += (_, args) =>
        {
            if (args.ExceptionObject is Exception error) _log?.Error("app.domain_unhandled_exception", error);
            else _log?.Info("app.domain_unhandled_exception", args.ExceptionObject?.GetType().Name);
        };
        TaskScheduler.UnobservedTaskException += (_, args) =>
            _log?.Error("app.unobserved_task_exception", args.Exception);
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        string dataDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "RimV");
        _log = new FileAppLog(Path.Combine(dataDirectory, "logs", "windows-native.log"));
        _log.Info("app.launch", $"pid={Environment.ProcessId}; os={Environment.OSVersion.Version}");
        EventWaitHandle activationEvent = new(
            initialState: false,
            EventResetMode.AutoReset,
            ActivationEventName);
        _activationEvent = activationEvent;
        Mutex instanceMutex = new(initiallyOwned: true, InstanceMutexName, out bool firstInstance);
        _instanceMutex = instanceMutex;
        if (!firstInstance)
        {
            _log.Info("app.secondary_launch");
            activationEvent.Set();
            // Older running builds do not listen for the named activation event.
            bool activatedExistingWindow = false;
            foreach (Process process in Process.GetProcessesByName("RimV.Windows"))
            {
                using (process)
                {
                    if (process.Id != Environment.ProcessId
                        && NativeMethods.ActivateProcessWindow(process.Id, "RimV"))
                    {
                        activatedExistingWindow = true;
                        _log.Info("app.existing_window_activated", $"pid={process.Id}");
                        break;
                    }
                }
            }
            if (!activatedExistingWindow) _log.Info("app.existing_window_not_found");
            activationEvent.Dispose();
            _activationEvent = null;
            instanceMutex.Dispose();
            _instanceMutex = null;
            Exit();
            return;
        }

        UiQueue = DispatcherQueue.GetForCurrentThread();
        _log.Info("app.primary_instance");
        Coordinator = new AppCoordinator(
            dataDirectory,
            new CoreClientFactory(_log),
            new FileUserPreferencesStore(Path.Combine(dataDirectory, "preferences.json")),
            new WinUiDispatcher(UiQueue),
            _log);
        _log.Info("theme_manager.initializing");
        ThemeManager = new ThemeManager(Coordinator);
        _log.Info("theme_manager.initialized");
        _log.Info("shell_window.initializing");
        _shell = new ShellWindow(Coordinator);
        _log.Info("shell_window.initialized");
        _tray = new TrayIcon(_shell);
        CancellationTokenSource activationCancellation = new();
        _activationCancellation = activationCancellation;
        CancellationToken activationToken = activationCancellation.Token;
        _activationListener = Task.Run(() => ListenForActivation(activationToken));
        try
        {
            _tray.Initialize();
            _log.Info("tray.initialized");
        }
        catch (Exception error)
        {
            _tray.Dispose();
            _log.Error("tray.initialization_failed", error);
            Coordinator.ReportError(error);
        }

        // The installed app must provide a visible first-run entry point even
        // when the notification-area icon initializes successfully.
        _shell.ToggleNearTray();
        _log.Info("shell.opened_on_launch");

        try
        {
            await Coordinator.InitializeAsync();
            _log.Info("app.initialization_complete");
        }
        catch (Exception error)
        {
            Coordinator.ReportError(error);
            _log.Error("app.initialization_failed", error);
        }
    }

    private void ListenForActivation(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                _activationEvent?.WaitOne();
            }
            catch (ObjectDisposedException)
            {
                return;
            }

            if (cancellationToken.IsCancellationRequested) return;
            UiQueue.TryEnqueue(() =>
            {
                if (!IsShuttingDown)
                {
                    _log?.Info("app.activation_received");
                    _shell?.ShowFromActivation();
                }
            });
        }
    }

    public void TogglePopup()
    {
        _log?.Info("shell.toggle_requested");
        _shell?.ToggleNearTray();
    }

    public void OpenLanguageWindow() => _shell?.OpenLanguageWindow();
    public void OpenModelWindow() => _shell?.OpenModelWindow();
    public void OpenRecordingsWindow() => _shell?.OpenRecordingsWindow();
    public void OpenSettingsWindow() => _shell?.OpenSettingsWindow();
    public void OpenTranscriptionWindow(string sessionId) => _shell?.OpenTranscriptionWindow(sessionId);

    public async void Quit()
    {
        if (IsShuttingDown) return;
        _log?.Info("app.quit_requested");
        IsShuttingDown = true;
        _activationCancellation?.Cancel();
        _activationEvent?.Set();
        if (_activationListener is not null) await _activationListener;
        _activationEvent?.Dispose();
        _activationEvent = null;
        _activationCancellation?.Dispose();
        ThemeManager.Dispose();
        _tray?.Dispose();
        try
        {
            await Coordinator.ShutdownAsync();
        }
        catch (Exception error)
        {
            Coordinator.ReportError(error);
        }
        _shell?.DestroyShellWindow();
        _instanceMutex?.ReleaseMutex();
        _instanceMutex?.Dispose();
        Exit();
    }
}
