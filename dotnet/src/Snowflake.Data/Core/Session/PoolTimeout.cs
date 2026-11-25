namespace Snowflake.Data.Core.Session;

/// <summary>Converts pool timeouts to the whole seconds the legacy pool API returns.</summary>
internal static class PoolTimeout
{
    /// <summary>
    /// Converts <paramref name="timeout"/> to whole seconds, truncating toward zero.
    /// </summary>
    /// <param name="timeout">The timeout to convert.</param>
    /// <param name="mapInfiniteToMinusOne">
    /// <see langword="true"/> for expiration and connection timeout, which report
    /// <see cref="Timeout.InfiniteTimeSpan"/> as -1. The idle-session wait passes
    /// <see langword="false"/> and truncates that value with the other timeouts.
    /// </param>
    /// <returns>The whole seconds, or -1 when <paramref name="mapInfiniteToMinusOne"/> applies.</returns>
    internal static long ToWholeSeconds(TimeSpan timeout, bool mapInfiniteToMinusOne)
    {
        if (mapInfiniteToMinusOne && timeout == Timeout.InfiniteTimeSpan)
            return -1;

        return (long)timeout.TotalSeconds;
    }
}
