using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using Windows.ApplicationModel.DataTransfer;
using Windows.Media.Core;
using Windows.Media.Playback;
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace RimV.Windows;

internal sealed record TranscriptRow(string Key, ulong StartMs, string Text, string StateText, bool IsFinal);

public sealed partial class TranscriptionWindow : Window
{
    private readonly AppCoordinator _coordinator;
    private readonly ObservableCollection<TranscriptRow> _rows = [];
    private readonly Dictionary<string, TranscriptRow> _byKey = new(StringComparer.Ordinal);
    private readonly MediaPlayer _player = new();
    private string? _sessionId;
    private RecordingSummary? _summary;
    private bool _loading;
    private bool _finalizedLoaded;
    private string _previousStatus = "idle";

    public TranscriptionWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        WindowHelpers.Configure(this, 920, 700);
        TranscriptList.ItemsSource = _rows;
        AudioPlayer.SetMediaPlayer(_player);
        _coordinator.CoreEventReceived += CoreEventReceived;
        _coordinator.Changed += Coordinator_Changed;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.CoreEventReceived -= CoreEventReceived;
            _coordinator.Changed -= Coordinator_Changed;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
            _player.Pause();
            _player.Source = null;
            _player.Dispose();
        };
    }

    public async void ShowSession(string sessionId)
    {
        _sessionId = sessionId;
        _summary = null;
        _finalizedLoaded = false;
        _previousStatus = _coordinator.Snapshot.Status;
        _rows.Clear();
        _byKey.Clear();
        SessionTitle.Text = "Transcript";
        SessionStatus.Text = "Loading recording…";
        EmptyText.Text = "Your transcript will appear here while listening.";
        EmptyText.Visibility = Visibility.Visible;
        TranscriptList.Visibility = Visibility.Collapsed;
        AudioPlayer.Visibility = Visibility.Collapsed;
        _player.Pause();
        _player.Source = null;
        AudioPlayer.SetMediaPlayer(_player);
        Activate();
        await LoadSessionAsync(sessionId);
    }

    private async Task LoadSessionAsync(string sessionId)
    {
        _loading = true;
        try
        {
            long revision;
            RecordingDetails details;
            int attempts = 0;
            do
            {
                revision = _coordinator.TranscriptRevision;
                details = await _coordinator.GetRecordingAsync(sessionId);
                attempts++;
            }
            while (details.Summary.State == "recording" && revision != _coordinator.TranscriptRevision && attempts < 3);
            if (_sessionId != sessionId) return;
            _summary = details.Summary;
            _finalizedLoaded = details.Summary.State == "completed";
            DeleteButton.IsEnabled = _finalizedLoaded;
            DeleteButton.Visibility = _finalizedLoaded ? Visibility.Visible : Visibility.Collapsed;
            SessionTitle.Text = details.Summary.Title;
            SessionStatus.Text = details.Summary.State == "recording"
                ? "Recording · this window can close without stopping capture"
                : $"Saved recording · {details.Transcript.Count} transcript lines";
            foreach (TranscriptLine line in details.Transcript)
            {
                string key = MakeKey(line.Source, line.StartMs);
                AddOrReplace(new TranscriptRow(key, line.StartMs, line.Text, $"{line.StartMs / 1000d:0.0}s · {FriendlySource(line.Source)}", true));
            }
            UpdateEmptyState();
            if (details.Summary.State == "completed") ConfigurePlayback(sessionId);
        }
        catch (Exception error)
        {
            _coordinator.ReportError(error);
            ErrorInfo.Message = "RimV couldn't open this recording. Refresh Recordings and try again.";
            ErrorInfo.IsOpen = true;
        }
        finally
        {
            _loading = false;
        }
    }

    private void ConfigurePlayback(string sessionId)
    {
        string root = _coordinator.RecordingsDirectory;
        string directory = Path.Combine(root, sessionId);
        string? path = new[] { "microphone.wav", "system.wav" }
            .Select(name => Path.Combine(directory, name))
            .FirstOrDefault(File.Exists);
        if (path is null) return;
        _player.Source = MediaSource.CreateFromUri(new Uri(path));
        AudioPlayer.Visibility = Visibility.Visible;
    }

    private void CoreEventReceived(CoreEvent item)
    {
        if (item.Type != "transcript_update" || _sessionId is null || _coordinator.Snapshot.Session?.Id != _sessionId) return;
        TranscriptUpdate? update = item.Payload.GetProperty("update").Deserialize<TranscriptUpdate>();
        if (update is null) return;
        string key = MakeKey(update.Source, update.StartMs);
        string text = string.Join(" ", new[] { update.StableText, update.UnstableText }.Where(value => !string.IsNullOrWhiteSpace(value)));
        if (string.IsNullOrWhiteSpace(text))
        {
            Remove(key);
            return;
        }
        AddOrReplace(new TranscriptRow(key, update.StartMs, text,
            $"{update.StartMs / 1000d:0.0}s · {FriendlySource(update.Source)} · {(update.IsFinal ? "Final" : "Live draft")}", update.IsFinal));
        SessionStatus.Text = update.IsFinal ? "Listening · final transcript updated" : "Listening · transcript updating";
        UpdateEmptyState();
    }

    private void Coordinator_Changed(object? sender, EventArgs e)
    {
        if (_loading || _sessionId is null) return;
        string status = _coordinator.Snapshot.Status;
        bool finalizedNow = status == "idle" && (_previousStatus is "recording" or "stopping");
        _previousStatus = status;
        if (!_finalizedLoaded && finalizedNow && _coordinator.Snapshot.Session?.Id == _sessionId)
            _ = LoadSessionAsync(_sessionId);
    }

    private void AddOrReplace(TranscriptRow row)
    {
        if (_byKey.TryGetValue(row.Key, out TranscriptRow? previous))
        {
            int index = _rows.IndexOf(previous);
            if (index >= 0) _rows[index] = row;
        }
        else
        {
            int index = 0;
            while (index < _rows.Count && _rows[index].StartMs <= row.StartMs) index++;
            _rows.Insert(index, row);
        }
        _byKey[row.Key] = row;
    }

    private void Remove(string key)
    {
        if (_byKey.Remove(key, out TranscriptRow? previous)) _rows.Remove(previous);
        UpdateEmptyState();
    }

    private void UpdateEmptyState()
    {
        bool empty = _rows.Count == 0;
        EmptyText.Visibility = empty ? Visibility.Visible : Visibility.Collapsed;
        TranscriptList.Visibility = empty ? Visibility.Collapsed : Visibility.Visible;
        EmptyText.Text = _summary?.State == "recording"
            ? "Speak to see your transcript here. Partial text is marked while it is being recognized."
            : "There is no saved transcript for this recording.";
    }

    private async void ExportTxtButton_Click(object sender, RoutedEventArgs e) => await ExportAsync("txt");
    private async void ExportJsonButton_Click(object sender, RoutedEventArgs e) => await ExportAsync("json");

    private async void RenameButton_Click(object sender, RoutedEventArgs e)
    {
        if (_summary is null || _sessionId is null) return;
        var field = new TextBox { Text = _summary.Title, MaxLength = 120, Header = "Recording name" };
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
        await _coordinator.RenameRecordingAsync(_sessionId, field.Text);
        RecordingSummary? renamed = _coordinator.Recordings.FirstOrDefault(item => item.SessionId == _sessionId);
        if (renamed is not null)
        {
            _summary = renamed;
            SessionTitle.Text = renamed.Title;
        }
    }

    private async void DeleteButton_Click(object sender, RoutedEventArgs e)
    {
        if (_summary is null || !NativeWindowsPolicy.CanDeleteRecording(_summary.State) || _sessionId is null) return;
        var dialog = new ContentDialog
        {
            Title = "Delete this recording?",
            Content = $"\"{_summary.Title}\" and its saved transcript and audio will be permanently removed.",
            PrimaryButtonText = "Delete recording",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = RootGrid.XamlRoot,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary) return;
        await _coordinator.DeleteRecordingAsync(_sessionId);
        if (!_coordinator.Recordings.Any(item => item.SessionId == _sessionId)) Close();
    }

    private async Task ExportAsync(string format)
    {
        if (_sessionId is null || _summary?.State != "completed") return;
        var picker = new FileSavePicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName = $"{_summary.Title}.{format}",
        };
        picker.FileTypeChoices.Add(format == "json" ? "JSON transcript" : "Text transcript", new List<string> { $".{format}" });
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        StorageFile? file = await picker.PickSaveFileAsync();
        if (file is null) return;
        try
        {
            string content = await _coordinator.ExportRecordingAsync(_sessionId, format);
            await FileIO.WriteTextAsync(file, content);
        }
        catch (Exception error)
        {
            _coordinator.ReportError(error);
            ErrorInfo.Message = "RimV couldn't export this transcript. Choose another location and try again.";
            ErrorInfo.IsOpen = true;
        }
    }

    private void CopyButton_Click(object sender, RoutedEventArgs e)
    {
        string text = string.Join("\n\n", _rows.Select(item => item.Text));
        if (text.Length == 0) return;
        var package = new DataPackage();
        package.SetText(text);
        Clipboard.SetContent(package);
    }

    private static string MakeKey(string source, ulong start) => $"{source}:{start}";
    private static string FriendlySource(string source) => source.ToLowerInvariant() switch
    {
        "system" => "System audio",
        "systemaudio" => "System audio",
        _ => "Microphone",
    };
}
