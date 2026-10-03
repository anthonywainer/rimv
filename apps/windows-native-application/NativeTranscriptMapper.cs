using System.Collections.Concurrent;

namespace RimV.Windows.Application;

/// <summary>Maps Windows streaming callbacks to RimV's replaceable partial/final contract.</summary>
public sealed class NativeTranscriptMapper
{
    private readonly ConcurrentDictionary<string, UtteranceState> _sources = new(StringComparer.Ordinal);
    private long _sequence;

    public TranscriptUpdate Map(NativeSpeechResult result)
    {
        UtteranceState state = _sources.GetOrAdd(result.Source, _ => new UtteranceState());
        lock (state)
        {
            if (state.UtteranceId is null)
            {
                state.StartMs = Math.Max(0, result.OffsetMs);
                state.UtteranceId = $"native-{result.Source}-{Interlocked.Increment(ref _sequence)}";
            }
            long startMs = result.IsFinal ? Math.Max(0, result.OffsetMs) : state.StartMs;
            long endMs = Math.Max(startMs, result.OffsetMs + result.DurationMs);
            var update = new TranscriptUpdate
            {
                Source = result.Source,
                UtteranceId = state.UtteranceId,
                StartMs = (ulong)startMs,
                EndMs = (ulong)endMs,
                StableText = result.IsFinal ? result.Text : "",
                UnstableText = result.IsFinal ? "" : result.Text,
                IsFinal = result.IsFinal,
                Language = "en-US",
            };
            if (result.IsFinal) state.UtteranceId = null;
            return update;
        }
    }

    private sealed class UtteranceState
    {
        public string? UtteranceId;
        public long StartMs;
    }
}
