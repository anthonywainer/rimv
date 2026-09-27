namespace RimV.Windows.Application;

/// <summary>Presentation rules derived from shared-core status and capabilities.</summary>
public static class NativeWindowsPolicy
{
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

    public static bool CanInstallModel(string state) => state is "available" or "incomplete";

    public static bool CanCancelModelInstall(string state) => state == "downloading";

    public static bool CanSelectModel(string modelState, string engineStatus) =>
        modelState == "installed" && engineStatus == "idle";

    public static bool CanDeleteRecording(string state) => state == "completed";
}
