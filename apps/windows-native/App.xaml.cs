using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using RimV.Windows.Application;
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

    public static App CurrentApp => (App)Current;
    public AppCoordinator Coordinator { get; private set; } = null!;
    internal ThemeManager ThemeManager { get; private set; } = null!;
    public DispatcherQueue UiQueue { get; private set; } = null!;
    public bool IsShuttingDown { get; private set; }

    public App()
    {
        InitializeComponent();
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        EventWaitHandle activationEvent = new(
            initialState: false,
            EventResetMode.AutoReset,
            ActivationEventName);
        _activationEvent = activationEvent;
        Mutex instanceMutex = new(initiallyOwned: true, InstanceMutexName, out bool firstInstance);
        _instanceMutex = instanceMutex;
        if (!firstInstance)
        {
            activationEvent.Set();
            activationEvent.Dispose();
            _activationEvent = null;
            instanceMutex.Dispose();
            _instanceMutex = null;
            Exit();
            return;
        }

        UiQueue = DispatcherQueue.GetForCurrentThread();
        string dataDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "RimV");
        Coordinator = new AppCoordinator(
            dataDirectory,
            new CoreClientFactory(),
            new FileUserPreferencesStore(Path.Combine(dataDirectory, "preferences.json")),
            new WinUiDispatcher(UiQueue));
        ThemeManager = new ThemeManager(Coordinator);
        _shell = new ShellWindow(Coordinator);
        _tray = new TrayIcon(_shell);
        CancellationTokenSource activationCancellation = new();
        _activationCancellation = activationCancellation;
        CancellationToken activationToken = activationCancellation.Token;
        _activationListener = Task.Run(() => ListenForActivation(activationToken));
        try
        {
            _tray.Initialize();
        }
        catch (Exception error)
        {
            _tray.Dispose();
            Coordinator.ReportError(error);
        }

        // The installed app must provide a visible first-run entry point even
        // when the notification-area icon initializes successfully.
        _shell.ToggleNearTray();

        try
        {
            await Coordinator.InitializeAsync();
        }
        catch (Exception error)
        {
            Coordinator.ReportError(error);
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
                if (!IsShuttingDown) _shell?.ShowFromActivation();
            });
        }
    }

    public void TogglePopup()
    {
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
