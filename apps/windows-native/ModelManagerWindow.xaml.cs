using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using Windows.Storage;
using Windows.System;

namespace RimV.Windows;

internal sealed record ModelCardRow(
    string Id,
    string Name,
    string Description,
    string Size,
    string StateText,
    string SelectionLabel,
    string ActionLabel,
    bool ActionEnabled,
    Visibility RemoveVisibility,
    bool RemoveEnabled,
    double ProgressPercent,
    Visibility ProgressVisibility);

public sealed partial class ModelManagerWindow : Window
{
    private const string Available = "available";
    private const string Installed = "installed";
    private const string Settings = "settings";
    private readonly AppCoordinator _coordinator;
    private readonly ObservableCollection<ModelCardRow> _rows = [];
    private readonly Dictionary<string, ModelProgress> _progressByModel = new(StringComparer.Ordinal);
    private string _tab = Available;

    public ModelManagerWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        WindowHelpers.Configure(this, 780, 620);
        ModelList.ItemsSource = _rows;
        StoragePathText.Text = _coordinator.ModelsDirectory;
        _coordinator.Changed += Coordinator_Changed;
        _coordinator.ModelProgressReceived += ModelProgressReceived;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.Changed -= Coordinator_Changed;
            _coordinator.ModelProgressReceived -= ModelProgressReceived;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        };
        Render();
        _ = _coordinator.RefreshModelsAsync();
    }

    private void Coordinator_Changed(object? sender, EventArgs args) => Render();

    private void ModelProgressReceived(ModelProgress progress)
    {
        _progressByModel[progress.ModelId] = progress;
        Render();
        if (progress.Phase is "complete" or "failed" or "cancelled")
            _ = _coordinator.RefreshModelsAsync();
    }

    private void AvailableTab_Click(object sender, RoutedEventArgs e) => SetTab(Available);
    private void InstalledTab_Click(object sender, RoutedEventArgs e) => SetTab(Installed);
    private void SettingsTab_Click(object sender, RoutedEventArgs e) => SetTab(Settings);

    private void SetTab(string tab)
    {
        _tab = tab;
        Render();
    }

    private void Render()
    {
        bool settings = _tab == Settings;
        CatalogPage.Visibility = settings ? Visibility.Collapsed : Visibility.Visible;
        SettingsPage.Visibility = settings ? Visibility.Visible : Visibility.Collapsed;
        PageTitle.Text = _tab switch
        {
            Installed => "Installed Models",
            Settings => "Model Settings",
            _ => "Available Models",
        };
        PageDescription.Text = _tab switch
        {
            Installed => "Choose a locally installed model or remove one you no longer need.",
            Settings => "Choose where RimV stores its downloaded speech models.",
            _ => "Download and manage your local transcription models.",
        };
        AvailableTab.Content = _tab == Available ? "●  Available Models" : "    Available Models";
        InstalledTab.Content = _tab == Installed ? "●  Installed Models" : "    Installed Models";
        SettingsTab.Content = _tab == Settings ? "●  Settings" : "    Settings";
        ErrorInfo.Message = _coordinator.ErrorMessage ?? string.Empty;
        ErrorInfo.IsOpen = _coordinator.ErrorMessage is not null;
        if (settings) return;

        _rows.Clear();
        IEnumerable<ModelRecord> models = _coordinator.Models;
        if (_tab == Installed) models = models.Where(model => model.State == "installed");
        foreach (ModelRecord model in models)
            _rows.Add(CreateRow(model));
        EmptyText.Text = _tab == Installed
            ? "No installed models yet. Choose Available Models to download one."
            : "No models are available in this build.";
        EmptyText.Visibility = _rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private ModelCardRow CreateRow(ModelRecord model)
    {
        ModelDescriptor descriptor = model.Descriptor;
        bool installed = model.State == "installed";
        bool downloading = model.State == "downloading";
        bool unsupported = model.State == "unsupported";
        _progressByModel.TryGetValue(descriptor.Id, out ModelProgress? progress);
        ulong? totalBytes = descriptor.Files
            .Where(file => file.ExpectedSizeBytes.HasValue)
            .Select(file => file.ExpectedSizeBytes!.Value)
            .Aggregate<ulong, ulong?>(0, (total, next) => total + next);
        bool hasSize = totalBytes is > 0;
        string size = hasSize ? FormatSize(totalBytes!.Value) : "Managed package";
        double percent = progress is null ? 0 : NativeWindowsPolicy.DownloadProgressPercent(progress);
        string state = progress?.Phase == "failed"
            ? $"Download failed · {progress.Error ?? "Retry to try again."}"
            : model.State switch
            {
                "installed" => "Installed and ready",
                "downloading" => progress?.Phase == "downloading" ? $"Downloading · {percent:0}%" : "Preparing download…",
                "incomplete" => "Installation is incomplete · retry to repair",
                "unsupported" => "Unavailable in this Windows build",
                _ => $"Supports {descriptor.Languages.Count} languages",
            };
        string action = unsupported ? "Unavailable"
            : downloading ? "Cancel download"
            : installed ? model.Selected ? "Selected" : "Use Model"
            : model.State == "incomplete" ? "Repair download" : "Download";
        bool actionEnabled = downloading
            ? NativeWindowsPolicy.CanCancelModelInstall(model.State)
            : installed
                ? !model.Selected && NativeWindowsPolicy.CanSelectModel(model.State, _coordinator.Snapshot.Status)
                : NativeWindowsPolicy.CanInstallModel(model.State);
        bool canRemove = NativeWindowsPolicy.CanRemoveModel(model, _coordinator.Snapshot.Status);
        return new ModelCardRow(
            descriptor.Id,
            descriptor.DisplayName,
            descriptor.InstallHint ?? $"{descriptor.Backend} speech recognition · {descriptor.Languages.Count} supported languages",
            size,
            state,
            model.Selected ? "Selected" : string.Empty,
            action,
            actionEnabled,
            installed ? Visibility.Visible : Visibility.Collapsed,
            canRemove,
            percent,
            downloading && progress?.TotalBytes is > 0 ? Visibility.Visible : Visibility.Collapsed);
    }

    private async void ModelActionButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string id }) return;
        ModelRecord? model = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == id);
        if (model is null) return;
        if (model.State == "downloading") await _coordinator.CancelModelInstallAsync(id);
        else if (model.State == "installed") await _coordinator.SelectModelAsync(id);
        else await _coordinator.InstallModelAsync(id);
        await _coordinator.RefreshModelsAsync();
    }

    private async void RemoveButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string id }) return;
        ModelRecord? model = _coordinator.Models.FirstOrDefault(item => item.Descriptor.Id == id);
        if (model is null || !NativeWindowsPolicy.CanRemoveModel(model, _coordinator.Snapshot.Status)) return;
        var dialog = new ContentDialog
        {
            Title = "Remove model?",
            Content = $"{model.Descriptor.DisplayName} will be removed from local storage. You can download it again later.",
            PrimaryButtonText = "Remove",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await _coordinator.RemoveModelAsync(id);
    }

    private static string FormatSize(ulong bytes)
    {
        double gigabytes = bytes / 1_000_000_000d;
        return gigabytes >= 1 ? $"{gigabytes:0.0} GB" : $"{bytes / 1_000_000d:0} MB";
    }

    private async void OpenFolderButton_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            StorageFolder folder = await StorageFolder.GetFolderFromPathAsync(_coordinator.ModelsDirectory);
            await Launcher.LaunchFolderAsync(folder);
        }
        catch (Exception error)
        {
            _coordinator.ReportError(error);
            Render();
        }
    }
}
