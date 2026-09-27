using Microsoft.Win32.SafeHandles;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RimV.Windows;

internal sealed class CoreClient : IDisposable
{
    private readonly EngineHandle _handle;
    private readonly SemaphoreSlim _commands = new(1, 1);
    private bool _disposed;

    private CoreClient(EngineHandle handle) => _handle = handle;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };

    public static CoreClient Create(string recordingsDirectory, string modelsDirectory)
    {
        uint version = Native.ApiVersion();
        if (version != 1) throw new InvalidOperationException($"RimV core API version {version} is not supported.");

        string configuration = JsonSerializer.Serialize(new
        {
            recordings_directory = recordingsDirectory,
            models_directory = modelsDirectory,
        });
        nint input = Marshal.StringToCoTaskMemUTF8(configuration);
        try
        {
            nint response = Native.EngineCreate(input, out nint handle);
            try
            {
                ThrowIfError(ReadOwnedString(response));
            }
            catch
            {
                if (handle != 0) Native.EngineDestroy(handle);
                throw;
            }
            if (handle == 0) throw new InvalidOperationException("The shared RimV engine did not return a handle.");
            return new CoreClient(new EngineHandle(handle));
        }
        finally
        {
            Marshal.FreeCoTaskMem(input);
        }
    }

    public Task<T> RequestAsync<T>(object request, CancellationToken cancellationToken = default)
    {
        string json = JsonSerializer.Serialize(request, JsonOptions);
        return Task.Run(() =>
        {
            _commands.Wait(cancellationToken);
            try
            {
                ObjectDisposedException.ThrowIf(_disposed, this);
                return Invoke<T>(json);
            }
            finally { _commands.Release(); }
        }, cancellationToken);
    }

    public Task<CoreEvent?> PollEventAsync(CancellationToken cancellationToken) => Task.Run(() =>
    {
        string json = JsonSerializer.Serialize(new { type = "poll_event", timeout_ms = 300 }, JsonOptions);
        JsonElement result = Invoke<JsonElement>(json);
        if (result.ValueKind is JsonValueKind.Null or JsonValueKind.Undefined) return null;
        string type = result.TryGetProperty("type", out JsonElement typeValue)
            ? typeValue.GetString() ?? ""
            : "";
        return new CoreEvent(type, result);
    }, cancellationToken);

    private T Invoke<T>(string requestJson)
    {
        nint input = Marshal.StringToCoTaskMemUTF8(requestJson);
        try
        {
            nint response = Native.EngineRequest(_handle.DangerousGetHandle(), input);
            string payload = ReadOwnedString(response);
            using JsonDocument document = JsonDocument.Parse(payload);
            JsonElement root = document.RootElement;
            if (!root.GetProperty("ok").GetBoolean())
            {
                string message = root.GetProperty("error").GetProperty("message").GetString() ?? "RimV request failed.";
                throw new CoreRequestException(message);
            }
            JsonElement result = root.GetProperty("result");
            if (typeof(T) == typeof(JsonElement))
                return (T)(object)result.Clone();
            if (typeof(T) == typeof(string) && result.ValueKind == JsonValueKind.String)
                return (T)(object)(result.GetString() ?? "");
            return result.Deserialize<T>(JsonOptions)
                ?? throw new CoreRequestException("RimV returned an empty response.");
        }
        finally
        {
            Marshal.FreeCoTaskMem(input);
        }
    }

    private static string ReadOwnedString(nint pointer)
    {
        if (pointer == 0) throw new CoreRequestException("RimV returned an invalid response.");
        try { return Marshal.PtrToStringUTF8(pointer) ?? ""; }
        finally { Native.StringFree(pointer); }
    }

    private static void ThrowIfError(string envelope)
    {
        using JsonDocument document = JsonDocument.Parse(envelope);
        if (!document.RootElement.GetProperty("ok").GetBoolean())
            throw new CoreRequestException(document.RootElement.GetProperty("error").GetProperty("message").GetString() ?? "RimV initialization failed.");
    }

    public void Shutdown()
    {
        _commands.Wait();
        try
        {
            if (_disposed) return;
            _disposed = true;
            try { Invoke<JsonElement>("{\"type\":\"shutdown\"}"); }
            finally { _handle.Dispose(); }
        }
        finally { _commands.Release(); }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _handle.Dispose();
    }

    private sealed class EngineHandle : SafeHandleZeroOrMinusOneIsInvalid
    {
        public EngineHandle(nint value) : base(ownsHandle: true) => SetHandle(value);
        public override bool IsInvalid => handle == 0 || base.IsInvalid;
        protected override bool ReleaseHandle()
        {
            Native.EngineDestroy(handle);
            return true;
        }
    }

    private static class Native
    {
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_api_version", CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint ApiVersion();

        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint EngineCreate(nint configuration, out nint handle);

        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_request", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint EngineRequest(nint handle, nint request);

        [DllImport("rimv_core_ffi", EntryPoint = "rimv_string_free", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void StringFree(nint value);

        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_destroy", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void EngineDestroy(nint handle);
    }
}

internal sealed record CoreEvent(string Type, JsonElement Payload);
internal sealed class CoreRequestException(string message) : Exception(message);

internal sealed class CoreSnapshot
{
    [JsonPropertyName("status")] public string Status { get; init; } = "idle";
    [JsonPropertyName("session")] public SessionInfo? Session { get; init; }
    [JsonPropertyName("elapsed_ms")] public ulong ElapsedMs { get; init; }
    [JsonPropertyName("microphone")] public SourceState Microphone { get; init; } = new();
    [JsonPropertyName("system_audio")] public SourceState SystemAudio { get; init; } = new();
    [JsonPropertyName("transcription")] public TranscriptionState Transcription { get; init; } = new();
    [JsonPropertyName("capabilities")] public EngineCapabilities Capabilities { get; init; } = new();
}

internal sealed class SessionInfo
{
    [JsonPropertyName("id")] public string Id { get; init; } = "";
    [JsonPropertyName("started_at_unix_ms")] public ulong StartedAtUnixMs { get; init; }
    [JsonPropertyName("recording_directory")] public string RecordingDirectory { get; init; } = "";
}

internal sealed class SourceState
{
    [JsonPropertyName("enabled")] public bool Enabled { get; init; }
    [JsonPropertyName("active")] public bool Active { get; init; }
}

internal sealed class TranscriptionState
{
    [JsonPropertyName("enabled")] public bool Enabled { get; init; }
    [JsonPropertyName("available")] public bool Available { get; init; }
    [JsonPropertyName("status")] public string Status { get; init; } = "disabled";
    [JsonPropertyName("backend")] public string? Backend { get; init; }
    [JsonPropertyName("model_path")] public string? ModelPath { get; init; }
    [JsonPropertyName("language")] public string? Language { get; init; }
}

internal sealed class EngineCapabilities
{
    [JsonPropertyName("microphone_capture")] public bool MicrophoneCapture { get; init; }
    [JsonPropertyName("system_audio_capture")] public bool SystemAudioCapture { get; init; }
    [JsonPropertyName("transcription")] public bool Transcription { get; init; }
}

internal sealed class ModelRecord
{
    [JsonPropertyName("descriptor")] public ModelDescriptor Descriptor { get; init; } = new();
    [JsonPropertyName("state")] public string State { get; set; } = "available";
}

internal sealed class ModelDescriptor
{
    [JsonPropertyName("id")] public string Id { get; init; } = "";
    [JsonPropertyName("backend")] public string Backend { get; init; } = "";
    [JsonPropertyName("display_name")] public string DisplayName { get; init; } = "";
    [JsonPropertyName("version")] public string Version { get; init; } = "";
    [JsonPropertyName("storage_directory")] public string StorageDirectory { get; init; } = "";
    [JsonPropertyName("files")] public List<ModelFile> Files { get; init; } = [];
    [JsonPropertyName("languages")] public List<string> Languages { get; init; } = [];
}

internal sealed class ModelFile
{
    [JsonPropertyName("filename")] public string Filename { get; init; } = "";
}

internal sealed class RecordingSummary
{
    [JsonPropertyName("session_id")] public string SessionId { get; init; } = "";
    [JsonPropertyName("title")] public string Title { get; init; } = "Recording";
    [JsonPropertyName("started_at_unix_ms")] public ulong StartedAtUnixMs { get; init; }
    [JsonPropertyName("duration_ms")] public ulong DurationMs { get; init; }
    [JsonPropertyName("state")] public string State { get; init; } = "completed";
    [JsonPropertyName("sources")] public List<string> Sources { get; init; } = [];
    [JsonPropertyName("has_transcript")] public bool HasTranscript { get; init; }
}

internal sealed class RecordingDetails
{
    [JsonPropertyName("summary")] public RecordingSummary Summary { get; init; } = new();
    [JsonPropertyName("transcript")] public List<TranscriptLine> Transcript { get; init; } = [];
}

internal sealed class TranscriptLine
{
    [JsonPropertyName("source")] public string Source { get; init; } = "";
    [JsonPropertyName("start_ms")] public ulong StartMs { get; init; }
    [JsonPropertyName("end_ms")] public ulong EndMs { get; init; }
    [JsonPropertyName("text")] public string Text { get; init; } = "";
}

internal sealed class TranscriptUpdate
{
    [JsonPropertyName("source")] public string Source { get; init; } = "";
    [JsonPropertyName("utterance_id")] public string UtteranceId { get; init; } = "";
    [JsonPropertyName("start_ms")] public ulong StartMs { get; init; }
    [JsonPropertyName("end_ms")] public ulong EndMs { get; init; }
    [JsonPropertyName("stable_text")] public string StableText { get; init; } = "";
    [JsonPropertyName("unstable_text")] public string UnstableText { get; init; } = "";
    [JsonPropertyName("is_final")] public bool IsFinal { get; init; }
}

internal sealed class ModelProgress
{
    [JsonPropertyName("model_id")] public string ModelId { get; init; } = "";
    [JsonPropertyName("phase")] public string Phase { get; init; } = "";
    [JsonPropertyName("downloaded_bytes")] public ulong DownloadedBytes { get; init; }
    [JsonPropertyName("total_bytes")] public ulong? TotalBytes { get; init; }
    [JsonPropertyName("error")] public string? Error { get; init; }
}

internal sealed class ExportPayload
{
    [JsonPropertyName("content")] public string Content { get; init; } = "";
}
