using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace RimV.Windows;

public sealed partial class LanguageWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private bool _updating;
    private List<string> _languages = [];

    public LanguageWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _coordinator.Changed += Coordinator_Changed;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.Changed -= Coordinator_Changed;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        };
        Render();
    }

    private void Coordinator_Changed(object? sender, EventArgs args) => Render();

    private void Render()
    {
        ModelRecord? selected = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == _coordinator.SelectedModelId);
        _languages = selected?.State == "installed" ? selected.Descriptor.Languages : [];
        ModelHint.Text = selected is null
            ? "Choose and install a model before selecting a transcription language."
            : $"Languages supported by {selected.Descriptor.DisplayName}.";
        _updating = true;
        RebuildList();
        _updating = false;
    }

    private void RebuildList()
    {
        string query = SearchBox.Text.Trim();
        var values = new List<string> { "Automatic" };
        values.AddRange(_languages.Where(item => item.Contains(query, StringComparison.CurrentCultureIgnoreCase)));
        LanguageList.ItemsSource = values;
        LanguageList.SelectedItem = string.IsNullOrWhiteSpace(_coordinator.SelectedLanguage)
            ? "Automatic"
            : _languages.FirstOrDefault(item => item.Equals(_coordinator.SelectedLanguage, StringComparison.OrdinalIgnoreCase));
        bool none = _languages.Count == 0;
        EmptyText.Visibility = none ? Visibility.Visible : Visibility.Collapsed;
        LanguageList.Visibility = none ? Visibility.Collapsed : Visibility.Visible;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_updating) RebuildList();
    }

    private async void LanguageList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_updating || LanguageList.SelectedItem is not string value) return;
        string? language = value == "Automatic" ? null : value;
        if (language == _coordinator.SelectedLanguage) return;
        await _coordinator.SelectLanguageAsync(language);
    }
}
