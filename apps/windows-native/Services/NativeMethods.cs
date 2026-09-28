using System.Runtime.InteropServices;
using System.Text;

namespace RimV.Windows;

internal static class NativeMethods
{
    internal const uint WM_APP = 0x8000;
    internal const uint MONITOR_DEFAULTTONEAREST = 2;
    internal const uint MDT_EFFECTIVE_DPI = 0;
    internal const int GWLP_HWNDPARENT = -8;
    private const int GwlExStyle = -20;
    private const int WsExAppWindow = 0x00040000;
    private const int WsExToolWindow = 0x00000080;
    private const uint SwpFrameChanged = 0x0020;
    private const uint SwpNoMove = 0x0002;
    private const uint SwpNoSize = 0x0001;
    private const uint SwpNoZOrder = 0x0004;

    [StructLayout(LayoutKind.Sequential)]
    internal struct Point { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    internal struct Rect { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    internal struct MonitorInfo
    {
        public uint Size;
        public Rect Monitor;
        public Rect Work;
        public uint Flags;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct NativeRect { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool GetCursorPos(out Point point);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool IsWindowVisible(nint window);

    [DllImport("user32.dll")]
    internal static extern nint GetForegroundWindow();

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool GetWindowRect(nint window, out NativeRect rect);

    [DllImport("user32.dll")]
    internal static extern uint GetDpiForWindow(nint window);

    [DllImport("user32.dll")]
    private static extern short GetKeyState(int virtualKey);

    internal static bool IsControlDown() => (GetKeyState(0x11) & 0x8000) != 0;

    internal static void SetOwner(nint window, nint owner) => SetWindowLongPtr(window, GWLP_HWNDPARENT, owner);

    [DllImport("user32.dll")]
    internal static extern nint MonitorFromPoint(Point point, uint flags);

    [DllImport("user32.dll", EntryPoint = "GetMonitorInfoW", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool GetMonitorInfo(nint monitor, ref MonitorInfo info);

    [DllImport("Shcore.dll")]
    internal static extern int GetDpiForMonitor(nint monitor, uint dpiType, out uint dpiX, out uint dpiY);

    [DllImport("comctl32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool SetWindowSubclass(nint hwnd, SubclassProc callback, nuint subclassId, nuint referenceData);

    [DllImport("comctl32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool RemoveWindowSubclass(nint hwnd, SubclassProc callback, nuint subclassId);

    [DllImport("comctl32.dll")]
    internal static extern nint DefSubclassProc(nint hwnd, uint message, nuint wParam, nint lParam);

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    internal delegate nint SubclassProc(nint hwnd, uint message, nuint wParam, nint lParam, nuint subclassId, nuint referenceData);

    [DllImport("user32.dll", EntryPoint = "LoadImageW", CharSet = CharSet.Unicode, SetLastError = true)]
    internal static extern nint LoadImage(nint instance, string name, uint type, int width, int height, uint flags);

    [DllImport("user32.dll", EntryPoint = "DestroyIcon", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool DestroyIcon(nint icon);

    internal static void MakeToolWindow(nint window)
    {
        nint style = GetWindowLongPtr(window, GwlExStyle);
        SetWindowLongPtr(window, GwlExStyle, (style | (nint)WsExToolWindow) & ~(nint)WsExAppWindow);
        SetWindowPos(window, 0, 0, 0, 0, 0, SwpFrameChanged | SwpNoMove | SwpNoSize | SwpNoZOrder);
    }

    internal static void ClipTrayPopup(nint window, int width, int height, int cornerRadius,
        int pointerSize, int pointerOffset, RimV.Windows.Application.PopupPointerEdge edge)
    {
        int left = edge == RimV.Windows.Application.PopupPointerEdge.Left ? pointerSize : 0;
        int top = edge == RimV.Windows.Application.PopupPointerEdge.Top ? pointerSize : 0;
        int right = width - (edge == RimV.Windows.Application.PopupPointerEdge.Right ? pointerSize : 0);
        int bottom = height - (edge == RimV.Windows.Application.PopupPointerEdge.Bottom ? pointerSize : 0);
        nint shape = CreateRoundRectRgn(left, top, right + 1, bottom + 1, cornerRadius, cornerRadius);
        if (shape == 0) return;
        Point[] points = edge switch
        {
            RimV.Windows.Application.PopupPointerEdge.Bottom =>
                [new() { X = pointerOffset - 12, Y = bottom - 1 }, new() { X = pointerOffset + 12, Y = bottom - 1 }, new() { X = pointerOffset, Y = height }],
            RimV.Windows.Application.PopupPointerEdge.Top =>
                [new() { X = pointerOffset - 12, Y = top + 1 }, new() { X = pointerOffset + 12, Y = top + 1 }, new() { X = pointerOffset, Y = 0 }],
            RimV.Windows.Application.PopupPointerEdge.Left =>
                [new() { X = left + 1, Y = pointerOffset - 12 }, new() { X = left + 1, Y = pointerOffset + 12 }, new() { X = 0, Y = pointerOffset }],
            _ =>
                [new() { X = right - 1, Y = pointerOffset - 12 }, new() { X = right - 1, Y = pointerOffset + 12 }, new() { X = width, Y = pointerOffset }],
        };
        nint pointer = CreatePolygonRgn(points, points.Length, 2);
        if (pointer != 0)
        {
            CombineRgn(shape, shape, pointer, 2);
            DeleteObject(pointer);
        }
        if (SetWindowRgn(window, shape, true) == 0) DeleteObject(shape);
    }

    internal static void ClipSelectorPopup(nint window, int width, int height, int cornerRadius,
        int pointerSize, int pointerOffset, RimV.Windows.Application.PopupPointerEdge edge)
    {
        int left = edge == RimV.Windows.Application.PopupPointerEdge.Left ? pointerSize : 0;
        int right = width - (edge == RimV.Windows.Application.PopupPointerEdge.Right ? pointerSize : 0);
        nint shape = CreateRoundRectRgn(left, 0, right + 1, height + 1, cornerRadius, cornerRadius);
        if (shape == 0) return;
        Point[] points = edge == RimV.Windows.Application.PopupPointerEdge.Left
            ? [new() { X = left + 1, Y = pointerOffset - 12 }, new() { X = left + 1, Y = pointerOffset + 12 }, new() { X = 0, Y = pointerOffset }]
            : [new() { X = right - 1, Y = pointerOffset - 12 }, new() { X = right - 1, Y = pointerOffset + 12 }, new() { X = width, Y = pointerOffset }];
        nint pointer = CreatePolygonRgn(points, points.Length, 2);
        if (pointer != 0)
        {
            CombineRgn(shape, shape, pointer, 2);
            DeleteObject(pointer);
        }
        if (SetWindowRgn(window, shape, true) == 0) DeleteObject(shape);
    }

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
    private static extern nint GetWindowLongPtr(nint window, int index);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
    private static extern nint SetWindowLongPtr(nint window, int index, nint value);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(nint window, nint insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern nint CreateRoundRectRgn(int left, int top, int right, int bottom, int ellipseWidth, int ellipseHeight);

    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern nint CreatePolygonRgn([In] Point[] points, int count, int fillMode);

    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern int CombineRgn(nint destination, nint source1, nint source2, int mode);

    [DllImport("gdi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DeleteObject(nint handle);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern int SetWindowRgn(nint window, nint region, [MarshalAs(UnmanagedType.Bool)] bool redraw);

    internal static bool ActivateProcessWindow(int processId, string expectedTitle)
    {
        nint target = 0;
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out uint ownerProcessId);
            if (ownerProcessId != (uint)processId) return true;

            StringBuilder title = new(256);
            GetWindowText(window, title, title.Capacity);
            if (!string.Equals(title.ToString(), expectedTitle, StringComparison.Ordinal)) return true;

            target = window;
            return false;
        }, 0);

        if (target == 0) return false;
        ShowWindow(target, ShowWindowRestore);
        SetForegroundWindow(target);
        return true;
    }

    private const int ShowWindowRestore = 9;

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private delegate bool EnumWindowsCallback(nint window, nint parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowsCallback callback, nint parameter);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(nint window, out uint processId);

    [DllImport("user32.dll", EntryPoint = "GetWindowTextW", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(nint window, StringBuilder text, int maximumCount);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowWindow(nint window, int command);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(nint window);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    internal static extern int SetCurrentProcessExplicitAppUserModelID(string appId);
}
