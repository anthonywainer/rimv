using System.Text.Json;
using System.Text.Json.Serialization;

namespace RimV.Windows.Application;

public sealed record CoreEvent(string Type, JsonElement Payload);
public sealed class CoreRequestException(string message) : Exception(message);

public sealed class CoreSnapshot
{
    [JsonPropertyName("status")] public string Status { get; init; } = "idle";
    [JsonPropertyName("session")] public SessionInfo? Session { get; init; }
    [JsonPropertyName("elapsed_ms")] public ulong ElapsedMs { get; init; }
    [JsonPropertyName("microphone")] public SourceState Microphone { get; init; } = new();
    [JsonPropertyName("system_audio")] public SourceState SystemAudio { get; init; } = new();
    [JsonPropertyName("transcription")] public TranscriptionState Transcription { get; init; } = new();
    [JsonPropertyName("capabilities")] public EngineCapabilities Capabilities { get; init; } = new();
}

public sealed class SessionInfo
{
    [JsonPropertyName("id")] public string Id { get; init; } = "";
    [JsonPropertyName("started_at_unix_ms")] public ulong StartedAtUnixMs { get; init; }
    [JsonPropertyName("recording_directory")] public string RecordingDirectory { get; init; } = "";
}

public sealed class SourceState
{
    [JsonPropertyName("enabled")] public bool Enabled { get; init; }
    [JsonPropertyName("active")] public bool Active { get; init; }
}

public sealed class TranscriptionState
{
    [JsonPropertyName("enabled")] public bool Enabled { get; init; }
    [JsonPropertyName("available")] public bool Available { get; init; }
    [JsonPropertyName("status")] public string Status { get; init; } = "disabled";
    [JsonPropertyName("backend")] public string? Backend { get; init; }
    [JsonPropertyName("model_path")] public string? ModelPath { get; init; }
    [JsonPropertyName("language")] public string? Language { get; init; }
}

public sealed class EngineCapabilities
{
    [JsonPropertyName("microphone_capture")] public bool MicrophoneCapture { get; init; }
    [JsonPropertyName("system_audio_capture")] public bool SystemAudioCapture { get; init; }
    [JsonPropertyName("transcription")] public bool Transcription { get; init; }
}

public sealed class ModelRecord
{
    [JsonPropertyName("descriptor")] public ModelDescriptor Descriptor { get; init; } = new();
    [JsonPropertyName("state")] public string State { get; set; } = "available";
}

public sealed class ModelDescriptor
{
    [JsonPropertyName("id")] public string Id { get; init; } = "";
    [JsonPropertyName("backend")] public string Backend { get; init; } = "";
    [JsonPropertyName("display_name")] public string DisplayName { get; init; } = "";
    [JsonPropertyName("version")] public string Version { get; init; } = "";
    [JsonPropertyName("storage_directory")] public string StorageDirectory { get; init; } = "";
    [JsonPropertyName("files")] public List<ModelFile> Files { get; init; } = [];
    [JsonPropertyName("languages")] public List<string> Languages { get; init; } = [];
    [JsonPropertyName("capabilities")] public ModelCapabilities Capabilities { get; init; } = new();
}

public sealed class ModelCapabilities
{
    [JsonPropertyName("supports_language_detection")] public bool SupportsLanguageDetection { get; init; }
}

public sealed class ModelFile
{
    [JsonPropertyName("filename")] public string Filename { get; init; } = "";
}

public sealed class RecordingSummary
{
    [JsonPropertyName("session_id")] public string SessionId { get; init; } = "";
    [JsonPropertyName("title")] public string Title { get; init; } = "Recording";
    [JsonPropertyName("started_at_unix_ms")] public ulong StartedAtUnixMs { get; init; }
    [JsonPropertyName("duration_ms")] public ulong DurationMs { get; init; }
    [JsonPropertyName("state")] public string State { get; init; } = "completed";
    [JsonPropertyName("sources")] public List<string> Sources { get; init; } = [];
    [JsonPropertyName("has_transcript")] public bool HasTranscript { get; init; }
}

public sealed class RecordingDetails
{
    [JsonPropertyName("summary")] public RecordingSummary Summary { get; init; } = new();
    [JsonPropertyName("transcript")] public List<TranscriptLine> Transcript { get; init; } = [];
}

public sealed class TranscriptLine
{
    [JsonPropertyName("source")] public string Source { get; init; } = "";
    [JsonPropertyName("start_ms")] public ulong StartMs { get; init; }
    [JsonPropertyName("end_ms")] public ulong EndMs { get; init; }
    [JsonPropertyName("text")] public string Text { get; init; } = "";
}

public sealed class TranscriptUpdate
{
    [JsonPropertyName("source")] public string Source { get; init; } = "";
    [JsonPropertyName("utterance_id")] public string UtteranceId { get; init; } = "";
    [JsonPropertyName("start_ms")] public ulong StartMs { get; init; }
    [JsonPropertyName("end_ms")] public ulong EndMs { get; init; }
    [JsonPropertyName("stable_text")] public string StableText { get; init; } = "";
    [JsonPropertyName("unstable_text")] public string UnstableText { get; init; } = "";
    [JsonPropertyName("is_final")] public bool IsFinal { get; init; }
}

public sealed class ModelProgress
{
    [JsonPropertyName("model_id")] public string ModelId { get; init; } = "";
    [JsonPropertyName("phase")] public string Phase { get; init; } = "";
    [JsonPropertyName("downloaded_bytes")] public ulong DownloadedBytes { get; init; }
    [JsonPropertyName("total_bytes")] public ulong? TotalBytes { get; init; }
    [JsonPropertyName("error")] public string? Error { get; init; }
}

public sealed class ExportPayload
{
    [JsonPropertyName("content")] public string Content { get; init; } = "";
}
