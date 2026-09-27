using System.Runtime.InteropServices;

namespace RimV.Windows;

/// <summary>Raw imports for the stable rimv-ffi C ABI. Ownership is handled by CoreClient.</summary>
internal static class CoreNativeMethods
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
