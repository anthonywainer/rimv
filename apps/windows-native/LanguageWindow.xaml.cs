using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;

namespace RimV.Windows;

public sealed class LanguageSelectorRow
{
    public string Code { get; init; } = "";
    public string Name { get; init; } = "";
    public string Flag { get; init; } = "";
    public Visibility FlagVisibility { get; init; }
    public Visibility AutoDetectVisibility { get; init; }
    public string Checkmark { get; init; } = "";
    public string AccessibleName { get; init; } = "";
    public Brush Background { get; init; } = new SolidColorBrush(Colors.Transparent);
}

public sealed partial class LanguageWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private List<string> _languages = [];
    private IReadOnlyList<LanguagePresentation> _presentations = [];
    private bool _supportsDetection;
    private bool _showAll;
    private bool _updating;

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
        RootGrid.Loaded += (_, _) =>
        {
            Render();
            SearchBox.Focus(FocusState.Programmatic);
        };
        Render();
    }

    internal void ApplyPopupPlacement(SelectorPopupPlacement placement, double scale) =>
        WindowHelpers.ApplySelectorPointer(PopupSurface, LeftPointer, RightPointer, placement, scale);

    private void Coordinator_Changed(object? sender, EventArgs args)
    {
        if (App.CurrentApp.UiQueue.HasThreadAccess) Render();
        else App.CurrentApp.UiQueue.TryEnqueue(Render);
    }

    private void Render()
    {
        ModelRecord? selected = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == _coordinator.SelectedModelId);
        bool installed = selected?.State == "installed";
        _languages = installed ? selected!.Descriptor.Languages : [];
        _presentations = _languages.Count > 0 ? CoreClient.PresentLanguages(_languages) : [];
        _supportsDetection = installed && selected!.Descriptor.Capabilities.SupportsLanguageDetection;
        CurrentLanguage.Text = _coordinator.SelectedLanguage is { Length: > 0 } code
            ? PresentTitle(code)
            : _supportsDetection ? "Auto Detect" : "Default";
        _updating = true;
        RebuildList();
        _updating = false;
    }

    private void RebuildList()
    {
        string query = SearchBox.Text;
        string selectedCode = _coordinator.SelectedLanguage ?? "auto";
        LanguageSelectorSections sections = LanguageSelectorPolicy.Build(
            _presentations, query, _supportsDetection, _showAll, selectedCode);
        bool searching = sections.IsSearching;
        bool hasRemaining = !searching && sections.HasRemaining;
        PopularHeader.Text = searching ? "SEARCH RESULTS" : "POPULAR LANGUAGES";
        PopularHeader.Visibility = sections.Popular.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
        PopularList.ItemsSource = sections.Popular.Select(item => ToRow(item, selectedCode)).ToList();
        AllHeader.Visibility = _showAll && hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllList.Visibility = _showAll && hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllList.ItemsSource = _showAll && hasRemaining
            ? sections.Remaining.Select(item => ToRow(item, selectedCode)).ToList() : [];
        AllLanguagesButton.Visibility = hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllLanguagesButton.Content = _showAll ? "Show fewer languages  ⌃" : "Show all supported languages  ⌄";
        EmptyText.Visibility = sections.Popular.Count == 0 && sections.Remaining.Count == 0
            ? Visibility.Visible : Visibility.Collapsed;
        if (_languages.Count == 0)
            EmptyText.Text = _coordinator.SelectedModelId is null
                ? "Choose and install a model before selecting a language."
                : "The selected model does not report supported languages.";
        else EmptyText.Text = "No supported languages match this search.";
    }

    private string PresentTitle(string code)
    {
        LanguagePresentation? presentation = _presentations.FirstOrDefault(item =>
            item.Id.Equals(code, StringComparison.OrdinalIgnoreCase));
        return presentation is null ? code : presentation.RegionName is { Length: > 0 } region
            ? $"{presentation.LanguageName} ({region})" : presentation.LanguageName;
    }

    private LanguageSelectorRow ToRow(LanguageOption option, string selectedCode)
    {
        bool selected = string.Equals(option.Code, selectedCode, StringComparison.OrdinalIgnoreCase);
        bool darkTheme = RootGrid.ActualTheme == ElementTheme.Dark;
        global::Windows.UI.Color fill = selected && !ThemeManager.IsHighContrastEnabled()
            ? darkTheme
                ? global::Windows.UI.Color.FromArgb(255, 23, 59, 57)
                : global::Windows.UI.Color.FromArgb(255, 229, 245, 243)
            : Colors.Transparent;
        string displayName = option.Code == "auto" ? option.Name
            : option.RegionName is { Length: > 0 } region ? $"{option.Name} ({region})" : option.Name;
        string accessibleName = selected ? $"{displayName}, selected" : displayName;
        bool autoDetect = option.Code == "auto";
        return new LanguageSelectorRow
        {
            Code = option.Code,
            Name = displayName,
            Flag = autoDetect ? "" : option.Flag,
            FlagVisibility = autoDetect ? Visibility.Collapsed : Visibility.Visible,
            AutoDetectVisibility = autoDetect ? Visibility.Visible : Visibility.Collapsed,
            Checkmark = selected ? "✓" : "",
            AccessibleName = accessibleName,
            Background = new SolidColorBrush(fill),
        };
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_updating) RebuildList();
    }

    private void AllLanguagesButton_Click(object sender, RoutedEventArgs e)
    {
        _showAll = !_showAll;
        RebuildList();
    }

    private async void LanguageOption_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string code }) return;
        if (code == "auto" && !_supportsDetection) return;
        string? language = code == "auto" ? null : code;
        if (language == _coordinator.SelectedLanguage)
        {
            App.CurrentApp.CloseSelectorPopup();
            return;
        }
        await _coordinator.SelectLanguageAsync(language);
        if (_coordinator.ErrorMessage is null) App.CurrentApp.CloseSelectorPopup();
    }

    private void RootGrid_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != global::Windows.System.VirtualKey.Escape) return;
        App.CurrentApp.CloseSelectorPopup();
        e.Handled = true;
    }
}
