using NSubstitute;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class QueryStatusPollerTest
{
    private const int NoDataMaxRetries = 2;

    private static readonly ConnectionHandle Connection = new() { Id = 3, Magic = 17 };
    private static readonly QueryId SampleQueryId = QueryId.Parse("01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5");
    private static readonly QueryResultsRetryConfig ZeroDelayConfig = new(NoDataMaxRetries, Enumerable.Repeat(TimeSpan.Zero, 2 * NoDataMaxRetries));

    private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();
    private readonly QueryStatusPoller _poller;

    public QueryStatusPollerTest()
    {
        _poller = new QueryStatusPoller(_driver, ZeroDelayConfig);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task FetchStatusSendsConnectionAndQueryIdAndMapsTheServerNameAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>()).Returns(Status("RUNNING"));

        var status = await _poller.FetchStatusAsync(Connection, SampleQueryId, CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(QueryStatus.Running, status);
        _ = _driver.Received(1).ConnectionGetQueryStatusAsync(
            Arg.Is<ConnectionGetQueryStatusRequest>(request => request.ConnHandle.Equals(Connection) && request.QueryId == SampleQueryId.Value),
            Arg.Any<CancellationToken>());
        Assert.Single(_driver.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task FetchStatusRejectsAnUnknownStatusNameAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>()).Returns(Status("MYSTERY"));

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(
            () => _poller.FetchStatusAsync(Connection, SampleQueryId, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(SFError.UNKNOWN_QUERY_STATUS, exception.Error);
        Assert.Equal(SampleQueryId.Value, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task WaitPollsUntilTheQueryStopsRunningAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>())
            .Returns(Status("RUNNING"), Status("QUEUED"), Status("RESUMING_WAREHOUSE"), Status("SUCCESS"));
        var handleLookups = 0;

        await _poller.WaitUntilFinishedAsync(() =>
        {
            handleLookups++;
            return Connection;
        }, SampleQueryId, CancellationToken.None).ConfigureAwait(false);

        _ = _driver.ReceivedWithAnyArgs(4).ConnectionGetQueryStatusAsync(default!, default);
        Assert.Equal(4, handleLookups);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("FAILED_WITH_ERROR")]
    [InlineData("ABORTED")]
    public async Task WaitEndsWithoutThrowingForAFailedQueryAsync(string finalStatus)
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>())
            .Returns(Status("RUNNING"), Status(finalStatus));

        await _poller.WaitUntilFinishedAsync(() => Connection, SampleQueryId, CancellationToken.None).ConfigureAwait(false);

        _ = _driver.ReceivedWithAnyArgs(2).ConnectionGetQueryStatusAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task WaitGivesUpAfterTooManyNoDataAnswersAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>()).Returns(Status("NO_DATA"));

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(
            () => _poller.WaitUntilFinishedAsync(() => Connection, SampleQueryId, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED, exception.Error);
        Assert.Equal(SampleQueryId.Value, exception.QueryId);
        _ = _driver.ReceivedWithAnyArgs(NoDataMaxRetries + 1).ConnectionGetQueryStatusAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancellationInterruptsTheDelayBetweenPollsAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>())
            .Returns(Status("RUNNING"));
        var poller = new QueryStatusPoller(_driver, new QueryResultsRetryConfig(NoDataMaxRetries, [TimeSpan.FromHours(1)]));
        using var cancellation = new CancellationTokenSource(TimeSpan.FromMilliseconds(100));

        var waiting = poller.WaitUntilFinishedAsync(() => Connection, SampleQueryId, cancellation.Token);
        var finished = await Task.WhenAny(waiting, Task.Delay(TimeSpan.FromSeconds(10), CancellationToken.None)).ConfigureAwait(false);

        Assert.Same(waiting, finished);
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => waiting).ConfigureAwait(false);
        _ = _driver.ReceivedWithAnyArgs(1).ConnectionGetQueryStatusAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task WaitWithCancelledTokenSendsNothingAsync()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => _poller.WaitUntilFinishedAsync(() => Connection, SampleQueryId, cancellation.Token)).ConfigureAwait(false);

        Assert.Empty(_driver.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task WaitStopsWhenTheConnectionHandleIsNoLongerAvailableAsync()
    {
        _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>()).Returns(Status("RUNNING"));
        var closed = new SnowflakeDriverException(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, null);
        var handleLookups = 0;

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => _poller.WaitUntilFinishedAsync(
            () => ++handleLookups == 1 ? Connection : throw closed, SampleQueryId, CancellationToken.None)).ConfigureAwait(false);

        Assert.Same(closed, exception);
        _ = _driver.ReceivedWithAnyArgs(1).ConnectionGetQueryStatusAsync(default!, default);
    }

    private static ConnectionGetQueryStatusResponse Status(string statusName) => new() { StatusName = statusName };
}
