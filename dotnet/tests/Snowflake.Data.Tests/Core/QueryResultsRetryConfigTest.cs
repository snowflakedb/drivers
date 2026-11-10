using Snowflake.Data.Core;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class QueryResultsRetryConfigTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DefaultScheduleMatchesTheOldDriver()
    {
        var config = QueryResultsRetryConfig.GetDefault;

        Assert.Equal(24, config.NoDataMaxRetries);
        Assert.Equal(
            new[] { 1, 1, 2, 3, 4, 8, 10, 10, 10, 10 }.Select(seconds => TimeSpan.FromSeconds(seconds)),
            config.RetryPattern.Take(10));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DefaultScheduleKeepsRepeatingTheLastDelay()
    {
        var delays = QueryResultsRetryConfig.GetDefault.RetryPattern.Skip(6).Take(1000).ToArray();

        Assert.Equal(1000, delays.Length);
        Assert.All(delays, delay => Assert.Equal(TimeSpan.FromSeconds(10), delay));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyScheduleIsRejected()
    {
        var exception = Assert.Throws<ArgumentException>(() => new QueryResultsRetryConfig(1, []));

        Assert.Equal("retryPattern", exception.ParamName);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CustomScheduleIsKeptAsGiven()
    {
        TimeSpan[] pattern = [TimeSpan.FromMilliseconds(5), TimeSpan.FromMilliseconds(7)];

        var config = new QueryResultsRetryConfig(3, pattern);

        Assert.Equal(3, config.NoDataMaxRetries);
        Assert.Equal(pattern, config.RetryPattern);
    }
}
