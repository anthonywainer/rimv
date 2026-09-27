using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using RimV.Windows.Application;
using System.Threading;

namespace RimV.Windows;

public partial class App : Microsoft.UI.Xaml.Application
{
    private Mutex? _instanceMutex;
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
        _instanceMutex = new Mutex(initiallyOwned: true, "Local\\RimV.Native.Windows.v01", out bool firstInstance);
        if (!firstInstance)
        {
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
        try
        {
            _tray.Initialize();
        }
        catch (Exception error)
        {
            _tray.Dispose();
            Coordinator.ReportError(error);
            _shell.ToggleNearTray();
        }

        try
        {
            await Coordinator.InitializeAsync();
        }
        catch (Exception error)
        {
            Coordinator.ReportError(error);
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
