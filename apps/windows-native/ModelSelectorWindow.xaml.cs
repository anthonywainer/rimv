using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;

namespace RimV.Windows;

public sealed class ModelSelectorRow
{
    public string Id { get; init; } = "";
    public string Name { get; init; } = "";
    public string Status { get; init; } = "";
    public string Checkmark { get; init; } = "";
    public bool CanSelect { get; init; }
    public string AccessibleName { get; init; } = "";
}

public sealed partial class ModelSelectorWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly Action _manageModels;

    public ModelSelectorWindow(AppCoordinator coordinator, Action manageModels)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _manageModels = manageModels;
        _coordinator.Changed += Coordinator_Changed;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.Changed -= Coordinator_Changed;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        };
        Render();
        _ = _coordinator.RefreshModelsAsync();
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
        string current = _coordinator.SelectedModelId ?? "";
        bool idle = _coordinator.Snapshot.Status == "idle";
        ModelRecord[] installed = ModelSelectorPolicy.InstalledReady(_coordinator.Models).ToArray();
        ModelSelectorRow[] rows = installed.Select(model =>
        {
            bool selected = model.Descriptor.Id == current;
            string status = selected ? "Installed · Selected" : "Installed and ready";
            return new ModelSelectorRow
            {
                Id = model.Descriptor.Id,
                Name = model.Descriptor.DisplayName,
                Status = status,
                Checkmark = selected ? "✓" : "",
                CanSelect = idle,
                AccessibleName = $"{model.Descriptor.DisplayName}, {status}",
            };
        }).ToArray();
        ModelList.ItemsSource = rows;
        EmptyText.Visibility = rows.Length == 0 ? Visibility.Visible : Visibility.Collapsed;
        EmptyText.Text = rows.Length == 0
            ? "No models are installed yet. Manage Models to download one."
            : "";
    }

    private async void ModelOption_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string id } || string.IsNullOrWhiteSpace(id)) return;
        if (id == _coordinator.SelectedModelId)
        {
            App.CurrentApp.CloseSelectorPopup();
            return;
        }
        await _coordinator.SelectModelAsync(id);
        if (_coordinator.ErrorMessage is null) App.CurrentApp.CloseSelectorPopup();
    }

    private void ManageModels_Click(object sender, RoutedEventArgs e)
    {
        App.CurrentApp.CloseSelectorPopup();
        _manageModels();
    }

    private void RootGrid_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != global::Windows.System.VirtualKey.Escape) return;
        App.CurrentApp.CloseSelectorPopup();
        e.Handled = true;
    }
}
