namespace RimV.Windows.Application;

/// <summary>Reconciles one live session's stable utterance updates without appending duplicates.</summary>
public sealed class LiveTranscriptBuffer
{
    private readonly Dictionary<string, TranscriptUpdate> _updates = new(StringComparer.Ordinal);
    private string? _sessionId;
    private ulong _revision;

    public string? SessionId => _sessionId;
    public ulong Revision => _revision;
    public IReadOnlyList<TranscriptUpdate> Updates => _updates.Values
        .OrderBy(update => update.StartMs)
        .ThenBy(update => update.Source, StringComparer.Ordinal)
        .ThenBy(update => update.UtteranceId, StringComparer.Ordinal)
        .ToArray();

    public void Open(string sessionId)
    {
        _sessionId = sessionId;
        _revision = 0;
        _updates.Clear();
    }

    public bool Restore(LiveTranscriptSnapshot snapshot)
    {
        if (_sessionId is null || snapshot.SessionId != _sessionId) return false;
        if (snapshot.Revision < _revision) return false;
        _revision = snapshot.Revision;
        _updates.Clear();
        foreach (TranscriptUpdate update in snapshot.Updates)
        {
            if (HasText(update)) _updates[Key(update)] = update;
        }
        return true;
    }

    public bool Apply(string sessionId, TranscriptUpdate update, ulong revision = 0)
    {
        if (_sessionId != sessionId || string.IsNullOrWhiteSpace(update.UtteranceId)) return false;
        if (revision > 0 && revision < _revision) return false;
        if (revision > _revision) _revision = revision;
        string key = Key(update);
        if (HasText(update)) _updates[key] = update;
        else _updates.Remove(key);
        return true;
    }

    private static bool HasText(TranscriptUpdate update) =>
        !string.IsNullOrWhiteSpace(update.StableText) || !string.IsNullOrWhiteSpace(update.UnstableText);

    private static string Key(TranscriptUpdate update) => $"{update.Source}:{update.UtteranceId}";
}
