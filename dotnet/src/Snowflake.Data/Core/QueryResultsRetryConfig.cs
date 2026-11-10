namespace Snowflake.Data.Core;

/// <summary>
/// Polling schedule used while waiting for an asynchronously executed query to finish before its
/// results are fetched.
/// </summary>
internal sealed class QueryResultsRetryConfig
{
    internal QueryResultsRetryConfig(int noDataMaxRetries, IEnumerable<TimeSpan> retryPattern)
    {
        if (!retryPattern.Any())
            throw new ArgumentException("The retry pattern must contain at least one delay.", nameof(retryPattern));

        NoDataMaxRetries = noDataMaxRetries;
        RetryPattern = retryPattern;
    }

    internal static QueryResultsRetryConfig GetDefault => new(24, GetDefaultRetryPattern());

    /// <summary>How many NO_DATA answers can be observed before giving up.</summary>
    internal int NoDataMaxRetries { get; }

    /// <summary>Delays between status checks. May be unbounded.</summary>
    internal IEnumerable<TimeSpan> RetryPattern { get; }

    private static IEnumerable<TimeSpan> GetDefaultRetryPattern()
    {
        yield return TimeSpan.FromSeconds(1);
        yield return TimeSpan.FromSeconds(1);
        yield return TimeSpan.FromSeconds(2);
        yield return TimeSpan.FromSeconds(3);
        yield return TimeSpan.FromSeconds(4);
        yield return TimeSpan.FromSeconds(8);

        for (; ; )
            yield return TimeSpan.FromSeconds(10);
    }
}
