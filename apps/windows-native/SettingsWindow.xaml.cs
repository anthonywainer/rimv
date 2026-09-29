using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Reflection;

namespace RimV.Windows;

public sealed partial class SettingsWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private bool _initialized;

    public SettingsWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        WindowHelpers.Configure(this, 560, 560, resizable: false);
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) => App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        DataLocation.Text = $"Local recordings and models: {coordinator.RecordingsDirectory}";
        string version = typeof(App)
            .Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion ?? "0.1.0-beta";
        AppVersionText.Text = $"RimV Native Windows · v{version}";
        ThemeCombo.SelectedItem = ThemeCombo.Items.Cast<ComboBoxItem>().FirstOrDefault(item => (string)item.Tag == coordinator.ThemeName);
        InputDeviceCombo.Items.Add(new ComboBoxItem { Content = "Windows default microphone", Tag = "" });
        foreach (AudioInputDevice device in coordinator.InputDevices)
            InputDeviceCombo.Items.Add(new ComboBoxItem
            {
                Content = device.IsDefault ? $"{device.Name} · Default" : device.Name,
                Tag = device.Id,
            });
        InputDeviceCombo.SelectedItem = InputDeviceCombo.Items.Cast<ComboBoxItem>()
            .FirstOrDefault(item => (string)item.Tag == (coordinator.SelectedMicrophoneDeviceId ?? ""));
        _initialized = true;
    }

    private void ThemeCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || ThemeCombo.SelectedItem is not ComboBoxItem item) return;
        _coordinator.SetTheme((string)item.Tag);
    }

    private async void InputDeviceCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || InputDeviceCombo.SelectedItem is not ComboBoxItem item) return;
        string id = (string)item.Tag;
        await _coordinator.SelectMicrophoneDeviceAsync(id.Length == 0 ? null : id);
    }
}
