using System.Runtime.InteropServices;
using System.Text.Json;

namespace RimV.Windows.IntegrationTests;

public sealed class SharedCoreInteropTests
{
    [Fact]
    public void NativeLibrarySupportsVersionedLifecycleAndStructuredErrors()
    {
        string? libraryPath = Environment.GetEnvironmentVariable("RIMV_CORE_FFI_DLL");
        Assert.False(string.IsNullOrWhiteSpace(libraryPath), "Set RIMV_CORE_FFI_DLL to a built rimv_core_ffi.dll.");
        nint nativeLibrary = NativeLibrary.Load(libraryPath!);
        NativeLibrary.SetDllImportResolver(typeof(NativeApi).Assembly, (name, _, _) =>
            name == "rimv_core_ffi" ? nativeLibrary : nint.Zero);

        using var temporary = new TemporaryDirectory();
        nint engine = nint.Zero;
        try
        {
            Assert.Equal(1u, NativeApi.ApiVersion());
            string configuration = JsonSerializer.Serialize(new
            {
                recordings_directory = Path.Combine(temporary.Path, "recordings"),
                models_directory = Path.Combine(temporary.Path, "models"),
            });
            nint configurationPointer = Marshal.StringToCoTaskMemUTF8(configuration);
            string createEnvelope;
            try { createEnvelope = ReadOwned(NativeApi.EngineCreate(configurationPointer, out engine)); }
            finally { Marshal.FreeCoTaskMem(configurationPointer); }
            using (JsonDocument created = JsonDocument.Parse(createEnvelope))
                Assert.True(created.RootElement.GetProperty("ok").GetBoolean());
            Assert.NotEqual(nint.Zero, engine);

            using (JsonDocument state = JsonDocument.Parse(Request(engine, "{\"type\":\"get_state\"}")))
            {
                Assert.True(state.RootElement.GetProperty("ok").GetBoolean());
                Assert.Equal("idle", state.RootElement.GetProperty("result").GetProperty("status").GetString());
            }

            using (JsonDocument error = JsonDocument.Parse(Request(engine, "{\"type\":\"unknown_operation\"}")))
            {
                Assert.False(error.RootElement.GetProperty("ok").GetBoolean());
                Assert.False(string.IsNullOrWhiteSpace(error.RootElement.GetProperty("error").GetProperty("message").GetString()));
            }

            using (JsonDocument malformed = JsonDocument.Parse(Request(engine, "not-json")))
                Assert.False(malformed.RootElement.GetProperty("ok").GetBoolean());

            using (JsonDocument shutdown = JsonDocument.Parse(Request(engine, "{\"type\":\"shutdown\"}")))
                Assert.True(shutdown.RootElement.GetProperty("ok").GetBoolean());
        }
        finally
        {
            if (engine != nint.Zero) NativeApi.EngineDestroy(engine);
            NativeLibrary.Free(nativeLibrary);
        }
    }

    private static string Request(nint engine, string request)
    {
        nint input = Marshal.StringToCoTaskMemUTF8(request);
        try { return ReadOwned(NativeApi.EngineRequest(engine, input)); }
        finally { Marshal.FreeCoTaskMem(input); }
    }

    private static string ReadOwned(nint value)
    {
        Assert.NotEqual(nint.Zero, value);
        try { return Marshal.PtrToStringUTF8(value) ?? throw new InvalidOperationException("Null native response."); }
        finally { NativeApi.StringFree(value); }
    }

    private sealed class TemporaryDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"rimv-ffi-tests-{Guid.NewGuid():N}");
        public TemporaryDirectory() => Directory.CreateDirectory(Path);
        public void Dispose() => Directory.Delete(Path, recursive: true);
    }

    private static class NativeApi
    {
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_api_version", CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint ApiVersion();
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint EngineCreate(nint configuration, out nint engine);
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_request", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint EngineRequest(nint engine, nint request);
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_string_free", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void StringFree(nint value);
        [DllImport("rimv_core_ffi", EntryPoint = "rimv_engine_destroy", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void EngineDestroy(nint engine);
    }
}
