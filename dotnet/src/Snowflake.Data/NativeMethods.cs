// TODO intended for future use
global using LockObject =
#if NET10_0_OR_GREATER
System.Threading.Lock;
#else
    object;
#endif

using System.Runtime.InteropServices;

namespace Snowflake.Data;

/// <summary>
/// Raw P/Invoke declarations for the sf_core native library.
/// </summary>
internal static unsafe partial class NativeMethods
{
    private const string LibName = "sf_core";

    [LibraryImport(LibName, EntryPoint = "sf_core_init")]
    public static partial uint sf_core_init(delegate* unmanaged[Cdecl]<uint, byte*, byte*, uint, byte*, uint> callback);

    [LibraryImport(LibName, EntryPoint = "sf_core_api_call_proto", StringMarshalling = StringMarshalling.Utf8)]
    public static partial nuint sf_core_api_call_proto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen);

    [LibraryImport(LibName, EntryPoint = "sf_core_free_buffer")]
    public static partial void sf_core_free_buffer(byte* buffer, nuint len);
}
