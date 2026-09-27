using System.Runtime.InteropServices;

namespace RimV.Windows;

internal sealed class TrayIcon : IDisposable
{
    private const uint CallbackMessage = NativeMethods.WM_APP + 31;
    private const uint ImageIcon = 1;
    private const uint LoadFromFile = 0x0010;
    private const uint DefaultSize = 0x0040;
    private const uint NimAdd = 0;
    private const uint NimModify = 1;
    private const uint NimDelete = 2;
    private const uint NimSetVersion = 4;
    private const uint NifMessage = 1;
    private const uint NifIcon = 2;
    private const uint NifTip = 4;
    private const int SubclassId = 0x52494D56;
    private const uint NinSelect = 0x0400;
    private const uint NinKeySelect = 0x0401;
    private const uint WmContextMenu = 0x007B;
    private readonly ShellWindow _window;
    private readonly NativeMethods.SubclassProc _subclass;
    private nint _icon;
    private bool _initialized;
    private bool _subclassAttached;

    public TrayIcon(ShellWindow window)
    {
        _window = window;
        _subclass = WindowProcedure;
        _window.Coordinator.Changed += Coordinator_Changed;
    }

    public void Initialize()
    {
        NativeMethods.SetCurrentProcessExplicitAppUserModelID("dev.rimv.native.windows");
        string iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", "rimv.ico");
        _icon = NativeMethods.LoadImage(0, iconPath, ImageIcon, 0, 0, LoadFromFile | DefaultSize);
        if (_icon == 0) throw new InvalidOperationException("RimV's notification-area icon could not be loaded.");

        if (!NativeMethods.SetWindowSubclass(_window.WindowHandle, _subclass, (nuint)SubclassId, 0))
            throw new InvalidOperationException("RimV couldn't connect to the Windows notification area.");
        _subclassAttached = true;

        var data = CreateData();
        if (!ShellNotifyIcon(NimAdd, ref data))
            throw new InvalidOperationException("Windows couldn't add RimV to the notification area.");
        data.Version = 4;
        ShellNotifyIcon(NimSetVersion, ref data);
        _initialized = true;
        UpdateTooltip();
    }

    private nint WindowProcedure(nint hwnd, uint message, nuint wParam, nint lParam, nuint subclassId, nuint referenceData)
    {
        if (message == CallbackMessage)
        {
            // Notification icon v4 packs the event in the low word of lParam;
            // the high word contains the icon identifier.
            uint action = unchecked((uint)lParam) & 0xffff;
            if (action is NinSelect or NinKeySelect or WmContextMenu or 0x0202 or 0x0205) // Both buttons open the same compact menu.
                App.CurrentApp.TogglePopup();
            return 0;
        }
        return NativeMethods.DefSubclassProc(hwnd, message, wParam, lParam);
    }

    private NotifyIconData CreateData() => new()
    {
        Size = (uint)Marshal.SizeOf<NotifyIconData>(),
        Window = _window.WindowHandle,
        Id = 1,
        Flags = NifMessage | NifIcon | NifTip,
        CallbackMessage = CallbackMessage,
        Icon = _icon,
        Tip = "RimV",
        Info = "",
        InfoTitle = "",
    };

    private void Coordinator_Changed(object? sender, EventArgs e) => UpdateTooltip();

    private void UpdateTooltip()
    {
        if (!_initialized) return;
        string status = _window.Coordinator.Snapshot.Status switch
        {
            "recording" => "Listening",
            "starting" => "Starting listening",
            "stopping" => "Finishing recording",
            "error" => "Audio error",
            _ => "Ready",
        };
        NotifyIconData data = CreateData();
        data.Tip = $"RimV — {status}";
        ShellNotifyIcon(NimModify, ref data);
    }

    public void Dispose()
    {
        if (_initialized)
        {
            NotifyIconData data = CreateData();
            ShellNotifyIcon(NimDelete, ref data);
            NativeMethods.RemoveWindowSubclass(_window.WindowHandle, _subclass, (nuint)SubclassId);
            _initialized = false;
        }
        if (_subclassAttached)
        {
            NativeMethods.RemoveWindowSubclass(_window.WindowHandle, _subclass, (nuint)SubclassId);
            _subclassAttached = false;
        }
        if (_icon != 0)
        {
            NativeMethods.DestroyIcon(_icon);
            _icon = 0;
        }
        _window.Coordinator.Changed -= Coordinator_Changed;
    }

    [DllImport("shell32.dll", EntryPoint = "Shell_NotifyIconW", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShellNotifyIcon(uint message, ref NotifyIconData data);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NotifyIconData
    {
        public uint Size;
        public nint Window;
        public uint Id;
        public uint Flags;
        public uint CallbackMessage;
        public nint Icon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string Tip;
        public uint State;
        public uint StateMask;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string Info;
        public uint TimeoutOrVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string InfoTitle;
        public uint InfoFlags;
        public Guid ItemGuid;
        public nint BalloonIcon;

        public uint Version
        {
            readonly get => TimeoutOrVersion;
            set => TimeoutOrVersion = value;
        }
    }
}
