#if NET7_0_OR_GREATER
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop;

/// <summary>
/// Modern native interop using LibraryImport (source-generated) and function pointers.
/// Requires .NET 7+ for LibraryImport and UnmanagedCallersOnly with function pointer syntax.
/// </summary>
/// ///  TODO this is PoC, will be subject to refactoring in the future
internal sealed unsafe class NativeInterop : INativeInterop
{
    public static readonly NativeInterop Instance = new();

    private NativeInterop() { }

    public void Initialize()
    {
        NativeLibraryLoader.Register();
        var result = NativeMethods.sf_core_init(&LogCallback);

        if (result.Status != 0)
            throw new InvalidOperationException($"sf_core_init failed with code {result.Status}. Check stderr for details.");
    }

    public nuint CallProto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen) => NativeMethods.sf_core_api_call_proto(api, method, request, requestLen, response, responseLen);

    public void FreeBuffer(byte* buffer, nuint len) => NativeMethods.sf_core_free_buffer(buffer, len);

    public string PtrToStringUtf8(byte* ptr) => Marshal.PtrToStringUTF8((nint)ptr) ?? string.Empty;

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static uint LogCallback(uint level, byte* message, byte* filename, uint line, byte* function) =>
        LogCallbackProvider.LogCallback(level, message, filename, line, function);
}

#endif
