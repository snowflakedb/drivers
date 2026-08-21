namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// P/Invoke declarations for test-control exports exposed only by
/// <c>libsf_core_stub</c> and not present in the real <c>sf_core</c> library.
/// </summary>
internal static class StubNativeMethods
{
    private const string LibName = "sf_core_stub";

    /// <summary>
    /// Returns the current live allocation count.
    /// A value of 0 means every buffer allocated by <c>sf_core_api_call_proto</c>
    /// was freed by <c>sf_core_free_buffer</c> exactly once.
    /// Positive = leak; negative = double-free.
    /// </summary>
    [DllImport(LibName, EntryPoint = "sf_stub_leaked_alloc_count", CallingConvention = CallingConvention.Cdecl)]
    public static extern long sf_stub_leaked_alloc_count();
}
