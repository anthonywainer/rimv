using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using System.Diagnostics;
using Windows.System;
using WinRT.Interop;

namespace RimV.Windows;

public sealed partial class ShellWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly AppWindow _appWindow;
    private Func<PixelRect?>? _trayBounds;
    private readonly PopupCoordinator _popups;
    private LanguageWindow? _languageWindow;
    private ModelSelectorWindow? _modelSelectorWindow;
    private ModelManagerWindow? _modelManagerWindow;
    private RecordingsWindow? _recordingsWindow;
    private SettingsWindow? _settingsWindow;
    private TranscriptionWindow? _transcriptionWindow;

    public ShellWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _popups = new PopupCoordinator(this, App.CurrentApp.UiQueue);
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
        RootGrid.Loaded += (_, _) => App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _popups.Dispose();
            _coordinator.Changed -= Coordinator_Changed;
        };
        Render();
    }

    internal nint WindowHandle { get; }
    internal AppWindow NativeAppWindow => _appWindow;
    internal AppCoordinator Coordinator => _coordinator;
    internal bool IsPopupVisible => _popups.IsMenuVisible;
    internal bool HasActiveSelector => _popups.ActiveSelector != PopupSelectorKind.None;

    internal async Task<bool> ConfirmNativeSpeechModelDownloadAsync(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var dialog = new ContentDialog
        {
            Title = "Prepare Windows speech recognition?",
            Content = "Windows may download its optional on-device speech recognition model through Windows Update. Audio is processed locally. Continue?",
            PrimaryButtonText = "Continue",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = RootGrid.XamlRoot,
        };
        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }

    internal void SetTrayBoundsProvider(Func<PixelRect?> provider) => _trayBounds = provider;

    private void AppWindow_Closing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (App.CurrentApp.IsShuttingDown) return;
        App.CurrentApp.Log.Info("shell.close_requested_hide_to_tray");
        args.Cancel = true;
        _popups.DismissAll("shell_close_requested");
    }

    public void ToggleNearTray()
    {
        _popups.ToggleMenu();
    }

    public void ShowFromActivation()
    {
        _popups.ShowMenu();
        Render();
        PositionNearTray();
        _appWindow.Show();
        Activate();
        App.CurrentApp.Log.Info("shell.show_completed", $"native_window_visible={NativeMethods.IsWindowVisible(WindowHandle)}");
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
        int margin = (int)Math.Round(8 * scale);
        int width = Math.Min((int)Math.Round(420d * scale), Math.Max(1, work.Width - margin * 2));
        MenuScrollViewer.Measure(new global::Windows.Foundation.Size(width / scale - 34, double.PositiveInfinity));
        MenuScrollViewer.UpdateLayout();
        double contentHeightDip = MenuScrollViewer.ExtentHeight > 0
            ? MenuScrollViewer.ExtentHeight
            : MenuScrollViewer.DesiredSize.Height;
        if (!double.IsFinite(contentHeightDip) || contentHeightDip <= 0) contentHeightDip = 560;
        int desiredHeight = (int)Math.Ceiling((contentHeightDip + MenuSurface.Padding.Top + MenuSurface.Padding.Bottom + 14) * scale);
        int availableHeight = Math.Max(1, work.Height - margin * 2);
        PopupSizingPolicy.Result size = PopupSizingPolicy.FitContent(desiredHeight, availableHeight,
            (int)Math.Round(500 * scale), (int)Math.Round(680 * scale));
        int height = size.Height;
        MenuScrollViewer.VerticalScrollBarVisibility = size.RequiresScrolling
            ? ScrollBarVisibility.Auto
            : ScrollBarVisibility.Disabled;
        TrayPopupPlacement placement = TrayPopupPositioner.Place(icon, work, width, height, (int)Math.Round(32 * scale));
        _appWindow.MoveAndResize(new global::Windows.Graphics.RectInt32(placement.X, placement.Y, width, height));
        ShowPointer(placement, scale);
        NativeMethods.ClipTrayPopup(WindowHandle, width, height, (int)Math.Round(32 * scale),
            (int)Math.Round(14 * scale), placement.PointerOffset, placement.Edge);
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

    internal void HideMainFromCoordinator()
    {
        _appWindow.Hide();
        App.CurrentApp.Log.Info("shell.hidden_to_tray");
    }

    internal bool PointerIsOverTrayIcon()
    {
        PixelRect? bounds = _trayBounds?.Invoke();
        if (bounds is null || !NativeMethods.GetCursorPos(out NativeMethods.Point point)) return false;
        return point.X >= bounds.Value.Left && point.X < bounds.Value.Right
            && point.Y >= bounds.Value.Top && point.Y < bounds.Value.Bottom;
    }

    internal void CloseSelectorPopup() => _popups.CloseSelector();

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
        bool visibleError = _coordinator.ErrorMessage is not null;
        string status = state.Status switch
        {
            "idle" => "Ready",
            "starting" => "Starting",
            "recording" => "Listening",
            "stopping" => "Stopping",
            "error" when visibleError => "Error",
            "error" => "Ready",
            _ => state.Status,
        };
        StatusText.Text = _coordinator.IsCoreAvailable ? status : "Unavailable";
        BrandSubtitle.Text = visibleError ? "Capture needs attention" : "Real-time transcription";
        bool unavailable = !_coordinator.IsCoreAvailable || visibleError;
        Brush statusBrush = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            unavailable ? "RimVErrorBrush" : "RimVMenuReadyTextBrush"];
        StatusText.Foreground = statusBrush;
        StatusDot.Fill = statusBrush;
        StatusBadge.Background = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            unavailable ? "RimVMenuPanelBrush" : "RimVMenuReadyBackgroundBrush"];
        bool transition = state.Status is "starting" or "stopping" || _coordinator.IsPreparingToListen;
        bool canListen = _coordinator.IsCoreAvailable && !transition;
        bool listening = state.Status is "recording" or "starting";
        ListenButton.IsEnabled = canListen;
        ListenButton.Background = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            !canListen ? "RimVMenuDisabledBackgroundBrush"
                : listening ? "RimVMenuStopBackgroundBrush" : "RimVMenuPrimaryActionBrush"];
        Brush listenForeground = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
            !canListen ? "RimVMenuDisabledTextBrush"
                : listening ? "RimVMenuStopTextBrush" : "RimVMenuPrimaryActionForegroundBrush"];
        ListenButton.Foreground = listenForeground;
        ListenIcon.Foreground = listenForeground;
        ListenLabel.Foreground = listenForeground;
        ListenLabel.Text = NativeWindowsPolicy.CaptureActionLabel(
            state.Status, _coordinator.IsPreparingToListen, visibleError);
        ListenIcon.Glyph = transition ? "\uE895" : listening ? "\uE71A" : visibleError ? "\uE72C" : "\uE768";
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
            bool selected = source == _coordinator.SelectedSource;
            sourceButton.IsChecked = selected;
            sourceButton.Background = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
                !sourceButton.IsEnabled ? "RimVMenuDisabledBackgroundBrush"
                    : selected ? "RimVMenuSelectedBackgroundBrush" : "RimVMenuSurfaceBrush"];
            sourceButton.BorderThickness = new Thickness(selected ? 2 : 1);
            sourceButton.BorderBrush = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
                selected ? "RimVMenuAccentBrush" : "RimVMenuBorderBrush"];
            sourceButton.Foreground = (Brush)Microsoft.UI.Xaml.Application.Current.Resources[
                !sourceButton.IsEnabled ? "RimVMenuDisabledTextBrush"
                    : selected ? "RimVMenuSelectionTextBrush" : "RimVMenuTextBrush"];
            if (sourceButton.Content is StackPanel content)
            {
                foreach (FrameworkElement child in content.Children)
                {
                    if (child is TextBlock text) text.Foreground = sourceButton.Foreground;
                    else if (child is FontIcon icon) icon.Foreground = sourceButton.Foreground;
                }
            }
        }

        ModelRecord? model = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == _coordinator.SelectedModelId);
        ModelName.Text = model?.Descriptor.DisplayName ?? "Choose a model";
        bool supportsLanguageDetection = model?.Descriptor.Capabilities.SupportsLanguageDetection == true;
        LanguagePresentation? languagePresentation = PresentLanguage(_coordinator.SelectedLanguage);
        LanguageName.Text = languagePresentation?.LanguageName
            ?? (supportsLanguageDetection ? "Auto Detect" : "Default");
        LanguageFlag.Source = LanguageFlagAssets.ForRegion(languagePresentation?.RegionCode);
        LanguageFlag.Visibility = LanguageFlag.Source is null ? Visibility.Collapsed : Visibility.Visible;
        ErrorMessage.Text = _coordinator.ErrorMessage ?? "";
        ErrorTitle.Text = _coordinator.ErrorMessage is string error
            && error.Contains("model in Model Manager", StringComparison.OrdinalIgnoreCase)
            ? "Models required"
            : "No active source";
        ErrorCard.Visibility = visibleError ? Visibility.Visible : Visibility.Collapsed;
        ModelInfo.Message = !_coordinator.IsCoreAvailable
            ? "The shared Rust engine is unavailable. Rebuild with rimv_core_ffi.dll to use capture and transcription."
            : model?.State == "unsupported" ? "The selected model backend isn't included in this Windows build. Choose an available model."
            : model is null && !_coordinator.NativeSpeech.Supported
                ? "Windows Native Speech is unavailable here. Open Model Manager to install Parakeet or Whisper for local transcription."
            : model is null ? "Choose a speech model to turn listening into a transcript."
            : NativeWindowsPolicy.SpeechModelStatusMessage(state.Transcription.Status, model.State);
        ModelInfo.IsOpen = !_coordinator.IsCoreAvailable || state.Transcription.Status is "loading" or "error"
            || model is null || model.State != "installed";
    }

    private static LanguagePresentation? PresentLanguage(string? code) =>
        string.IsNullOrWhiteSpace(code) || code == "auto"
            ? null
            : CoreClient.PresentLanguages([code]).FirstOrDefault();

    private async void ListenButton_Click(object sender, RoutedEventArgs e) => await _coordinator.StartOrStopAsync();
    private void DismissError_Click(object sender, RoutedEventArgs e) => _coordinator.ClearError();
    private void OpenSoundSettings_Click(object sender, RoutedEventArgs e)
    {
        Process.Start(new ProcessStartInfo("ms-settings:sound") { UseShellExecute = true });
    }
    private async void SourceButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is ToggleButton button) await _coordinator.SetSourceAsync((string)button.Tag);
        Render();
    }

    private void RootGrid_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Escape)
        {
            _popups.HandleEscape();
            e.Handled = true;
            return;
        }

        object? focused = FocusManager.GetFocusedElement(RootGrid.XamlRoot);
        bool textInputFocused = focused is TextBox or PasswordBox or RichEditBox or AutoSuggestBox;
        if (!MenuInteractionPolicy.ShouldQuit(e.Key == VirtualKey.Q, NativeMethods.IsControlDown(), textInputFocused)) return;
        App.CurrentApp.Quit();
        e.Handled = true;
    }

    private void LanguageButton_Click(object sender, RoutedEventArgs e) => OpenLanguageWindow();
    private void ModelButton_Click(object sender, RoutedEventArgs e) => OpenModelWindow();
    private void RecordingsButton_Click(object sender, RoutedEventArgs e) => OpenRecordingsWindow();
    private void QuitButton_Click(object sender, RoutedEventArgs e) => App.CurrentApp.Quit();

    public void OpenLanguageWindow()
    {
        EnsureMenuVisible();
        if (_popups.ActiveSelector == PopupSelectorKind.Language) { _popups.CloseSelector(); return; }
        _languageWindow = new LanguageWindow(_coordinator);
        _popups.OpenSelector(PopupSelectorKind.Language, _languageWindow, LanguageAnchor, 336, 466,
            (placement, scale) => _languageWindow?.ApplyPopupPlacement(placement, scale));
    }

    public void OpenModelWindow()
    {
        EnsureMenuVisible();
        if (_popups.ActiveSelector == PopupSelectorKind.Model) { _popups.CloseSelector(); return; }
        _modelSelectorWindow = new ModelSelectorWindow(_coordinator, OpenModelManagerWindow);
        _popups.OpenSelector(PopupSelectorKind.Model, _modelSelectorWindow, ModelAnchor, 336, 190,
            (placement, scale) => _modelSelectorWindow?.ApplyPopupPlacement(placement, scale));
    }

    public void OpenRecordingsWindow()
    {
        EnsureMenuVisible();
        if (_popups.ActiveSelector == PopupSelectorKind.Recordings) { _popups.CloseSelector(); return; }
        _recordingsWindow = new RecordingsWindow(_coordinator, OpenTranscriptionWindow);
        _popups.OpenSelector(PopupSelectorKind.Recordings, _recordingsWindow, RecordingsAnchor, 352, 440,
            (placement, scale) => _recordingsWindow?.ApplyPopupPlacement(placement, scale));
    }

    public void OpenModelManagerWindow()
    {
        _popups.CloseForIndependentWindow();
        if (_modelManagerWindow is null)
        {
            _modelManagerWindow = new ModelManagerWindow(_coordinator);
            _modelManagerWindow.Closed += (_, _) => _modelManagerWindow = null;
        }
        _modelManagerWindow.Activate();
    }

    private void EnsureMenuVisible()
    {
        if (!_popups.IsMenuVisible) ShowFromActivation();
    }

    public void OpenSettingsWindow()
    {
        App.CurrentApp.Log.Info("window.open", "settings");
        _popups.CloseForIndependentWindow();
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
        _popups.CloseForIndependentWindow();
        if (_transcriptionWindow is null)
        {
            _transcriptionWindow = new TranscriptionWindow(_coordinator);
            _transcriptionWindow.Closed += (_, _) => _transcriptionWindow = null;
        }
        _transcriptionWindow.ShowSession(sessionId);
    }

    public void CloseTranscriptionWindowForShutdown() => _transcriptionWindow?.Close();
}
