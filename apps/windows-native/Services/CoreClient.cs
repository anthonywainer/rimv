using Microsoft.Win32.SafeHandles;
using RimV.Windows.Application;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace RimV.Windows;

internal sealed class CoreClient : ISharedCoreClient
{
    private readonly EngineHandle _handle;
    private readonly IAppLog _log;
    private readonly SemaphoreSlim _commands = new(1, 1);
    private bool _disposed;

    private CoreClient(EngineHandle handle, IAppLog log) { _handle = handle; _log = log; }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };

    public static CoreClient Create(string recordingsDirectory, string modelsDirectory, IAppLog log)
    {
        uint version = CoreNativeMethods.ApiVersion();
        if (version != 1) throw new InvalidOperationException($"RimV core API version {version} is not supported.");

        string configuration = JsonSerializer.Serialize(new
        {
            recordings_directory = recordingsDirectory,
            models_directory = modelsDirectory,
            diagnostics_log_path = Path.Combine(Path.GetDirectoryName(recordingsDirectory)!, "logs", "rust-engine.log"),
        });
        nint input = Marshal.StringToCoTaskMemUTF8(configuration);
        try
        {
            nint response = CoreNativeMethods.EngineCreate(input, out nint handle);
            try
            {
                ThrowIfError(ReadOwnedString(response));
            }
            catch
            {
                if (handle != 0) CoreNativeMethods.EngineDestroy(handle);
                throw;
            }
            if (handle == 0) throw new InvalidOperationException("The shared RimV engine did not return a handle.");
            log.Info("rust_engine.created", $"api_version={version}");
            return new CoreClient(new EngineHandle(handle), log);
        }
        finally
        {
            Marshal.FreeCoTaskMem(input);
        }
    }

    public Task<T> RequestAsync<T>(object request, CancellationToken cancellationToken = default)
    {
        string json = JsonSerializer.Serialize(request, JsonOptions);
        string operation = GetOperationName(json);
        return Task.Run(() =>
        {
            _commands.Wait(cancellationToken);
            try
            {
                ObjectDisposedException.ThrowIf(_disposed, this);
                try
                {
                    T result = Invoke<T>(json);
                    if (operation != "poll_event") _log.Info("rust.request", operation);
                    return result;
                }
                catch (Exception error)
                {
                    _log.Error("rust.request_failed", error, operation);
                    throw;
                }
            }
            finally { _commands.Release(); }
        }, cancellationToken);
    }

    public Task<CoreEvent?> PollEventAsync(CancellationToken cancellationToken) => Task.Run<CoreEvent?>(() =>
    {
        string json = JsonSerializer.Serialize(new { type = "poll_event", timeout_ms = 300 }, JsonOptions);
        try
        {
            JsonElement result = Invoke<JsonElement>(json);
            if (result.ValueKind is JsonValueKind.Null or JsonValueKind.Undefined) return null;
            string type = result.TryGetProperty("type", out JsonElement typeValue)
                ? typeValue.GetString() ?? ""
                : "";
            if (type.Length > 0) _log.Info("rust.event", type);
            return new CoreEvent(type, result);
        }
        catch (Exception error)
        {
            _log.Error("rust.event_poll_failed", error);
            throw;
        }
    }, cancellationToken);

    private T Invoke<T>(string requestJson)
    {
        nint input = Marshal.StringToCoTaskMemUTF8(requestJson);
        try
        {
            nint response = CoreNativeMethods.EngineRequest(_handle.DangerousGetHandle(), input);
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
        finally { CoreNativeMethods.StringFree(pointer); }
    }

    private static void ThrowIfError(string envelope)
    {
        using JsonDocument document = JsonDocument.Parse(envelope);
        if (!document.RootElement.GetProperty("ok").GetBoolean())
            throw new CoreRequestException(document.RootElement.GetProperty("error").GetProperty("message").GetString() ?? "RimV initialization failed.");
    }

    private static string GetOperationName(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        JsonElement root = document.RootElement;
        if (root.TryGetProperty("type", out JsonElement type))
        {
            string operation = type.GetString() ?? "unknown";
            if (operation == "send" && root.TryGetProperty("command", out JsonElement command)
                && command.TryGetProperty("type", out JsonElement commandType))
                return $"send.{commandType.GetString() ?? "unknown"}";
            return operation;
        }
        return "unknown";
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
            CoreNativeMethods.EngineDestroy(handle);
            return true;
        }
    }

}
