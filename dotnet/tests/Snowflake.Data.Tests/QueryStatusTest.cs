using Snowflake.Data.Core;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class QueryStatusTest
{
    [SnowflakeFact]
    public void IsOpen_ReflectsConnectionStateWithoutConnecting()
    {
        using ISnowflakeDbConnection connection = new SnowflakeDbConnection();
        Assert.False(connection.IsOpen());
    }

    [SnowflakeFact]
    public async Task CloseAsync_LeavesNeverOpenedConnectionClosedAsync()
    {
        using ISnowflakeDbConnection connection = new SnowflakeDbConnection();
        await connection.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.False(connection.IsOpen());
    }

    [SnowflakeFact]
    public async Task CloseAsync_WhenCancellationAlreadyRequested_ThrowsAndLeavesConnectionClosedAsync()
    {
        using ISnowflakeDbConnection connection = new SnowflakeDbConnection();
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        await Assert.ThrowsAsync<TaskCanceledException>(
            () => connection.CloseAsync(cts.Token)).ConfigureAwait(false);
        Assert.False(connection.IsOpen());
    }

    [SnowflakeTheory]
    [InlineData(QueryStatus.NoData, true)]
    [InlineData(QueryStatus.Running, true)]
    [InlineData(QueryStatus.ResumingWarehouse, true)]
    [InlineData(QueryStatus.Queued, true)]
    [InlineData(QueryStatus.QueuedReparingWarehouse, true)]
    [InlineData(QueryStatus.Aborting, false)]
    [InlineData(QueryStatus.Success, false)]
    [InlineData(QueryStatus.FailedWithError, false)]
    [InlineData(QueryStatus.Aborted, false)]
    [InlineData(QueryStatus.FailedWithIncident, false)]
    [InlineData(QueryStatus.Disconnected, false)]
    [InlineData(QueryStatus.Restarted, false)]
    [InlineData(QueryStatus.Blocked, false)]
    public void IsStillRunning_MatchesExpectedStatusClassification(QueryStatus status, bool expected)
    {
        using ISnowflakeDbConnection connection = new SnowflakeDbConnection();
        Assert.Equal(expected, connection.IsStillRunning(status));
    }

    [SnowflakeTheory]
    [InlineData(QueryStatus.Aborting, true)]
    [InlineData(QueryStatus.FailedWithError, true)]
    [InlineData(QueryStatus.Aborted, true)]
    [InlineData(QueryStatus.FailedWithIncident, true)]
    [InlineData(QueryStatus.Disconnected, true)]
    [InlineData(QueryStatus.Blocked, true)]
    [InlineData(QueryStatus.NoData, false)]
    [InlineData(QueryStatus.Running, false)]
    [InlineData(QueryStatus.Success, false)]
    [InlineData(QueryStatus.Queued, false)]
    [InlineData(QueryStatus.ResumingWarehouse, false)]
    [InlineData(QueryStatus.QueuedReparingWarehouse, false)]
    [InlineData(QueryStatus.Restarted, false)]
    public void IsAnError_MatchesExpectedStatusClassification(QueryStatus status, bool expected)
    {
        using ISnowflakeDbConnection connection = new SnowflakeDbConnection();
        Assert.Equal(expected, connection.IsAnError(status));
    }
}
