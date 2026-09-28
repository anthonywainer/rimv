using Microsoft.UI.Xaml;
using RimV.Windows.Application;
using System.Runtime.InteropServices;
using Windows.UI.ViewManagement;

namespace RimV.Windows;

internal sealed class ThemeManager : IDisposable
{
    private readonly AppCoordinator _coordinator;
    private readonly AccessibilitySettings _accessibility = new();
    private readonly List<WeakReference<FrameworkElement>> _roots = [];
    private bool _highContrastListenerRegistered;

    public ThemeManager(AppCoordinator coordinator)
    {
        _coordinator = coordinator;
        _coordinator.Changed += Coordinator_Changed;
        try
        {
            _accessibility.HighContrastChanged += Accessibility_HighContrastChanged;
            _highContrastListenerRegistered = true;
        }
        catch (COMException error)
        {
            // This view-management event is unavailable in some unpackaged
            // desktop sessions. Keep the system theme fallback and let RimV
            // continue starting rather than failing before its shell appears.
            App.CurrentApp.Log.Error("theme.high_contrast_listener_unavailable", error);
        }
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
    private void Accessibility_HighContrastChanged(AccessibilitySettings sender, object args) => ApplyAll();

    private void ApplyAll()
    {
        foreach (WeakReference<FrameworkElement> item in _roots.ToArray())
            if (item.TryGetTarget(out FrameworkElement? root)) Apply(root);
    }

    private void Apply(FrameworkElement root)
    {
        bool highContrast = false;
        try { highContrast = _accessibility.HighContrast; }
        catch { /* Use the selected system theme if accessibility settings are unavailable. */ }
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
        if (_highContrastListenerRegistered)
            _accessibility.HighContrastChanged -= Accessibility_HighContrastChanged;
        _roots.Clear();
    }
}
