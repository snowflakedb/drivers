#if NETFRAMEWORK
using System.Runtime.InteropServices;
using Snowflake.Data.Interop.Callback;

namespace Snowflake.Data.Interop.TfmDependent.Framework;

internal unsafe class SfCoreNativeMethods : ISfCoreInterop
{
    public static ISfCoreInterop Instance { get; } = new SfCoreNativeMethods();

    private const string LibName = "sf_core";

    public void Initialize()
    {
        var result = sf_core_init(LogCallbackPtr);

        if (result.Status != 0)
            throw new InvalidOperationException($"sf_core_init failed with code {result}. Check stderr for details.");
    }

    public nuint CallProto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen) => sf_core_api_call_proto(api, method, request, requestLen, response, responseLen);

    public ulong CallProtoAsync(string api, string method, byte* request, nuint requestLen, void* userData) => sf_core_api_call_proto_async(api, method, request, requestLen, ResponseCallbackPtr, userData);

    public void CallProtoCancel(ulong asyncHandle) => sf_core_api_cancel(asyncHandle);

    public void CallFreeBuffer(byte* buffer, nuint responseLen) => sf_core_free_buffer(buffer, responseLen);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void ResponseCallbackDelegate(void* userDataPointer, nuint status, nuint ptr, nuint len);

    [DllImport(LibName, EntryPoint = "sf_core_init", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern SfCoreInitResult sf_core_init(IntPtr callback);

    [DllImport(LibName, EntryPoint = "sf_core_api_call_proto", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern nuint sf_core_api_call_proto(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string api,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        byte* request,
        nuint requestLen,
        byte** response,
        nuint* responseLen);

    [DllImport(LibName, EntryPoint = "sf_core_api_call_proto_async", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern ulong sf_core_api_call_proto_async(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string api,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        byte* request,
        nuint requestLen,
        IntPtr callback,
        void* userData);

    [DllImport(LibName, EntryPoint = "sf_core_free_buffer", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern void sf_core_free_buffer(byte* buffer, nuint len);

    [DllImport(LibName, EntryPoint = "sf_core_api_cancel", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern void sf_core_api_cancel(ulong asyncHandle);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint LogCallbackDelegate(uint level, byte* message, byte* filename, uint line, byte* function, byte* loggerName);

    /// <summary>
    /// Rooted static delegates — must not be GC'd while native code holds the function pointers. Static fields live for the process lifetime, which is exactly the required scope.
    /// </summary>
    private static readonly LogCallbackDelegate LogCallbackDelegateInstance = LogCallbackProvider.LogCallback;
    private static readonly IntPtr LogCallbackPtr = Marshal.GetFunctionPointerForDelegate(LogCallbackDelegateInstance);

    private static readonly ResponseCallbackDelegate ResponseCallbackDelegateInstance = ProtoAsyncCallbackProvider.ResponseCallback;
    private static readonly IntPtr ResponseCallbackPtr = Marshal.GetFunctionPointerForDelegate(ResponseCallbackDelegateInstance);
}
#endif
