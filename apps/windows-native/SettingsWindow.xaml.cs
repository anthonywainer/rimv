using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace RimV.Windows;

public sealed partial class SettingsWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private bool _initialized;

    public SettingsWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        WindowHelpers.Configure(this, 520, 480, resizable: false);
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) => App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        DataLocation.Text = $"Local recordings and models: {coordinator.RecordingsDirectory}";
        ThemeCombo.SelectedItem = ThemeCombo.Items.Cast<ComboBoxItem>().FirstOrDefault(item => (string)item.Tag == coordinator.ThemeName);
        _initialized = true;
    }

    private void ThemeCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initialized || ThemeCombo.SelectedItem is not ComboBoxItem item) return;
        _coordinator.SetTheme((string)item.Tag);
    }
}
