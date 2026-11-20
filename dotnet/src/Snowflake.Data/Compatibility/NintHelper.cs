namespace Snowflake.Data.Compatibility;

#if NETFRAMEWORK
internal static class NintHelper
{
    extension(nint)
    {
        internal static nint Zero => IntPtr.Zero;
        internal static nint Size => IntPtr.Size;
    }
}
#endif
