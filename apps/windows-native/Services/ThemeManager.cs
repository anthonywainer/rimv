using Microsoft.UI.Xaml;
using RimV.Windows.Application;
using System.Runtime.InteropServices;

namespace RimV.Windows;

internal sealed class ThemeManager : IDisposable
{
    private readonly AppCoordinator _coordinator;
    private readonly List<WeakReference<FrameworkElement>> _roots = [];
    private readonly Timer _appearanceTimer;
    private bool? _lastHighContrast;

    public ThemeManager(AppCoordinator coordinator)
    {
        _coordinator = coordinator;
        _coordinator.Changed += Coordinator_Changed;
        _appearanceTimer = new Timer(
            _ => App.CurrentApp.UiQueue.TryEnqueue(() =>
            {
                bool highContrast = IsHighContrastEnabled();
                if (_lastHighContrast == highContrast) return;
                _lastHighContrast = highContrast;
                ApplyAll();
            }),
            null,
            TimeSpan.FromSeconds(1),
            TimeSpan.FromSeconds(1));
    }

    public void RegisterRoot(FrameworkElement root)
    {
        _roots.RemoveAll(item => !item.TryGetTarget(out _));
        if (!_roots.Any(item => item.TryGetTarget(out FrameworkElement? current) && ReferenceEquals(current, root)))
            _roots.Add(new WeakReference<FrameworkElement>(root));
        Apply(root);
    }

    public void UnregisterRoot(FrameworkElement root) =>
        _roots.RemoveAll(item => !item.TryGetTarget(out FrameworkElement? current) || ReferenceEquals(current, root));

    private void Coordinator_Changed(object? sender, EventArgs args) => ApplyAll();
    private static bool IsHighContrastEnabled()
    {
        HighContrast settings = new() { Size = (uint)Marshal.SizeOf<HighContrast>() };
        return SystemParametersInfo(0x0042, settings.Size, ref settings, 0)
            && (settings.Flags & 0x00000001) != 0;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct HighContrast
    {
        public uint Size;
        public uint Flags;
        public nint DefaultScheme;
    }

    [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SystemParametersInfo(uint action, uint parameter, ref HighContrast settings, uint updateProfile);

    private void ApplyAll()
    {
        foreach (WeakReference<FrameworkElement> item in _roots.ToArray())
            if (item.TryGetTarget(out FrameworkElement? root)) Apply(root);
    }

    private void Apply(FrameworkElement root)
    {
        bool highContrast = IsHighContrastEnabled();
        root.RequestedTheme = highContrast ? ElementTheme.Default : _coordinator.ThemeName switch
        {
            "light" => ElementTheme.Light,
            "dark" => ElementTheme.Dark,
            _ => ElementTheme.Default,
        };
    }

    public void Dispose()
    {
        _coordinator.Changed -= Coordinator_Changed;
        _appearanceTimer.Dispose();
        _roots.Clear();
    }
}
