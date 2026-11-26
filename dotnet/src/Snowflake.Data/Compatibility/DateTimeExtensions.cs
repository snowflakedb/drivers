namespace Snowflake.Data.Compatibility;

#if NETFRAMEWORK
internal static class DateTimeExtensions
{
    extension(DateTime)
    {
        internal static DateTime UnixEpoch => new(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);
    }
}
#endif
