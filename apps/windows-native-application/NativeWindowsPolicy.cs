namespace RimV.Windows.Application;

/// <summary>Presentation rules derived from shared-core status and capabilities.</summary>
public static class NativeWindowsPolicy
{
    public static bool IsTrayToggleEvent(uint action, bool version4) => version4
        ? action is 0x0400 or 0x0401 or 0x007B // NIN_SELECT, NIN_KEYSELECT, WM_CONTEXTMENU
        : action is 0x0202 or 0x0205; // Legacy left/right button release

    public static string ResolveSource(string preferred, bool microphoneAvailable, bool systemAvailable) =>
        preferred switch
        {
            "microphone" when microphoneAvailable => "microphone",
            "system" when systemAvailable => "system",
            "both" when microphoneAvailable && systemAvailable => "both",
            _ when microphoneAvailable => "microphone",
            _ => "system",
        };

    public static string? ListeningCommand(string status) => status switch
    {
        "starting" or "recording" => "stop_capture",
        "stopping" => null,
        _ => "start_capture",
    };

    public static string CaptureActionLabel(string status, bool preparingSpeechDetector, bool visibleError) =>
        preparingSpeechDetector ? "Preparing speech detector…" : status switch
        {
            "starting" => "Starting…",
            "stopping" => "Stopping…",
            "recording" => "Stop Listening",
            "error" when visibleError => "Retry Listening",
            _ => "Start Listening",
        };

    public static bool ShouldAcceptSnapshot(ulong currentRevision, ulong incomingRevision) =>
        incomingRevision >= currentRevision;

    public static bool ShouldCancelViewerClose(bool applicationIsShuttingDown) =>
        !applicationIsShuttingDown;

    public static string SpeechModelStatusMessage(string transcriptionStatus, string? modelState) =>
        transcriptionStatus switch
        {
            "loading" => "Loading the speech model…",
            "error" => "The speech model couldn't be initialized. Check Model Manager, then try again.",
            "ready" or "transcribing" when modelState == "installed" => "Ready",
            _ when modelState == "installed" => "Model installed · loads when you start listening",
            _ => "This model needs to be downloaded.",
        };

    public static bool CanInstallModel(string state) => state is "available" or "incomplete";

    public static bool CanCancelModelInstall(string state) => state == "downloading";

    public static bool CanSelectModel(string modelState, string engineStatus) =>
        modelState == "installed" && engineStatus == "idle";

    public static bool CanRemoveModel(ModelRecord model, string engineStatus) =>
        model.State == "installed" && !model.Selected && engineStatus == "idle";

    public static double DownloadProgressPercent(ModelProgress progress) =>
        progress.TotalBytes is > 0
            ? Math.Clamp(100d * progress.DownloadedBytes / progress.TotalBytes.Value, 0, 100)
            : 0;

    public static bool CanDeleteRecording(string state) => state == "completed";
}
