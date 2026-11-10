using Snowflake.Data.Tests.Interop.Compatibility;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// P/Invoke declarations for test-control exports exposed only by
/// <c>libsf_core_stub</c> and not present in the real <c>sf_core</c> library.
/// </summary>
internal static class StubNativeMethods
{
    private const string LibName = StubLibraryNames.Stub;

    /// <summary>
    /// Returns the current live allocation count.
    /// A value of 0 means every buffer allocated by <c>sf_core_api_call_proto</c>
    /// was freed by <c>sf_core_free_buffer</c> exactly once.
    /// Positive = leak; negative = double-free.
    /// </summary>
    [DllImport(LibName, EntryPoint = "sf_stub_leaked_alloc_count", CallingConvention = CallingConvention.Cdecl)]
    public static extern long sf_stub_leaked_alloc_count();

    /// <summary>
    /// Invokes the log callback registered by <c>sf_core_init_dotnet</c> on a worker thread.
    /// Returns 0 on success, 1 when no callback is registered, and 2 if the worker panics.
    /// </summary>
    [DllImport(StubLibraryNames.LogCallbackHolder, EntryPoint = "sf_stub_emit_log", CallingConvention = CallingConvention.Cdecl)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.SafeDirectories | DllImportSearchPath.AssemblyDirectory)]
    public static extern uint sf_stub_emit_log(
        uint level,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string message,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string file,
        uint line,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string function,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string loggerName);
}
