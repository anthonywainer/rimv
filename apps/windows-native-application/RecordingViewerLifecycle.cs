namespace RimV.Windows.Application;

/// <summary>Tracks a viewer activation so stale async loads cannot populate a newly opened recording.</summary>
public sealed class RecordingViewerLifecycle
{
    private readonly object _sync = new();
    private long _generation;
    private string? _sessionId;
    private bool _isPlaying;
    private TimeSpan _playbackPosition;

    public string? SessionId { get { lock (_sync) return _sessionId; } }
    public bool IsOpen { get { lock (_sync) return _sessionId is not null; } }
    public bool IsPlaying { get { lock (_sync) return _isPlaying; } }
    public TimeSpan PlaybackPosition { get { lock (_sync) return _playbackPosition; } }
    public long Generation => Interlocked.Read(ref _generation);

    public long Open(string sessionId)
    {
        lock (_sync)
        {
            _sessionId = sessionId;
            ResetPlayback();
            return Interlocked.Increment(ref _generation);
        }
    }

    public void Close()
    {
        lock (_sync)
        {
            _sessionId = null;
            ResetPlayback();
            Interlocked.Increment(ref _generation);
        }
    }

    public bool IsCurrent(string sessionId, long generation)
    {
        lock (_sync) return generation == Generation && _sessionId == sessionId;
    }

    public void SetPlayback(bool isPlaying, TimeSpan position)
    {
        lock (_sync)
        {
            if (_sessionId is null) return;
            _isPlaying = isPlaying;
            _playbackPosition = position < TimeSpan.Zero ? TimeSpan.Zero : position;
        }
    }

    public void ResetPlayback()
    {
        lock (_sync)
        {
            _isPlaying = false;
            _playbackPosition = TimeSpan.Zero;
        }
    }
}
