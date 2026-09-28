using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class RecordingViewerLifecycleTests
{
    [Fact]
    public void ClosingAndReopeningInvalidatesOldLoadsAndResetsPlayback()
    {
        var lifecycle = new RecordingViewerLifecycle();
        long firstOpen = lifecycle.Open("completed-session");
        lifecycle.SetPlayback(isPlaying: true, position: TimeSpan.FromSeconds(25));

        lifecycle.Close();
        long secondOpen = lifecycle.Open("completed-session");

        Assert.False(lifecycle.IsCurrent("completed-session", firstOpen));
        Assert.True(lifecycle.IsCurrent("completed-session", secondOpen));
        Assert.False(lifecycle.IsPlaying);
        Assert.Equal(TimeSpan.Zero, lifecycle.PlaybackPosition);
    }

    [Fact]
    public void SwitchingRecordingsRejectsCompletionFromThePreviousLoad()
    {
        var lifecycle = new RecordingViewerLifecycle();
        long staleLoad = lifecycle.Open("session-a");
        long currentLoad = lifecycle.Open("session-b");

        Assert.False(lifecycle.IsCurrent("session-a", staleLoad));
        Assert.True(lifecycle.IsCurrent("session-b", currentLoad));
    }
}
