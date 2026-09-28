using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using WinRT.Interop;

namespace RimV.Windows;

public sealed partial class ShellWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly AppWindow _appWindow;
    private Func<PixelRect?>? _trayBounds;
    private bool _isVisible;
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
        var id = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(WindowHandle);
        _appWindow = AppWindow.GetFromWindowId(id);
        var presenter = OverlappedPresenter.Create();
        presenter.IsResizable = false;
        presenter.IsMaximizable = false;
        presenter.IsMinimizable = false;
        presenter.SetBorderAndTitleBar(false, false);
        _appWindow.SetPresenter(presenter);
        NativeMethods.MakeToolWindow(WindowHandle);
        _appWindow.Closing += AppWindow_Closing;
        Activated += ShellWindow_Activated;
        RootGrid.Loaded += (_, _) => App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) => _coordinator.Changed -= Coordinator_Changed;
        Render();
    }

    internal nint WindowHandle { get; }
    internal AppWindow NativeAppWindow => _appWindow;
    internal AppCoordinator Coordinator => _coordinator;

    internal void SetTrayBoundsProvider(Func<PixelRect?> provider) => _trayBounds = provider;

    private void AppWindow_Closing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (App.CurrentApp.IsShuttingDown) return;
        App.CurrentApp.Log.Info("shell.close_requested_hide_to_tray");
        args.Cancel = true;
        HidePopup();
    }

    private void ShellWindow_Activated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState == WindowActivationState.Deactivated && _isVisible && !PointerIsOverTrayIcon())
            HidePopup();
    }

    public void ToggleNearTray()
    {
        if (_isVisible)
        {
            HidePopup();
            return;
        }
        ShowFromActivation();
    }

    public void ShowFromActivation()
    {
        PositionNearTray();
        _isVisible = true;
        _appWindow.Show();
        Activate();
        App.CurrentApp.Log.Info("shell.show_completed", $"native_window_visible={NativeMethods.IsWindowVisible(WindowHandle)}");
        Render();
    }

    private void PositionNearTray()
    {
        PixelRect? iconBounds = _trayBounds?.Invoke();
        NativeMethods.GetCursorPos(out NativeMethods.Point cursor);
        PixelRect icon = iconBounds ?? new PixelRect(cursor.X, cursor.Y, cursor.X + 1, cursor.Y + 1);
        NativeMethods.Point iconCenter = new() { X = icon.CenterX, Y = icon.CenterY };
        nint monitor = NativeMethods.MonitorFromPoint(iconCenter, NativeMethods.MONITOR_DEFAULTTONEAREST);
        NativeMethods.MonitorInfo info = new() { Size = (uint)System.Runtime.InteropServices.Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (!NativeMethods.GetMonitorInfo(monitor, ref info)) return;

        if (NativeMethods.GetDpiForMonitor(monitor, NativeMethods.MDT_EFFECTIVE_DPI, out uint dpiX, out _) != 0 || dpiX == 0)
            dpiX = 96;
        double scale = dpiX / 96d;
        PixelRect work = new(info.Work.Left, info.Work.Top, info.Work.Right, info.Work.Bottom);
        int width = Math.Min((int)Math.Round(420d * scale), Math.Max(1, work.Width - 16));
        int height = Math.Min((int)Math.Round(580d * scale), Math.Max(1, work.Height - 16));
        TrayPopupPlacement placement = TrayPopupPositioner.Place(icon, work, width, height, (int)Math.Round(32 * scale));
        _appWindow.MoveAndResize(new global::Windows.Graphics.RectInt32(placement.X, placement.Y, width, height));
        ShowPointer(placement, scale);
        NativeMethods.ClipTrayPopup(WindowHandle, width, height, (int)Math.Round(32 * scale),
            (int)Math.Round(14 * scale), placement.PointerOffset, placement.Edge);
    }

    private bool PointerIsOverTrayIcon()
    {
        PixelRect? bounds = _trayBounds?.Invoke();
        if (bounds is null || !NativeMethods.GetCursorPos(out NativeMethods.Point point)) return false;
        return point.X >= bounds.Value.Left && point.X < bounds.Value.Right
            && point.Y >= bounds.Value.Top && point.Y < bounds.Value.Bottom;
    }

    private void ShowPointer(TrayPopupPlacement placement, double scale)
    {
        const double size = 14;
        BottomPointer.Visibility = Visibility.Collapsed;
        TopPointer.Visibility = Visibility.Collapsed;
        LeftPointer.Visibility = Visibility.Collapsed;
        RightPointer.Visibility = Visibility.Collapsed;
        MenuSurface.Margin = placement.Edge switch
        {
            PopupPointerEdge.Top => new Thickness(0, size, 0, 0),
            PopupPointerEdge.Bottom => new Thickness(0, 0, 0, size),
            PopupPointerEdge.Left => new Thickness(size, 0, 0, 0),
            _ => new Thickness(0, 0, size, 0),
        };
        double offset = placement.PointerOffset / scale - 12;
        if (placement.Edge == PopupPointerEdge.Bottom)
        {
            BottomPointer.Visibility = Visibility.Visible;
            Canvas.SetLeft(BottomPointerBorder, offset);
            Canvas.SetLeft(BottomPointerFill, offset);
        }
        else if (placement.Edge == PopupPointerEdge.Top)
        {
            TopPointer.Visibility = Visibility.Visible;
            Canvas.SetLeft(TopPointerBorder, offset);
            Canvas.SetLeft(TopPointerFill, offset);
        }
        else if (placement.Edge == PopupPointerEdge.Left)
        {
            LeftPointer.Visibility = Visibility.Visible;
            Canvas.SetTop(LeftPointerBorder, offset);
            Canvas.SetTop(LeftPointerFill, offset);
        }
        else
        {
            RightPointer.Visibility = Visibility.Visible;
            Canvas.SetTop(RightPointerBorder, offset);
            Canvas.SetTop(RightPointerFill, offset);
        }
    }

    private void HidePopup()
    {
        _isVisible = false;
        _appWindow.Hide();
        App.CurrentApp.Log.Info("shell.hidden_to_tray");
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
            "starting" => "Starting",
            "recording" => "Listening",
            "stopping" => "Stopping",
            "error" => "Error",
            _ => state.Status,
        };
        StatusText.Text = _coordinator.IsCoreAvailable ? status : "Unavailable";
        bool unavailable = !_coordinator.IsCoreAvailable || state.Status == "error";
        Brush statusBrush = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            unavailable ? "RimVErrorBrush" : "RimVMenuReadyTextBrush"];
        StatusText.Foreground = statusBrush;
        StatusDot.Fill = statusBrush;
        StatusBadge.Background = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            unavailable ? "RimVMenuPanelBrush" : "RimVMenuReadyBackgroundBrush"];
        bool transition = state.Status is "starting" or "stopping";
        ListenButton.IsEnabled = _coordinator.IsCoreAvailable && !transition;
        bool listening = state.Status is "recording" or "starting";
        ListenLabel.Text = listening ? "Stop Listening" : "Start Listening";
        ListenIcon.Glyph = listening ? "\uE71A" : "\uE768";
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(ListenButton, ListenLabel.Text);
        foreach (ToggleButton sourceButton in new[] { SystemSource, MicrophoneSource, BothSource })
        {
            string source = (string)sourceButton.Tag;
            bool available = source switch
            {
                "system" => state.Capabilities.SystemAudioCapture,
                "both" => state.Capabilities.SystemAudioCapture && state.Capabilities.MicrophoneCapture,
                _ => state.Capabilities.MicrophoneCapture,
            };
            sourceButton.IsEnabled = _coordinator.IsCoreAvailable && state.Status == "idle" && available;
            sourceButton.Opacity = sourceButton.IsEnabled ? 1 : 0.55;
            bool selected = source == _coordinator.SelectedSource;
            sourceButton.IsChecked = selected;
            sourceButton.BorderThickness = new Thickness(selected ? 2 : 1);
            sourceButton.BorderBrush = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
                selected ? "RimVMenuAccentBrush" : "RimVMenuBorderBrush"];
            sourceButton.Foreground = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
                selected ? "RimVMenuSelectionTextBrush" : "RimVMenuTextBrush"];
        }

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
    }

    private async void ListenButton_Click(object sender, RoutedEventArgs e) => await _coordinator.StartOrStopAsync();
    private async void SourceButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is ToggleButton button) await _coordinator.SetSourceAsync((string)button.Tag);
        Render();
    }

    private void RootGrid_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != global::Windows.System.VirtualKey.Escape) return;
        HidePopup();
        e.Handled = true;
    }

    private void LanguageButton_Click(object sender, RoutedEventArgs e) => OpenLanguageWindow();
    private void ModelButton_Click(object sender, RoutedEventArgs e) => OpenModelWindow();
    private void RecordingsButton_Click(object sender, RoutedEventArgs e) => OpenRecordingsWindow();
    private void QuitButton_Click(object sender, RoutedEventArgs e) => App.CurrentApp.Quit();

    public void OpenLanguageWindow()
    {
        App.CurrentApp.Log.Info("window.open", "language");
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
        App.CurrentApp.Log.Info("window.open", "model_manager");
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
        App.CurrentApp.Log.Info("window.open", "recordings");
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
        App.CurrentApp.Log.Info("window.open", "settings");
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
        App.CurrentApp.Log.Info("window.open", "transcription");
        HidePopup();
        if (_transcriptionWindow is null)
        {
            _transcriptionWindow = new TranscriptionWindow(_coordinator);
            _transcriptionWindow.Closed += (_, _) => _transcriptionWindow = null;
        }
        _transcriptionWindow.ShowSession(sessionId);
    }
}
