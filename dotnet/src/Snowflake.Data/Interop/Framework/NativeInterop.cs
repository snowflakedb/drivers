#if NETFRAMEWORK
using System.Runtime.InteropServices;
using System.Text;

namespace Snowflake.Data.Interop.Framework;

/// <summary>
/// Legacy native interop using DllImport and Marshal.GetFunctionPointerForDelegate.
/// Used on .NET Framework where LibraryImport and function pointers are unavailable.
/// On .NET Framework, sf_core.dll must be in the application output directory (copied by build).
/// </summary>
///  TODO this is PoC, will be subject to refactoring in the future
internal sealed unsafe class NativeInterop : INativeInterop
{
    public static readonly NativeInterop Instance = new();

    private NativeInterop() { }

    public void Initialize()
    {
        var logCallbackDelegate = (LogCallbackDelegate)LogCallbackProvider.LogCallback;
        GC.KeepAlive(logCallbackDelegate);
        var callbackPtr = Marshal.GetFunctionPointerForDelegate(logCallbackDelegate);
        var result = NativeMethods.sf_core_init(callbackPtr);

        if (result != 0)
            throw new InvalidOperationException($"sf_core_init failed with code {result}. Check stderr for details.");
    }

    public nuint CallProto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen) =>
        NativeMethods.sf_core_api_call_proto(api, method, request, requestLen, response, responseLen);

    public void FreeBuffer(byte* buffer, nuint len) => NativeMethods.sf_core_free_buffer(buffer, len);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint LogCallbackDelegate(uint level, byte* message, byte* filename, uint line, byte* function);

    public string PtrToStringUtf8(byte* ptr)
    {
        if (ptr == null)
            return string.Empty;

        var len = 0;
        while (ptr[len] != 0)
            len++;

        return len == 0
            ? string.Empty
            : Encoding.UTF8.GetString(ptr, len);
    }
}

#endif
