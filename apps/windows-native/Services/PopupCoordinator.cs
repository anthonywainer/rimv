using Microsoft.UI.Dispatching;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using RimV.Windows.Application;
using WinRT.Interop;

namespace RimV.Windows;

/// <summary>Owns visibility, selector switching, Escape, and native activation for the tray popup group.</summary>
internal sealed class PopupCoordinator : IDisposable
{
    private readonly ShellWindow _shell;
    private readonly DispatcherQueue _dispatcher;
    private readonly PopupGroupState _state = new();
    private Window? _selectorWindow;
    private nint _selectorHandle;
    private uint _transition;
    private bool _disposed;

    internal PopupCoordinator(ShellWindow shell, DispatcherQueue dispatcher)
    {
        _shell = shell;
        _dispatcher = dispatcher;
        _shell.Activated += Window_Activated;
    }

    internal bool IsMenuVisible => _state.IsMenuVisible;
    internal PopupSelectorKind ActiveSelector => _state.ActiveSelector;

    internal void ShowMenu() => _state.ShowMenu();

    internal bool ToggleMenu()
    {
        bool visible = _state.ToggleMenu();
        if (visible) _shell.ShowFromActivation();
        else DismissWindows();
        return visible;
    }

    internal void OpenSelector(PopupSelectorKind kind, Window window, FrameworkElement anchor,
        int widthDip, int heightDip, Action<SelectorPopupPlacement, double> applyPointer)
    {
        if (!_state.IsMenuVisible) return;
        if (_state.ActiveSelector == kind)
        {
            CloseSelector();
            return;
        }

        CloseSelector();
        _state.OpenSelector(kind);
        _selectorWindow = window;
        _selectorHandle = WindowNative.GetWindowHandle(window);
        AppWindow appWindow = WindowHelpers.ConfigurePopover(window,
            (int)Math.Round(widthDip * DpiScale(_shell.WindowHandle)),
            (int)Math.Round(heightDip * DpiScale(_shell.WindowHandle)), _shell.WindowHandle);
        window.Activated += Window_Activated;
        window.Closed += SelectorWindow_Closed;
        SelectorPopupPlacement placement = WindowHelpers.PositionPopover(_shell, anchor, _selectorHandle, appWindow, widthDip, heightDip);
        double scale = DpiScale(_selectorHandle);
        applyPointer(placement, scale);
        _transition++;
        appWindow.Show();
        window.Activate();
        App.CurrentApp.Log.Info("popup.selector_open", $"selector={kind}; edge={placement.Edge}; dpi_scale={scale:0.##}");
    }

    internal void CloseSelector()
    {
        Window? window = _selectorWindow;
        if (window is null)
        {
            _state.CloseSelector();
            return;
        }

        _selectorWindow = null;
        _selectorHandle = 0;
        _state.CloseSelector();
        window.Activated -= Window_Activated;
        window.Closed -= SelectorWindow_Closed;
        try { window.Close(); }
        catch (InvalidOperationException) { }
        _transition++;
    }

    internal void HandleEscape()
    {
        bool dismissedMenu = _state.Escape();
        if (dismissedMenu) DismissWindows();
        else CloseSelector();
    }

    internal void DismissAll(string reason)
    {
        if (!_state.IsMenuVisible && _selectorWindow is null) return;
        App.CurrentApp.Log.Info("popup.dismiss_all", $"reason={reason}");
        _state.HideAll();
        DismissWindows();
    }

    internal void CloseForIndependentWindow() => DismissAll("independent_window_opened");

    private void DismissWindows()
    {
        CloseSelector();
        _shell.HideMainFromCoordinator();
    }

    private void Window_Activated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState != WindowActivationState.Deactivated || !_state.IsMenuVisible) return;
        uint transition = _transition;
        _dispatcher.TryEnqueue(() =>
        {
            if (transition != _transition || !_state.IsMenuVisible) return;
            nint foreground = NativeMethods.GetForegroundWindow();
            bool inGroup = foreground == _shell.WindowHandle || (_selectorHandle != 0 && foreground == _selectorHandle);
            if (!_state.ShouldDismissForActivation(inGroup, _shell.PointerIsOverTrayIcon())) return;
            DismissAll("outside_activation");
        });
    }

    private void SelectorWindow_Closed(object sender, WindowEventArgs args)
    {
        if (!ReferenceEquals(sender, _selectorWindow)) return;
        _selectorWindow.Activated -= Window_Activated;
        _selectorWindow = null;
        _selectorHandle = 0;
        _state.CloseSelector();
        _transition++;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _shell.Activated -= Window_Activated;
        _state.HideAll();
        CloseSelector();
    }

    private static double DpiScale(nint hwnd)
    {
        uint dpi = NativeMethods.GetDpiForWindow(hwnd);
        return dpi == 0 ? 1 : dpi / 96d;
    }
}
