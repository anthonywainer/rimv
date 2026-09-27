using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;

namespace RimV.Windows;

internal sealed record ModelRow(string Id, string Name, string State);

public sealed partial class ModelManagerWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly ObservableCollection<ModelRow> _rows = [];
    private ModelRecord? _selected;
    private ModelProgress? _progress;
    private bool _refreshing;

    public ModelManagerWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        ModelList.ItemsSource = _rows;
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
        _progress = progress;
        Render();
        if (progress.Phase is "complete" or "failed" or "cancelled")
            _ = _coordinator.RefreshModelsAsync();
    }

    private void Render()
    {
        string? selectedId = _selected?.Descriptor.Id ?? _coordinator.SelectedModelId;
        _refreshing = true;
        _rows.Clear();
        foreach (ModelRecord model in _coordinator.Models)
        {
            string state = model.State switch
            {
                "installed" => "Installed",
                "downloading" => "Downloading",
                "incomplete" => "Needs repair",
                "unsupported" => "Not available in this Windows build",
                _ => "Available to download",
            };
            _rows.Add(new ModelRow(model.Descriptor.Id, model.Descriptor.DisplayName, state));
        }
        ModelList.SelectedItem = _rows.FirstOrDefault(row => row.Id == selectedId);
        _selected = _coordinator.Models.FirstOrDefault(model => model.Descriptor.Id == (ModelList.SelectedItem as ModelRow)?.Id);
        _refreshing = false;

        ErrorInfo.Message = _coordinator.ErrorMessage ?? "";
        ErrorInfo.IsOpen = _coordinator.ErrorMessage is not null;
        if (_selected is null)
        {
            DetailsText.Text = "Select a model to see its status.";
            ModelActionButton.IsEnabled = false;
            SelectButton.IsEnabled = false;
            DownloadProgress.Visibility = Visibility.Collapsed;
            return;
        }

        DetailsText.Text = _selected.State == "unsupported"
            ? $"{_selected.Descriptor.DisplayName} isn't included in this Windows build. Its inference runtime is pending Windows dependency validation."
            : $"{_selected.Descriptor.DisplayName} · {_selected.Descriptor.Version} · {_selected.Descriptor.Languages.Count} supported languages";
        bool downloading = _selected.State == "downloading";
        bool unsupported = _selected.State == "unsupported";
        ModelActionButton.Content = unsupported ? "Unavailable" : downloading ? "Cancel download" : _selected.State == "installed" ? "Installed" : "Download model";
        ModelActionButton.IsEnabled = NativeWindowsPolicy.CanCancelModelInstall(_selected.State)
            || NativeWindowsPolicy.CanInstallModel(_selected.State);
        SelectButton.IsEnabled = NativeWindowsPolicy.CanSelectModel(_selected.State, _coordinator.Snapshot.Status);

        bool hasProgress = _progress?.ModelId == _selected.Descriptor.Id;
        DownloadProgress.Visibility = downloading && hasProgress && _progress!.TotalBytes is > 0 ? Visibility.Visible : Visibility.Collapsed;
        if (hasProgress && _progress!.TotalBytes is > 0)
        {
            DownloadProgress.Value = Math.Clamp(100d * _progress.DownloadedBytes / _progress.TotalBytes.Value, 0, 100);
            ProgressText.Text = $"{_progress.Phase} · {DownloadProgress.Value:0}%";
        }
        else if (downloading)
        {
            ProgressText.Text = hasProgress ? _progress!.Phase : "Preparing download…";
        }
        else if (_progress?.ModelId == _selected.Descriptor.Id && _progress.Error is not null)
        {
            ProgressText.Text = "Download failed. Retry when you're ready.";
        }
        else
        {
            ProgressText.Text = "";
        }
    }

    private void ModelList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_refreshing) return;
        string? id = (ModelList.SelectedItem as ModelRow)?.Id;
        _selected = _coordinator.Models.FirstOrDefault(model => model.Descriptor.Id == id);
        Render();
    }

    private async void ModelActionButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is null) return;
        if (_selected.State == "downloading") await _coordinator.CancelModelInstallAsync(_selected.Descriptor.Id);
        else await _coordinator.InstallModelAsync(_selected.Descriptor.Id);
        await _coordinator.RefreshModelsAsync();
    }

    private async void SelectButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is null) return;
        await _coordinator.SelectModelAsync(_selected.Descriptor.Id);
        Render();
    }
}
