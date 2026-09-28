using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class NativeWindowsPolicyTests
{
    [Theory]
    [InlineData(0x0400u, true, true)]
    [InlineData(0x0401u, true, true)]
    [InlineData(0x007Bu, true, true)]
    [InlineData(0x0202u, true, false)]
    [InlineData(0x0205u, true, false)]
    [InlineData(0x0202u, false, true)]
    [InlineData(0x0205u, false, true)]
    public void TraySelectEventsToggleOnceForTheNegotiatedShellVersion(uint action, bool version4, bool expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.IsTrayToggleEvent(action, version4));

    [Theory]
    [InlineData("microphone", true, true, "microphone")]
    [InlineData("system", true, true, "system")]
    [InlineData("both", true, true, "both")]
    [InlineData("system", true, false, "microphone")]
    [InlineData("microphone", false, true, "system")]
    [InlineData("both", false, true, "system")]
    public void ResolveSourceUsesAvailableCaptureCapabilities(
        string preferred,
        bool microphone,
        bool system,
        string expected)
    {
        Assert.Equal(expected, NativeWindowsPolicy.ResolveSource(preferred, microphone, system));
    }

    [Theory]
    [InlineData("idle", "start_capture")]
    [InlineData("starting", "stop_capture")]
    [InlineData("recording", "stop_capture")]
    [InlineData("stopping", null)]
    [InlineData("error", "start_capture")]
    public void ListeningCommandDoesNotRestartWhileStopping(string status, string? expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.ListeningCommand(status));

    [Theory]
    [InlineData("available", true, false)]
    [InlineData("incomplete", true, false)]
    [InlineData("downloading", false, false)]
    [InlineData("installed", false, true)]
    [InlineData("unsupported", false, false)]
    public void ModelActionsMatchCoreReportedState(
        string state,
        bool canInstall,
        bool canSelect)
    {
        Assert.Equal(canInstall, NativeWindowsPolicy.CanInstallModel(state));
        Assert.Equal(state == "downloading", NativeWindowsPolicy.CanCancelModelInstall(state));
        Assert.Equal(canSelect, NativeWindowsPolicy.CanSelectModel(state, "idle"));
    }

    [Theory]
    [InlineData("completed", true)]
    [InlineData("recording", false)]
    [InlineData("starting", false)]
    public void OnlyCompletedRecordingsCanBeDeleted(string state, bool expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.CanDeleteRecording(state));
}
