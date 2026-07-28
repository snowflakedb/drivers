#if NETFRAMEWORK
using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop.Framework;
///  TODO this is PoC, will be subject to refactoring in the future
internal static unsafe class NativeMethods
{
    private const string LibName = "sf_core";

    [DllImport(LibName, EntryPoint = "sf_core_init", CallingConvention = CallingConvention.Cdecl)]
    public static extern SfCoreInitResult sf_core_init(IntPtr callback);

    [DllImport(LibName, EntryPoint = "sf_core_api_call_proto", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    public static extern UIntPtr sf_core_api_call_proto(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string api,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        byte* request,
        UIntPtr requestLen,
        byte** response,
        nuint* responseLen);

    [DllImport(LibName, EntryPoint = "sf_core_free_buffer", CallingConvention = CallingConvention.Cdecl)]
    public static extern void sf_core_free_buffer(byte* buffer, UIntPtr len);
}
#endif
