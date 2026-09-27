using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;

namespace RimV.Windows;

internal sealed record RecordingRow(string Id, string Title, string Subtitle, string State);

public sealed partial class RecordingsWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly Action<string> _open;
    private readonly ObservableCollection<RecordingRow> _rows = [];
    private RecordingSummary? _selected;
    private bool _refreshing;

    public RecordingsWindow(AppCoordinator coordinator, Action<string> open)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _open = open;
        WindowHelpers.Configure(this, 760, 620);
        RecordingList.ItemsSource = _rows;
        _coordinator.Changed += Coordinator_Changed;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.Changed -= Coordinator_Changed;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
        };
        Render();
        _ = _coordinator.RefreshRecordingsAsync();
    }

    private void Coordinator_Changed(object? sender, EventArgs args) => Render();

    private void Render()
    {
        string? selectedId = _selected?.SessionId;
        IEnumerable<RecordingSummary> items = _coordinator.Recordings;
        string query = SearchBox.Text.Trim();
        if (query.Length > 0)
            items = items.Where(item => item.Title.Contains(query, StringComparison.CurrentCultureIgnoreCase)
                || item.SessionId.Contains(query, StringComparison.OrdinalIgnoreCase));

        _refreshing = true;
        _rows.Clear();
        foreach (RecordingSummary recording in items)
        {
            string date = DateTimeOffset.FromUnixTimeMilliseconds(checked((long)recording.StartedAtUnixMs)).ToLocalTime().ToString("g");
            string duration = TimeSpan.FromMilliseconds(recording.DurationMs).ToString(recording.DurationMs >= 3_600_000 ? @"h\:mm\:ss" : @"m\:ss");
            string source = string.Join(" + ", recording.Sources.Select(name => name == "system" ? "System audio" : "Microphone"));
            string subtitle = recording.State == "recording" ? $"Recording now · {source}" : $"{date} · {duration} · {source}";
            _rows.Add(new RecordingRow(recording.SessionId, recording.Title, subtitle, recording.State));
        }
        RecordingList.SelectedItem = _rows.FirstOrDefault(row => row.Id == selectedId);
        _selected = _coordinator.Recordings.FirstOrDefault(item => item.SessionId == (RecordingList.SelectedItem as RecordingRow)?.Id);
        _refreshing = false;
        EmptyText.Text = _coordinator.Recordings.Count == 0
            ? "No recordings yet. Start listening from the RimV tray menu."
            : "No recordings match this search.";
        EmptyText.Visibility = _rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        RecordingList.Visibility = _rows.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
        ErrorInfo.Message = _coordinator.ErrorMessage ?? "";
        ErrorInfo.IsOpen = _coordinator.ErrorMessage is not null;
        bool hasSelection = _selected is not null;
        OpenButton.IsEnabled = hasSelection;
        RenameButton.IsEnabled = hasSelection;
        DeleteButton.IsEnabled = hasSelection && NativeWindowsPolicy.CanDeleteRecording(_selected!.State);
        if (_selected?.State == "recording") ToolTipService.SetToolTip(DeleteButton, "Active recordings cannot be deleted.");
        else DeleteButton.ClearValue(ToolTipService.ToolTipProperty);
    }

    private void RecordingList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_refreshing) return;
        string? id = (RecordingList.SelectedItem as RecordingRow)?.Id;
        _selected = _coordinator.Recordings.FirstOrDefault(item => item.SessionId == id);
        Render();
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e) => Render();
    private async void RefreshButton_Click(object sender, RoutedEventArgs e) => await _coordinator.RefreshRecordingsAsync();

    private void OpenButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is not null) _open(_selected.SessionId);
    }

    private async void RenameButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is null) return;
        var field = new TextBox { Text = _selected.Title, MaxLength = 120, Header = "Recording name" };
        var dialog = new ContentDialog
        {
            Title = "Rename recording",
            Content = field,
            PrimaryButtonText = "Save",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary) return;
        await _coordinator.RenameRecordingAsync(_selected.SessionId, field.Text);
    }

    private async void DeleteButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is null || _selected.State != "completed") return;
        var dialog = new ContentDialog
        {
            Title = "Delete this recording?",
            Content = $"\"{_selected.Title}\" and its saved transcript and audio will be permanently removed.",
            PrimaryButtonText = "Delete recording",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await _coordinator.DeleteRecordingAsync(_selected.SessionId);
    }
}
