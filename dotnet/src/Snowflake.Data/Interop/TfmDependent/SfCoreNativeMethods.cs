#if !NETFRAMEWORK
using System.Buffers;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Snowflake.Data.Interop.Callback;

namespace Snowflake.Data.Interop.TfmDependent;

internal unsafe partial class SfCoreNativeMethods : ISfCoreInterop
{
    public static ISfCoreInterop Instance { get; } = new SfCoreNativeMethods();

    private const string LibName = "sf_core";

    [LibraryImport(LibName, EntryPoint = "sf_core_init")]
    private static partial SfCoreInitResult sf_core_init(delegate* unmanaged[Cdecl]<uint, byte*, byte*, uint, byte*, byte*, uint> callback);

    [LibraryImport(LibName, EntryPoint = "sf_core_api_call_proto", StringMarshalling = StringMarshalling.Utf8)]
    private static partial nuint sf_core_api_call_proto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen);

    [LibraryImport(LibName, EntryPoint = "sf_core_api_call_proto_async", StringMarshalling = StringMarshalling.Utf8)]
    private static partial ulong sf_core_api_call_proto_async(string api, string method, byte* request, nuint requestLen, delegate* unmanaged[Cdecl]<void*, nuint, nuint, nuint, void> callback, void* userData);

    [LibraryImport(LibName, EntryPoint = "sf_core_free_buffer")]
    private static partial void sf_core_free_buffer(byte* buffer, nuint len);

    [LibraryImport(LibName, EntryPoint = "sf_core_api_cancel")]
    private static partial void sf_core_api_cancel(ulong asyncHandle);

    public void Initialize() => Initialize(SfCoreLibraryLoader.Instance);

    private static void Initialize(ISfCoreLibraryLoader loader)
    {
        loader.Register();
        var result = sf_core_init(&LogCallback);

        if (result.Status != 0)
            throw new InvalidOperationException($"sf_core_init failed with code {result}. Check stderr for details.");
    }

    public nuint CallProto(string api, string method, byte* request, UIntPtr requestLen, byte** response, UIntPtr* responseLen) => sf_core_api_call_proto(api, method, request, requestLen, response, responseLen);

    public ulong CallProtoAsync(string api, string method, byte* request, UIntPtr requestLen, void* userData) => sf_core_api_call_proto_async(api, method, request, requestLen, &ResponseCallback, userData);

    public void CallProtoCancel(ulong asyncHandle) => sf_core_api_cancel(asyncHandle);

    public void CallFreeBuffer(byte* buffer, UIntPtr responseLen) => sf_core_free_buffer(buffer, responseLen);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static void ResponseCallback(void* userDataPointer, nuint status, nuint ptr, nuint len) =>
        ProtoAsyncCallbackProvider.ResponseCallback(userDataPointer, status, ptr, len);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static uint LogCallback(uint level, byte* message, byte* filename, uint line, byte* function, byte* loggerName) =>
        LogCallbackProvider.LogCallback(level, message, filename, line, function, loggerName);
}
#endif
