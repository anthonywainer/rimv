using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Foundation;
using WinRT.Interop;

namespace RimV.Windows;

internal static class WindowHelpers
{
    public static AppWindow Configure(Window window, int width, int height, bool resizable = true)
    {
        nint hwnd = WindowNative.GetWindowHandle(window);
        var id = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hwnd);
        AppWindow appWindow = AppWindow.GetFromWindowId(id);
        appWindow.Resize(new global::Windows.Graphics.SizeInt32(width, height));
        OverlappedPresenter presenter = OverlappedPresenter.Create();
        presenter.IsResizable = resizable;
        presenter.IsMaximizable = resizable;
        appWindow.SetPresenter(presenter);
        return appWindow;
    }

    internal static AppWindow ConfigurePopover(Window window, int width, int height, nint owner)
    {
        nint hwnd = WindowNative.GetWindowHandle(window);
        AppWindow appWindow = AppWindow.GetFromWindowId(Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hwnd));
        OverlappedPresenter presenter = OverlappedPresenter.Create();
        presenter.IsResizable = false;
        presenter.IsMaximizable = false;
        presenter.IsMinimizable = false;
        presenter.SetBorderAndTitleBar(false, false);
        appWindow.SetPresenter(presenter);
        appWindow.Resize(new global::Windows.Graphics.SizeInt32(width, height));
        NativeMethods.MakeToolWindow(hwnd);
        NativeMethods.SetOwner(hwnd, owner);
        return appWindow;
    }

    internal static SelectorPopupPlacement PositionPopover(Window owner, FrameworkElement anchor, nint popupHwnd,
        AppWindow popup, int widthDip, int heightDip)
    {
        nint ownerHwnd = WindowNative.GetWindowHandle(owner);
        if (!NativeMethods.GetWindowRect(ownerHwnd, out NativeMethods.NativeRect ownerRect))
            throw new InvalidOperationException("Could not determine the RimV menu position.");

        Point anchorPoint = anchor.TransformToVisual(null).TransformPoint(new Point(0, 0));
        nint monitor = NativeMethods.MonitorFromPoint(new NativeMethods.Point
        {
            X = ownerRect.Left + (int)Math.Round((anchorPoint.X + anchor.ActualWidth / 2) * DpiScaleForWindow(ownerHwnd)),
            Y = ownerRect.Top + (int)Math.Round((anchorPoint.Y + anchor.ActualHeight / 2) * DpiScaleForWindow(ownerHwnd)),
        }, NativeMethods.MONITOR_DEFAULTTONEAREST);
        var info = new NativeMethods.MonitorInfo { Size = (uint)System.Runtime.InteropServices.Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (!NativeMethods.GetMonitorInfo(monitor, ref info))
            throw new InvalidOperationException("Could not determine the selector monitor work area.");
        double scale = DpiScaleForMonitor(monitor);
        int width = Math.Min((int)Math.Round(widthDip * scale), Math.Max(1, info.Work.Right - info.Work.Left - 16));
        int height = Math.Min((int)Math.Round(heightDip * scale), Math.Max(1, info.Work.Bottom - info.Work.Top - 16));
        var anchorRect = new PixelRect(
            ownerRect.Left + (int)Math.Round(anchorPoint.X * DpiScaleForWindow(ownerHwnd)),
            ownerRect.Top + (int)Math.Round(anchorPoint.Y * DpiScaleForWindow(ownerHwnd)),
            ownerRect.Left + (int)Math.Round((anchorPoint.X + anchor.ActualWidth) * DpiScaleForWindow(ownerHwnd)),
            ownerRect.Top + (int)Math.Round((anchorPoint.Y + anchor.ActualHeight) * DpiScaleForWindow(ownerHwnd)));
        var work = new PixelRect(info.Work.Left, info.Work.Top, info.Work.Right, info.Work.Bottom);
        SelectorPopupPlacement placement = SelectorPopupPositioner.Place(anchorRect, work, width, height,
            (int)Math.Round(4 * scale), (int)Math.Round(30 * scale));
        popup.MoveAndResize(new global::Windows.Graphics.RectInt32(placement.X, placement.Y, width, height));
        NativeMethods.ClipSelectorPopup(popupHwnd, width, height, (int)Math.Round(16 * scale),
            (int)Math.Round(14 * scale), placement.PointerOffset, placement.Edge);
        return placement;
    }

    internal static void ApplySelectorPointer(Border surface, Canvas left, Canvas right,
        SelectorPopupPlacement placement, double scale)
    {
        left.Visibility = Visibility.Collapsed;
        right.Visibility = Visibility.Collapsed;
        surface.Margin = placement.Edge switch
        {
            PopupPointerEdge.Left => new Thickness(14, 0, 0, 0),
            PopupPointerEdge.Right => new Thickness(0, 0, 14, 0),
            _ => new Thickness(0),
        };
        double top = placement.PointerOffset / scale - 12;
        if (placement.Edge == PopupPointerEdge.Left)
        {
            left.Visibility = Visibility.Visible;
            Canvas.SetTop(left, top);
        }
        else
        {
            right.Visibility = Visibility.Visible;
            Canvas.SetTop(right, top);
        }
    }

    private static double DpiScaleForWindow(nint window)
    {
        uint dpi = NativeMethods.GetDpiForWindow(window);
        return dpi == 0 ? 1 : dpi / 96d;
    }

    private static double DpiScaleForMonitor(nint monitor)
    {
        if (NativeMethods.GetDpiForMonitor(monitor, NativeMethods.MDT_EFFECTIVE_DPI, out uint dpi, out _) != 0 || dpi == 0)
            return 1;
        return dpi / 96d;
    }
}
