using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class LiveTranscriptBufferTests
{
    [Fact]
    public void RestoreAndSubsequentUpdatesReconcileByStableUtteranceIdentity()
    {
        var buffer = new LiveTranscriptBuffer();
        buffer.Open("session-1");
        var snapshot = new LiveTranscriptSnapshot
        {
            SessionId = "session-1",
            Revision = 2,
            Updates = [Update("utterance-1", "hello", isFinal: false)],
        };

        Assert.True(buffer.Restore(snapshot));
        Assert.True(buffer.Apply("session-1", Update("utterance-1", "hello there", isFinal: true)));

        TranscriptUpdate actual = Assert.Single(buffer.Updates);
        Assert.Equal("hello there", actual.StableText);
        Assert.True(actual.IsFinal);
    }

    [Fact]
    public void OpeningAnotherSessionDropsOldTranscriptAndIgnoresStaleEvents()
    {
        var buffer = new LiveTranscriptBuffer();
        buffer.Open("session-1");
        Assert.True(buffer.Apply("session-1", Update("utterance-1", "first")));

        buffer.Open("session-2");

        Assert.False(buffer.Apply("session-1", Update("utterance-2", "stale")));
        Assert.Empty(buffer.Updates);
    }

    [Fact]
    public void SnapshotForDifferentSessionCannotReplaceCurrentView()
    {
        var buffer = new LiveTranscriptBuffer();
        buffer.Open("session-current");

        Assert.False(buffer.Restore(new LiveTranscriptSnapshot
        {
            SessionId = "session-old",
            Updates = [Update("old", "old transcript")],
        }));
        Assert.Empty(buffer.Updates);
    }

    [Fact]
    public void StaleRevisionsAreIgnoredAndCoalescedCurrentRevisionUpdatesRemainApplicable()
    {
        var buffer = new LiveTranscriptBuffer();
        buffer.Open("session-1");
        Assert.True(buffer.Restore(new LiveTranscriptSnapshot
        {
            SessionId = "session-1",
            Revision = 4,
            Updates = [Update("utterance-1", "latest")],
        }));

        Assert.False(buffer.Apply("session-1", Update("utterance-1", "stale"), revision: 3));
        Assert.True(buffer.Apply("session-1", Update("utterance-2", "same current revision"), revision: 4));

        Assert.Equal(["utterance-1", "utterance-2"], buffer.Updates.Select(item => item.UtteranceId));
    }

    [Fact]
    public void LateOlderSnapshotCannotRollBackAnEventAlreadyAppliedDuringLoading()
    {
        var buffer = new LiveTranscriptBuffer();
        buffer.Open("session-1");
        Assert.True(buffer.Apply("session-1", Update("utterance-1", "latest event"), revision: 7));

        Assert.False(buffer.Restore(new LiveTranscriptSnapshot
        {
            SessionId = "session-1",
            Revision = 6,
            Updates = [Update("utterance-1", "older snapshot")],
        }));

        Assert.Equal("latest event", Assert.Single(buffer.Updates).StableText);
    }

    private static TranscriptUpdate Update(string id, string text, bool isFinal = false) => new()
    {
        Source = "microphone",
        UtteranceId = id,
        StableText = text,
        IsFinal = isFinal,
    };
}
