using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
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
}
