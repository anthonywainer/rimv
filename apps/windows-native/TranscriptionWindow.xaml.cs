using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using System.Text.Json;
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
    private readonly LiveTranscriptBuffer _liveTranscript = new();
    private readonly RecordingViewerLifecycle _viewerLifecycle = new();
    private readonly MediaPlayer _player = new();
    private string? _sessionId;
    private RecordingSummary? _summary;
    private bool _loading;

    public TranscriptionWindow(AppCoordinator coordinator)
    {
        InitializeComponent();
        _coordinator = coordinator;
        WindowHelpers.Configure(this, 920, 720);
        TranscriptList.ItemsSource = _rows;
        AudioPlayer.SetMediaPlayer(_player);
        _player.PlaybackSession.PlaybackStateChanged += PlaybackSession_Changed;
        _player.PlaybackSession.PositionChanged += PlaybackSession_Changed;
        _coordinator.CoreEventReceived += CoreEventReceived;
        _coordinator.Changed += Coordinator_Changed;
        App.CurrentApp.ThemeManager.RegisterRoot(RootGrid);
        Closed += (_, _) =>
        {
            _coordinator.CoreEventReceived -= CoreEventReceived;
            _coordinator.Changed -= Coordinator_Changed;
            App.CurrentApp.ThemeManager.UnregisterRoot(RootGrid);
            _player.PlaybackSession.PlaybackStateChanged -= PlaybackSession_Changed;
            _player.PlaybackSession.PositionChanged -= PlaybackSession_Changed;
            StopAndResetPlayback();
            _viewerLifecycle.Close();
            _player.Dispose();
        };
    }

    public async void ShowSession(string sessionId)
    {
        long generation = _viewerLifecycle.Open(sessionId);
        _sessionId = sessionId;
        _summary = null;
        _liveTranscript.Open(sessionId);
        _rows.Clear();
        _byKey.Clear();
        SessionTitle.Text = "Loading recording…";
        SessionStatus.Text = "Loading recording…";
        AudioStatus.Text = "Audio unavailable";
        DurationText.Text = "00:00";
        BrandSubtitle.Text = "Transcription viewer";
        BadgeText.Text = "LOADING";
        StatusBadge.Visibility = Visibility.Visible;
        LiveStatusBadge.Visibility = Visibility.Collapsed;
        CompletedStatusBadge.Visibility = Visibility.Collapsed;
        TranscriptTitle.Text = "Transcription";
        FooterText.Text = "Viewer only · Closing this window does not stop recording.";
        EmptyText.Text = "Loading transcript…";
        EmptyText.Visibility = Visibility.Visible;
        TranscriptList.Visibility = Visibility.Collapsed;
        AudioPlayer.Visibility = Visibility.Collapsed;
        LiveAudioText.Visibility = Visibility.Collapsed;
        ExportTxtButton.Visibility = Visibility.Collapsed;
        ExportJsonButton.Visibility = Visibility.Collapsed;
        DeleteButton.Visibility = Visibility.Collapsed;
        ErrorInfo.IsOpen = false;
        StopAndResetPlayback();
        AudioPlayer.SetMediaPlayer(_player);
        Activate();
        await LoadSessionAsync(sessionId, generation);
    }

    private async Task LoadSessionAsync(string sessionId, long generation)
    {
        _loading = true;
        try
        {
            RecordingDetails details = await _coordinator.GetRecordingAsync(sessionId);
            if (!IsCurrentOpen(sessionId, generation)) return;
            _summary = details.Summary;
            bool live = details.Summary.State == "recording";
            SessionTitle.Text = details.Summary.Title;
            Title = $"RimV — {details.Summary.Title}";
            BrandSubtitle.Text = live ? "Live transcription · Audio capture" : "Completed recording · Saved audio";
            StatusBadge.Visibility = Visibility.Collapsed;
            LiveStatusBadge.Visibility = live ? Visibility.Visible : Visibility.Collapsed;
            CompletedStatusBadge.Visibility = live ? Visibility.Collapsed : Visibility.Visible;
            TranscriptTitle.Text = live ? "Live transcription" : "Transcription";
            SessionStatus.Text = live
                ? "Listening · live results update here"
                : $"Processed · {details.Transcript.Count} transcript lines";
            AudioTitle.Text = live ? "Audio capture" : "Recorded audio";
            AudioDescription.Text = live ? CaptureSourceDescription() : "Saved session audio";
            AudioStatus.Text = live ? "Recording" : "Audio unavailable";
            DurationText.Text = live ? FormatDuration(_coordinator.Snapshot.ElapsedMs) : FormatDuration(details.Summary.DurationMs);
            LiveAudioText.Visibility = live ? Visibility.Visible : Visibility.Collapsed;
            DeleteButton.IsEnabled = !live && NativeWindowsPolicy.CanDeleteRecording(details.Summary.State);
            DeleteButton.Visibility = DeleteButton.IsEnabled ? Visibility.Visible : Visibility.Collapsed;
            ExportTxtButton.Visibility = live ? Visibility.Collapsed : Visibility.Visible;
            ExportJsonButton.Visibility = live ? Visibility.Collapsed : Visibility.Visible;
            FooterText.Text = live
                ? "Viewer only · Recording continues if you close this window."
                : "Final transcript · Timestamped source data is preserved in JSON.";

            if (live)
            {
                LiveTranscriptSnapshot snapshot = await _coordinator.GetLiveTranscriptSnapshotAsync();
                if (!IsCurrentOpen(sessionId, generation)) return;
                if (snapshot.SessionId == sessionId)
                {
                    if (_liveTranscript.Restore(snapshot))
                    {
                        _rows.Clear();
                        _byKey.Clear();
                        foreach (TranscriptUpdate update in snapshot.Updates)
                            ApplyTranscriptUpdate(update, snapshot.Revision);
                    }
                }
                EmptyText.Text = "Speak to see your transcript here. Partial text is shown while it is being recognized.";
            }
            else
            {
                foreach (TranscriptLine line in details.Transcript)
                {
                    string key = MakePersistedKey(line.Source, line.StartMs);
                    AddOrReplace(new TranscriptRow(key, line.StartMs, line.Text,
                        $"{line.StartMs / 1000d:0.0}s · {FriendlySource(line.Source)}", true));
                }
                EmptyText.Text = "There is no saved transcript for this recording.";
                await ConfigurePlaybackAsync(sessionId, generation);
            }
            if (!IsCurrentOpen(sessionId, generation)) return;
            UpdateEmptyState();
        }
        catch (Exception error)
        {
            if (!IsCurrentOpen(sessionId, generation)) return;
            _coordinator.ReportError(error);
            ErrorInfo.Message = "RimV couldn't open this recording. Refresh Recordings and try again.";
            ErrorInfo.IsOpen = true;
            EmptyText.Text = "The recording could not be loaded. Close this window and try again.";
        }
        finally
        {
            if (generation == _viewerLifecycle.Generation)
            {
                _loading = false;
                if (_summary?.State == "recording" && _coordinator.Snapshot.Status == "idle")
                    _ = LoadSessionAsync(sessionId, generation);
            }
        }
    }

    private async Task ConfigurePlaybackAsync(string sessionId, long generation)
    {
        string directory = Path.Combine(_coordinator.RecordingsDirectory, sessionId);
        foreach (string name in new[] { "microphone.wav", "system.wav" })
        {
            string path = Path.Combine(directory, name);
            if (!File.Exists(path)) continue;
            StorageFile file = await StorageFile.GetFileFromPathAsync(path);
            if (!IsCurrentOpen(sessionId, generation)) return;
            _player.Source = MediaSource.CreateFromStorageFile(file);
            _player.PlaybackSession.Position = TimeSpan.Zero;
            AudioPlayer.Visibility = Visibility.Visible;
            AudioStatus.Text = $"Saved · {(name == "microphone.wav" ? "Microphone" : "System audio")}";
            return;
        }
        AudioStatus.Text = "Audio unavailable";
    }

    private void CoreEventReceived(CoreEvent item)
    {
        if (item.Type != "transcript_update" || _summary?.State != "recording" || _sessionId is null
            || _coordinator.Snapshot.Session?.Id != _sessionId) return;
        if (item.Payload.TryGetProperty("session_id", out JsonElement sessionId)
            && sessionId.GetString() != _sessionId) return;
        if (!item.Payload.TryGetProperty("update", out JsonElement payload)) return;
        TranscriptUpdate? update = payload.Deserialize<TranscriptUpdate>();
        if (update is null) return;
        ulong revision = item.Payload.TryGetProperty("revision", out JsonElement revisionElement)
            ? revisionElement.GetUInt64()
            : 0;
        ApplyTranscriptUpdate(update, revision);
        SessionStatus.Text = update.IsFinal ? "Listening · final transcript updated" : "Listening · transcript updating";
        UpdateEmptyState();
    }

    private void Coordinator_Changed(object? sender, EventArgs e)
    {
        if (_loading || _summary?.State != "recording" || _sessionId is null) return;
        if (_coordinator.Snapshot.Session?.Id != _sessionId) return;
        DurationText.Text = FormatDuration(_coordinator.Snapshot.ElapsedMs);
        AudioDescription.Text = CaptureSourceDescription();
        if (_coordinator.Snapshot.Status is "idle" or "error")
            _ = LoadSessionAsync(_sessionId, _viewerLifecycle.Generation);
    }

    private void ApplyTranscriptUpdate(TranscriptUpdate update, ulong revision)
    {
        if (_sessionId is null || !_liveTranscript.Apply(_sessionId, update, revision)) return;
        string key = MakeLiveKey(update.Source, update.UtteranceId);
        string text = string.Join(" ", new[] { update.StableText, update.UnstableText }
            .Where(value => !string.IsNullOrWhiteSpace(value)));
        if (string.IsNullOrWhiteSpace(text))
        {
            Remove(key);
            return;
        }
        AddOrReplace(new TranscriptRow(key, update.StartMs, text,
            $"{update.StartMs / 1000d:0.0}s · {FriendlySource(update.Source)} · {(update.IsFinal ? "Final" : "Live draft")}", update.IsFinal));
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
    }

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
        string title = field.Text.Trim();
        if (title.Length is 0 or > 120) return;
        await _coordinator.RenameRecordingAsync(_sessionId, title);
        RecordingSummary? renamed = _coordinator.Recordings.FirstOrDefault(item => item.SessionId == _sessionId);
        if (renamed is not null)
        {
            _summary = renamed;
            SessionTitle.Text = renamed.Title;
            Title = $"RimV — {renamed.Title}";
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
        StopAndResetPlayback();
        await _coordinator.DeleteRecordingAsync(_sessionId);
        if (!_coordinator.Recordings.Any(item => item.SessionId == _sessionId)) Close();
    }

    private async void ExportTxtButton_Click(object sender, RoutedEventArgs e) => await ExportAsync("txt");
    private async void ExportJsonButton_Click(object sender, RoutedEventArgs e) => await ExportAsync("json");

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

    private string CaptureSourceDescription()
    {
        CoreSnapshot state = _coordinator.Snapshot;
        bool mic = state.Microphone.Enabled;
        bool system = state.SystemAudio.Enabled;
        string source = mic && system ? "Microphone + System" : mic ? "Microphone" : system ? "System" : "No source selected";
        return $"Capturing from {source}";
    }

    private bool IsCurrentOpen(string sessionId, long generation) =>
        _viewerLifecycle.IsCurrent(sessionId, generation) && _sessionId == sessionId;

    private void StopAndResetPlayback()
    {
        _player.Pause();
        _player.PlaybackSession.Position = TimeSpan.Zero;
        _player.Source = null;
        _viewerLifecycle.ResetPlayback();
    }

    private void PlaybackSession_Changed(Windows.Media.Playback.MediaPlaybackSession sender, object args) =>
        _viewerLifecycle.SetPlayback(
            sender.PlaybackState == Windows.Media.Playback.MediaPlaybackState.Playing,
            sender.Position);

    private static string MakeLiveKey(string source, string utteranceId) => $"{source}:{utteranceId}";
    private static string MakePersistedKey(string source, ulong start) => $"{source}:{start}";
    private static string FriendlySource(string source) => source.ToLowerInvariant() switch
    {
        "system" or "systemaudio" => "System audio",
        _ => "Microphone",
    };
    private static string FormatDuration(ulong milliseconds)
    {
        TimeSpan duration = TimeSpan.FromMilliseconds(milliseconds);
        return duration.TotalHours >= 1 ? duration.ToString(@"hh\:mm\:ss") : duration.ToString(@"mm\:ss");
    }
}
