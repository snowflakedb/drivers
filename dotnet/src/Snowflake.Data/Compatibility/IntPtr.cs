namespace Snowflake.Data.Compatibility;

internal static class PtrHelper
{
    public static readonly nint Zero =
#if NETFRAMEWORK
        IntPtr.Zero;
#else
        nint.Zero;
#endif
}
