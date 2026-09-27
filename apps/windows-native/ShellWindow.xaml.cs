using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using WinRT.Interop;

namespace RimV.Windows;

public sealed partial class ShellWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly AppWindow _appWindow;
    private bool _isVisible;
    private bool _changingSource;
    private LanguageWindow? _languageWindow;
    private ModelManagerWindow? _modelWindow;
    private RecordingsWindow? _recordingsWindow;
    private SettingsWindow? _settingsWindow;
    private TranscriptionWindow? _transcriptionWindow;

    public ShellWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _coordinator.Changed += Coordinator_Changed;
        WindowHandle = WindowNative.GetWindowHandle(this);
        WindowId id = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(WindowHandle);
        _appWindow = AppWindow.GetFromWindowId(id);
        _appWindow.Resize(new Windows.Graphics.SizeInt32(420, 640));
        var presenter = OverlappedPresenter.Create();
        presenter.IsResizable = false;
        presenter.IsMaximizable = false;
        presenter.IsMinimizable = false;
        presenter.IsAlwaysOnTop = true;
        _appWindow.SetPresenter(presenter);
        _appWindow.Closing += AppWindow_Closing;
        Activated += ShellWindow_Activated;
        RootGrid.Loaded += (_, _) => App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) => _coordinator.Changed -= Coordinator_Changed;
        Render();
    }

    internal nint WindowHandle { get; }
    internal AppWindow NativeAppWindow => _appWindow;
    internal AppCoordinator Coordinator => _coordinator;

    private void AppWindow_Closing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (App.CurrentApp.IsShuttingDown) return;
        args.Cancel = true;
        HidePopup();
    }

    private void ShellWindow_Activated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState == WindowActivationState.Deactivated && _isVisible)
            HidePopup();
    }

    public void ToggleNearTray()
    {
        if (_isVisible)
        {
            HidePopup();
            return;
        }
        PositionNearTaskbar();
        Activate();
        _isVisible = true;
        Render();
    }

    private void PositionNearTaskbar()
    {
        NativeMethods.GetCursorPos(out NativeMethods.Point cursor);
        nint monitor = NativeMethods.MonitorFromPoint(cursor, NativeMethods.MONITOR_DEFAULTTONEAREST);
        NativeMethods.MonitorInfo info = new() { Size = (uint)System.Runtime.InteropServices.Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (!NativeMethods.GetMonitorInfo(monitor, ref info)) return;

        if (NativeMethods.GetDpiForMonitor(monitor, NativeMethods.MDT_EFFECTIVE_DPI, out uint dpiX, out _) != 0 || dpiX == 0)
            dpiX = 96;
        int width = (int)Math.Round(420d * dpiX / 96d);
        int height = (int)Math.Round(640d * dpiX / 96d);
        NativeMethods.Rect work = info.Work;
        int x = Math.Clamp(cursor.X - width / 2, work.Left + 8, Math.Max(work.Left + 8, work.Right - width - 8));
        int y;
        if (cursor.Y >= work.Bottom)
            y = work.Bottom - height - 8;
        else if (cursor.Y <= work.Top)
            y = work.Top + 8;
        else if (cursor.X <= work.Left)
        {
            x = work.Left + 8;
            y = Math.Clamp(cursor.Y - height / 2, work.Top + 8, Math.Max(work.Top + 8, work.Bottom - height - 8));
        }
        else if (cursor.X >= work.Right)
        {
            x = work.Right - width - 8;
            y = Math.Clamp(cursor.Y - height / 2, work.Top + 8, Math.Max(work.Top + 8, work.Bottom - height - 8));
        }
        else
            y = work.Bottom - height - 8;

        y = Math.Clamp(y, work.Top + 8, Math.Max(work.Top + 8, work.Bottom - height - 8));
        _appWindow.MoveAndResize(new Windows.Graphics.RectInt32(x, y, width, height));
    }

    private void HidePopup()
    {
        _isVisible = false;
        _appWindow.Hide();
    }

    public void DestroyShellWindow()
    {
        _appWindow.Closing -= AppWindow_Closing;
        App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        Close();
    }

    private void Coordinator_Changed(object? sender, EventArgs args)
    {
        if (App.CurrentApp.UiQueue.HasThreadAccess) Render();
        else App.CurrentApp.UiQueue.TryEnqueue(Render);
    }

    private void Render()
    {
        CoreSnapshot state = _coordinator.Snapshot;
        string status = state.Status switch
        {
            "idle" => "Ready",
            "starting" => "Starting listening…",
            "recording" => "Listening",
            "stopping" => "Finishing recording…",
            "error" => "Audio stopped with an error",
            _ => state.Status,
        };
        StatusText.Text = _coordinator.IsCoreAvailable ? status : "Shared engine unavailable";
        StatusText.Foreground = state.Status == "recording"
            ? (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["RimVLiveBrush"]
            : (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["RimVSecondaryTextBrush"];
        bool transition = state.Status is "starting" or "stopping";
        ListenButton.IsEnabled = _coordinator.IsCoreAvailable && !transition;
        ListenButton.Content = state.Status is "recording" or "starting" ? "Stop listening" : "Start listening";
        SourceCombo.IsEnabled = _coordinator.IsCoreAvailable && state.Status == "idle";
        _changingSource = true;
        foreach (ComboBoxItem item in SourceCombo.Items)
        {
            string source = (string)item.Tag;
            item.IsEnabled = source switch
            {
                "system" => state.Capabilities.SystemAudioCapture,
                "both" => state.Capabilities.SystemAudioCapture && state.Capabilities.MicrophoneCapture,
                _ => state.Capabilities.MicrophoneCapture,
            };
            if (source == _coordinator.SelectedSource) SourceCombo.SelectedItem = item;
        }
        _changingSource = false;

        ModelRecord? model = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == _coordinator.SelectedModelId);
        ModelName.Text = model?.Descriptor.DisplayName ?? "Choose a model";
        LanguageName.Text = string.IsNullOrWhiteSpace(_coordinator.SelectedLanguage) ? "Automatic" : _coordinator.SelectedLanguage;
        ErrorInfo.Message = _coordinator.ErrorMessage ?? "";
        ErrorInfo.IsOpen = _coordinator.ErrorMessage is not null;
        ModelInfo.Message = !_coordinator.IsCoreAvailable
            ? "The shared Rust engine is unavailable. Rebuild with rimv_core_ffi.dll to use capture and transcription."
            : state.Transcription.Status == "loading" ? "Loading the speech model…"
            : model?.State == "unsupported" ? "The selected model backend isn't included in this Windows build. Choose an available model."
            : model is null ? "Choose a speech model to turn listening into a transcript." : model.State == "installed" ? "Model ready" : "This model needs to be downloaded.";
        ModelInfo.IsOpen = !_coordinator.IsCoreAvailable || state.Transcription.Status == "loading" || model is null || model.State != "installed";
        FooterState.Text = state.Session is null
            ? "Starting a recording won't open the transcript window. Open it later from Recordings."
            : state.Status == "recording" ? "Your current recording is available in Recordings." : "Your current session is being saved.";
    }

    private async void ListenButton_Click(object sender, RoutedEventArgs e) => await _coordinator.StartOrStopAsync();
    private async void SourceCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_changingSource || SourceCombo.SelectedItem is not ComboBoxItem item) return;
        await _coordinator.SetSourceAsync((string)item.Tag);
    }

    private void LanguageButton_Click(object sender, RoutedEventArgs e) => OpenLanguageWindow();
    private void ModelButton_Click(object sender, RoutedEventArgs e) => OpenModelWindow();
    private void RecordingsButton_Click(object sender, RoutedEventArgs e) => OpenRecordingsWindow();
    private void SettingsButton_Click(object sender, RoutedEventArgs e) => OpenSettingsWindow();
    private void QuitButton_Click(object sender, RoutedEventArgs e) => App.CurrentApp.Quit();

    private void MoreButton_Click(object sender, RoutedEventArgs e)
    {
        var menu = new MenuFlyout();
        menu.Items.Add(new MenuFlyoutItem { Text = "Open current recording" });
        menu.Items.Add(new MenuFlyoutSeparator());
        menu.Items.Add(new MenuFlyoutItem { Text = "Quit RimV" });
        ((MenuFlyoutItem)menu.Items[0]).Click += (_, _) =>
        {
            if (_coordinator.Snapshot.Session is { } session) OpenTranscriptionWindow(session.Id);
        };
        ((MenuFlyoutItem)menu.Items[2]).Click += (_, _) => App.CurrentApp.Quit();
        menu.ShowAt((FrameworkElement)sender);
    }

    public void OpenLanguageWindow()
    {
        HidePopup();
        if (_languageWindow is null)
        {
            _languageWindow = new LanguageWindow(_coordinator);
            _languageWindow.Closed += (_, _) => _languageWindow = null;
        }
        _languageWindow.Activate();
    }

    public void OpenModelWindow()
    {
        HidePopup();
        if (_modelWindow is null)
        {
            _modelWindow = new ModelManagerWindow(_coordinator);
            _modelWindow.Closed += (_, _) => _modelWindow = null;
        }
        _modelWindow.Activate();
    }

    public void OpenRecordingsWindow()
    {
        HidePopup();
        if (_recordingsWindow is null)
        {
            _recordingsWindow = new RecordingsWindow(_coordinator, OpenTranscriptionWindow);
            _recordingsWindow.Closed += (_, _) => _recordingsWindow = null;
        }
        _recordingsWindow.Activate();
    }

    public void OpenSettingsWindow()
    {
        HidePopup();
        if (_settingsWindow is null)
        {
            _settingsWindow = new SettingsWindow(_coordinator);
            _settingsWindow.Closed += (_, _) => _settingsWindow = null;
        }
        _settingsWindow.Activate();
    }

    public void OpenTranscriptionWindow(string sessionId)
    {
        HidePopup();
        if (_transcriptionWindow is null)
        {
            _transcriptionWindow = new TranscriptionWindow(_coordinator);
            _transcriptionWindow.Closed += (_, _) => _transcriptionWindow = null;
        }
        _transcriptionWindow.ShowSession(sessionId);
    }
}
