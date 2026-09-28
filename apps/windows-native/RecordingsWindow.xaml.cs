using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using System.Diagnostics;
using System.Globalization;

namespace RimV.Windows;

public sealed class RecordingSelectorRow
{
    public string Id { get; init; } = "";
    public string Title { get; init; } = "";
    public string Subtitle { get; init; } = "";
    public string Icon { get; init; } = "";
    public bool CanDelete { get; init; }
    public string AccessibleName { get; init; } = "";
}

public sealed partial class RecordingsWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly Action<string> _open;
    private readonly List<RecordingSelectorRow> _visibleRows = [];
    private RecordingFilter _filter = RecordingFilter.All;
    private bool _refreshing;
    private string? _localError;

    public RecordingsWindow(AppCoordinator coordinator, Action<string> open)
    {
        InitializeComponent();
        _coordinator = coordinator;
        _open = open;
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

    internal void ApplyPopupPlacement(SelectorPopupPlacement placement, double scale) =>
        WindowHelpers.ApplySelectorPointer(PopupSurface, LeftPointer, RightPointer, placement, scale);

    private void Coordinator_Changed(object? sender, EventArgs args)
    {
        if (App.CurrentApp.UiQueue.HasThreadAccess) Render();
        else App.CurrentApp.UiQueue.TryEnqueue(Render);
    }

    private void Render()
    {
        string query = SearchBox.Text;
        RecordingSummary[] filtered = RecordingSelectorPolicy.Filter(_coordinator.Recordings, query, _filter,
            item => item.Title, item => item.SessionId, item => item.State).ToArray();
        _visibleRows.Clear();
        foreach (RecordingSummary item in filtered)
        {
            string date = DateTimeOffset.FromUnixTimeMilliseconds(checked((long)item.StartedAtUnixMs))
                .ToLocalTime().ToString("d MMM yyyy · h:mm tt", CultureInfo.CurrentCulture);
            string duration = TimeSpan.FromMilliseconds(item.DurationMs)
                .ToString(item.DurationMs >= 3_600_000 ? @"h\:mm\:ss" : @"m\:ss", CultureInfo.InvariantCulture);
            string status = item.State == "recording" ? "Recording" : "Completed";
            string subtitle = item.State == "recording" ? $"Recording · {duration}" : $"{date} · {duration} · {status}";
            _visibleRows.Add(new RecordingSelectorRow
            {
                Id = item.SessionId,
                Title = item.Title,
                Subtitle = subtitle,
                Icon = item.State == "recording" ? "\uE768" : "\uE73E",
                CanDelete = NativeWindowsPolicy.CanDeleteRecording(item.State),
                AccessibleName = $"{item.Title}, {subtitle}",
            });
        }
        _refreshing = true;
        RecordingList.ItemsSource = _visibleRows.ToArray();
        _refreshing = false;
        RecordingCount.Text = _coordinator.Recordings.Count == 1 ? "1 item" : $"{_coordinator.Recordings.Count} items";
        EmptyText.Text = _coordinator.Recordings.Count == 0
            ? "No recordings yet. Start listening from the RimV menu."
            : "No recordings match this search or filter.";
        EmptyText.Visibility = _visibleRows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        ErrorInfo.Message = _coordinator.ErrorMessage ?? _localError ?? "";
        ErrorInfo.IsOpen = _coordinator.ErrorMessage is not null || _localError is not null;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_refreshing) Render();
    }

    private void Filter_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not ToggleButton { Tag: string name }) return;
        _filter = name switch
        {
            "Recording" => RecordingFilter.Recording,
            "Completed" => RecordingFilter.Completed,
            _ => RecordingFilter.All,
        };
        AllFilter.IsChecked = _filter == RecordingFilter.All;
        RecordingFilterToggle.IsChecked = _filter == RecordingFilter.Recording;
        CompletedFilter.IsChecked = _filter == RecordingFilter.Completed;
        Render();
    }

    private void OpenRecording_Click(object sender, RoutedEventArgs e)
    {
        string? id = (sender as FrameworkElement)?.Tag as string;
        if (id is not null && _coordinator.Recordings.Any(item => item.SessionId == id)) _open(id);
    }

    private async void RenameRecording_Click(object sender, RoutedEventArgs e)
    {
        RecordingSummary? recording = FindRecording(sender);
        if (recording is null) return;
        var field = new TextBox { Text = recording.Title, MaxLength = 120, Header = "Recording name" };
        var dialog = new ContentDialog
        {
            Title = "Rename recording",
            Content = field,
            PrimaryButtonText = "Save",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await _coordinator.RenameRecordingAsync(recording.SessionId, field.Text);
    }

    private async void DeleteRecording_Click(object sender, RoutedEventArgs e)
    {
        RecordingSummary? recording = FindRecording(sender);
        if (recording is null || !NativeWindowsPolicy.CanDeleteRecording(recording.State)) return;
        var dialog = new ContentDialog
        {
            Title = "Delete this recording?",
            Content = $"\"{recording.Title}\" and its saved transcript and audio will be permanently removed.",
            PrimaryButtonText = "Delete recording",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            await _coordinator.DeleteRecordingAsync(recording.SessionId);
    }

    private RecordingSummary? FindRecording(object sender)
    {
        string? id = (sender as FrameworkElement)?.Tag as string;
        return id is null ? null : _coordinator.Recordings.FirstOrDefault(item => item.SessionId == id);
    }

    private void OpenFolder_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            Directory.CreateDirectory(_coordinator.RecordingsDirectory);
            Process.Start(new ProcessStartInfo(_coordinator.RecordingsDirectory) { UseShellExecute = true });
        }
        catch (Exception error)
        {
            _localError = $"Could not open the recordings folder: {error.Message}";
            Render();
        }
    }

    private void RootGrid_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != global::Windows.System.VirtualKey.Escape) return;
        App.CurrentApp.CloseSelectorPopup();
        e.Handled = true;
    }
}
