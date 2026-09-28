using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Shapes;

namespace RimV.Windows;

public sealed class LanguageSelectorRow
{
    public string Code { get; init; } = "";
    public string Name { get; init; } = "";
    public ImageSource? Flag { get; init; }
    public Visibility FlagVisibility { get; init; }
    public Visibility AutoDetectVisibility { get; init; }
    public string Checkmark { get; init; } = "";
    public string AccessibleName { get; init; } = "";
    public Brush Background { get; init; } = new SolidColorBrush(Colors.Transparent);
}

public sealed partial class LanguageWindow : Window
{
    private static readonly Dictionary<string, ImageSource> FlagSources = new(StringComparer.OrdinalIgnoreCase);
    private readonly AppCoordinator _coordinator;
    private List<string> _languages = [];
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
        _supportsDetection = installed && selected!.Descriptor.Capabilities.SupportsLanguageDetection;
        CurrentLanguage.Text = _coordinator.SelectedLanguage is { Length: > 0 } code
            ? LanguageSelectorPolicy.GetName(code)
            : _supportsDetection ? "Auto Detect" : "Default";
        _updating = true;
        RebuildList();
        _updating = false;
    }

    private void RebuildList()
    {
        string query = SearchBox.Text;
        IEnumerable<LanguageOption> options = LanguageSelectorPolicy.Search(_languages, query);
        LanguageOption[] all = options.ToArray();
        string selectedCode = _coordinator.SelectedLanguage ?? "auto";
        List<LanguageOption> popular = [];
        bool searching = !string.IsNullOrWhiteSpace(query);
        bool autoMatches = !searching || "auto detect".Contains(query.Trim(), StringComparison.CurrentCultureIgnoreCase);
        if (_supportsDetection && autoMatches) popular.Add(new LanguageOption("auto", "Auto Detect", null));
        popular.AddRange(all.Take(10));
        List<LanguageOption> remaining = all.Skip(10).ToList();
        bool showAll = searching || _showAll;
        if (searching)
        {
            popular.Clear();
            if (_supportsDetection && autoMatches) popular.Add(new LanguageOption("auto", "Auto Detect", null));
            popular.AddRange(all);
        }

        PopularList.ItemsSource = popular.Select(item => ToRow(item, selectedCode)).ToList();
        bool hasRemaining = !searching && remaining.Count > 0;
        AllHeader.Visibility = showAll && hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllList.Visibility = showAll && hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllList.ItemsSource = hasRemaining ? remaining.Select(item => ToRow(item, selectedCode)).ToList() : [];
        AllLanguagesButton.Visibility = !searching && hasRemaining ? Visibility.Visible : Visibility.Collapsed;
        AllLanguagesButton.Content = _showAll ? "Popular languages  ⌃" : "All supported languages  ›";
        PopularHeader.Text = searching ? "SUPPORTED LANGUAGES" : "POPULAR LANGUAGES";
        EmptyText.Visibility = popular.Count == 0 && remaining.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        if (_languages.Count == 0)
            EmptyText.Text = _coordinator.SelectedModelId is null
                ? "Choose and install a model before selecting a language."
                : "The selected model does not report supported languages.";
        else EmptyText.Text = "No supported languages match this search.";
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
        string accessibleName = selected ? $"{option.Name}, selected" : option.Name;
        string? flagRegion = option.FlagRegion;
        ImageSource? flag = null;
        if (flagRegion is not null)
        {
            if (!FlagSources.TryGetValue(flagRegion, out flag))
            {
                string flagPath = Path.Combine(AppContext.BaseDirectory, "Assets", "Flags", $"{flagRegion}.svg");
                flag = new SvgImageSource(new Uri(Path.GetFullPath(flagPath)));
                FlagSources.Add(flagRegion, flag);
            }
        }
        bool autoDetect = option.Code == "auto";
        return new LanguageSelectorRow
        {
            Code = option.Code,
            Name = option.Name,
            Flag = flag,
            FlagVisibility = flag is null ? Visibility.Collapsed : Visibility.Visible,
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
