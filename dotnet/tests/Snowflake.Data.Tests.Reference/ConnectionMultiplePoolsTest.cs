using System.Collections.Concurrent;
using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using Snowflake.Data.Tests.Reference.Wiremock;
using NewSnowflakeDbConnectionPool = Snowflake.Data.SnowflakeDbConnectionPool;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbConnectionPool = Snowflake.Data.Client.SnowflakeDbConnectionPool;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

[CollectionDefinition(nameof(ConnectionMultiplePoolsTestCollection), DisableParallelization = true)]
public sealed class ConnectionMultiplePoolsTestCollection
{
}

[Collection(nameof(ConnectionMultiplePoolsTestCollection))]
public sealed class ConnectionMultiplePoolsTest : ReferenceTestBase
{
    private const int QueryCancelledErrorCode = 270005;
    private const int InternalErrorCode = 270001;
    private const string ConnectionFailureSqlState = "08006";
    private const string WaitQuery = "SELECT SYSTEM$WAIT(20)";
    private const string BeginQueryRegex = @"(?s)(?i).*\bbegin\b.*";
    private const string RollbackQueryRegex = "(?s)(?i).*rollback.*";

    public ConnectionMultiplePoolsTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldWaitForTheIdleConnectionWhenExceedingMaxConnectionsLimitAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        ClearAllPoolsOnOldDriver();
        var connectionString = AppendPoolSettings(
            "application=TestWaitForMaxSize1;waitingForIdleSessionTimeout=1s;maxPoolSize=2;minPoolSize=1;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));
        using var conn1 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        using var conn2 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        var started = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var thrown = Assert.ThrowsAny<DbException>(() => OpenConnection(connectionString));
        var elapsed = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - started;

        AssertPoolWaitTimeoutException(thrown);
        Assert.InRange(elapsed, 1000, 5000);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await conn2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldWaitInQueueForIdleSessionAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with max enforcement and wait
        Skip.FutureMilestone();

        const long aDelay = 0;
        const long bDelay = 400;
        const long cDelay = 2 * bDelay;
        const long dDelay = 3 * bDelay;
        const long abDelayAfterConnect = 2000;
        const long connectPessimisticEstimate = 2500;
        const long startDelayPessimisticEstimate = 350;
        const long aMinConnectionReleaseTime = aDelay + abDelayAfterConnect;
        const long aMaxConnectionReleaseTime =
            aDelay + startDelayPessimisticEstimate + connectPessimisticEstimate + abDelayAfterConnect;
        const long bMinConnectionReleaseTime = bDelay + abDelayAfterConnect;
        const long bMaxConnectionReleaseTime =
            bDelay + startDelayPessimisticEstimate + connectPessimisticEstimate + abDelayAfterConnect;
        const long cMinConnectDuration = aMinConnectionReleaseTime - cDelay - startDelayPessimisticEstimate;
        const long cMaxConnectDuration = aMaxConnectionReleaseTime - cDelay;
        const long dMinConnectDuration = bMinConnectionReleaseTime - dDelay - startDelayPessimisticEstimate;
        const long dMaxConnectDuration = bMaxConnectionReleaseTime - dDelay;
        const long measurementTolerance = 25;

        var ct = TestContext.Current.CancellationToken;
        ClearAllPoolsOnOldDriver();
        var connectionString = AppendPoolSettings(
            "application=TestWaitForMaxSize3;waitingForIdleSessionTimeout=15s;maxPoolSize=2;minPoolSize=0;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));
        var connectedEvents = new ConcurrentQueue<(string Name, long DurationMs)>();
        var sessionIds = new ConcurrentDictionary<string, string>();

        await Task.WhenAll(
            ConnectInBackgroundAsync("A", connectionString, aDelay, abDelayAfterConnect, connectedEvents, sessionIds, ct),
            ConnectInBackgroundAsync("B", connectionString, bDelay, abDelayAfterConnect, connectedEvents, sessionIds, ct),
            ConnectInBackgroundAsync("C", connectionString, cDelay, 0, connectedEvents, sessionIds, ct),
            ConnectInBackgroundAsync("D", connectionString, dDelay, 0, connectedEvents, sessionIds, ct))
            .ConfigureAwait(false);

        var events = connectedEvents.ToList();
        Assert.Equal(4, events.Count);
        var firstConnectedGroup = events.GetRange(0, 2);
        Assert.Equivalent(new[] { "A", "B" }, firstConnectedGroup.Select(e => e.Name));
        var lastConnectedGroup = events.GetRange(2, 2);
        Assert.Equal(new[] { "C", "D" }, lastConnectedGroup.Select(e => e.Name));
        Assert.InRange(firstConnectedGroup[0].DurationMs, long.MinValue, connectPessimisticEstimate);
        Assert.InRange(firstConnectedGroup[1].DurationMs, long.MinValue, connectPessimisticEstimate);
        Assert.InRange(
            lastConnectedGroup[0].DurationMs,
            cMinConnectDuration - measurementTolerance,
            cMaxConnectDuration);
        Assert.InRange(
            lastConnectedGroup[1].DurationMs,
            dMinConnectDuration - measurementTolerance,
            dMaxConnectDuration);
        var sessionIdsCreatedForAAndB = new[] { sessionIds["A"], sessionIds["B"] };
        Assert.Contains(sessionIds["C"], sessionIdsCreatedForAAndB);
        Assert.Contains(sessionIds["D"], sessionIdsCreatedForAAndB);
        Assert.Equal(2, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public void ShouldThrowWhenDisablingPoolingForAllPools()
    {
        ClearAllPoolsOnOldDriver();
        var thrown = Assert.Throws<Exception>(SetPoolingDisabled);
        Assert.Equal(
            "You cannot change connection pool parameters for all the pools. Instead you can change it on a particular pool",
            thrown.Message);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Once)]
    public async Task ShouldFillPoolUpToMinPoolSizeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        ClearAllPoolsOnOldDriver();
        var connectionString = AppendPoolSettings("application=TestMinPoolSize;minPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        connection.Open();

        var deadline = DateTime.UtcNow + TimeSpan.FromMilliseconds(1000);
        while (DateTime.UtcNow < deadline && GetOldPoolSize(connection.ConnectionString) != 3)
            await Task.Delay(50, ct).ConfigureAwait(false);

        Assert.Equal(3, GetOldPoolSize(connection.ConnectionString));
        await connection.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldPreventConnectionFromReturningToPoolAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        ClearAllPoolsOnOldDriver();
        var connectionString = AppendPoolSettings("minPoolSize=0;poolingEnabled=true");
        using var connection = await OpenConnectionAsync(connectionString, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));
        connection.PreventPooling();
        connection.Close();
        Assert.Equal(0, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldReleaseConnectionWhenRollbackFailsAsync()
    {
        // TODO(SNOW-4382598): real connection pooling and transactions
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        ClearAllPoolsOnOldDriver();
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct,
            "auth/login_success_any.json",
            "query/begin_success.json",
            "query/rollback_fails.json",
            "session/logout_success.json").ConfigureAwait(false);
        var connectionString = ReferenceWiremock.BuildHttpConnectionString(
            wiremock,
            "minPoolSize=0;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var transaction = connection.BeginTransaction();
        Assert.Equal(1, await ReferenceWiremock.CountQueryRequestsAsync(wiremock, BeginQueryRegex, ct).ConfigureAwait(false));
        Assert.Equal(1, GetOldPoolSize(connectionString));

        connection.Close();

        Assert.Equal(1, await ReferenceWiremock.CountQueryRequestsAsync(wiremock, RollbackQueryRegex, ct).ConfigureAwait(false));
        Assert.Equal(0, GetOldPoolSize(connectionString));
    }

    [SnowflakeTheory(retriesCount: RetriesCount.Once)]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ShouldReturnCancelledSessionToPoolAsync(bool cancelAsync)
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        ClearAllPoolsOnOldDriver();
        var connectionString = AppendPoolSettings(
            "minPoolSize=0;maxPoolSize=2;application=TestReturningCancelledSessionsToThePool;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));

        using var cts = new CancellationTokenSource();
        var opened = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        var task = Task.Run(async () =>
        {
            using var connection = Fixture.Factory.Create(Output, connectionString);
            await connection.OpenAsync(cts.Token).ConfigureAwait(false);
            opened.SetResult(await QueryCurrentSessionIdAsync(connection, cts.Token).ConfigureAwait(false));
            using var command = connection.CreateCommand();
            command.CommandText = WaitQuery;
            await command.ExecuteNonQueryAsync(cts.Token).ConfigureAwait(false);
        }, CancellationToken.None);

        if (await Task.WhenAny(opened.Task, task).ConfigureAwait(false) == task)
            await task.ConfigureAwait(false);
        var sessionId = await opened.Task.ConfigureAwait(false);
        Assert.True(
            await WaitUntilQueryRunsInSessionAsync(sessionId, WaitQuery, TimeSpan.FromSeconds(10), ct).ConfigureAwait(false),
            $"{WaitQuery} did not start running in session {sessionId}");
        Assert.Equal(1, GetOldPoolSize(connectionString));

        if (cancelAsync)
            await cts.CancelAsync().ConfigureAwait(false);
        else
            cts.Cancel();

        var cancelled = await Assert.ThrowsAsync<OperationCanceledException>(() => task).ConfigureAwait(false);
        AssertSnowflakeDbErrorCode(cancelled.InnerException, QueryCancelledErrorCode);
        Assert.Equal(1, GetOldPoolSize(connectionString));
        using var reusingConnection = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        Assert.Equal(sessionId, await QueryCurrentSessionIdAsync(reusingConnection, ct).ConfigureAwait(false));
        Assert.Equal(1, GetOldPoolSize(connectionString));
    }

    private string AppendPoolSettings(string settings)
    {
        using var probe = Fixture.Factory.Create(Output);
        return $"{probe.ConnectionString};{settings}";
    }

    private ISnowflakeDbConnection OpenConnection(string connectionString)
    {
        var connection = Fixture.Factory.Create(Output, connectionString);
        connection.Open();
        return connection;
    }

    private Task ConnectInBackgroundAsync(
        string name,
        string connectionString,
        long delayBeforeConnectMs,
        long delayAfterConnectMs,
        ConcurrentQueue<(string Name, long DurationMs)> connectedEvents,
        ConcurrentDictionary<string, string> sessionIds,
        CancellationToken cancellationToken) =>
        Task.Run(
            async () =>
            {
                using var connection = Fixture.Factory.Create(Output, connectionString);
                await Task.Delay(TimeSpan.FromMilliseconds(delayBeforeConnectMs), cancellationToken).ConfigureAwait(false);
                var stopwatch = Stopwatch.StartNew();
                connection.Open();
                stopwatch.Stop();
                connectedEvents.Enqueue((name, stopwatch.ElapsedMilliseconds));
                var holdConnection = Task.Delay(TimeSpan.FromMilliseconds(delayAfterConnectMs), cancellationToken);
                sessionIds[name] = await QueryCurrentSessionIdAsync(connection, cancellationToken).ConfigureAwait(false);
                await holdConnection.ConfigureAwait(false);
                connection.Close();
            },
            cancellationToken);

    private static async Task<string> QueryCurrentSessionIdAsync(
        ISnowflakeDbConnection connection,
        CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT CURRENT_SESSION()";
        var sessionId = await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false);
        return Convert.ToString(sessionId, CultureInfo.InvariantCulture)!;
    }

    private async Task<bool> WaitUntilQueryRunsInSessionAsync(
        string sessionId,
        string queryText,
        TimeSpan timeout,
        CancellationToken cancellationToken)
    {
        var sessionNumber = long.Parse(sessionId, CultureInfo.InvariantCulture);
        using var monitor = Fixture.Factory.Create(Output);
        await monitor.OpenAsync(cancellationToken).ConfigureAwait(false);
        using var command = monitor.CreateCommand();
        command.CommandText =
            $"SELECT COUNT(*) FROM TABLE(INFORMATION_SCHEMA.QUERY_HISTORY_BY_SESSION(SESSION_ID => {sessionNumber})) "
            + "WHERE EXECUTION_STATUS = 'RUNNING' AND QUERY_TEXT = ?";
        var parameter = command.CreateParameter();
        parameter.ParameterName = "1";
        parameter.DbType = DbType.String;
        parameter.Value = queryText;
        command.Parameters.Add(parameter);

        var deadline = DateTime.UtcNow + timeout;
        while (DateTime.UtcNow < deadline)
        {
            var running = Convert.ToInt64(
                await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false),
                CultureInfo.InvariantCulture);
            if (running > 0)
                return true;
            await Task.Delay(100, cancellationToken).ConfigureAwait(false);
        }

        return false;
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        return connection;
    }

    private static void ClearAllPoolsOnOldDriver()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.ClearAllPools();
    }

    private static int GetOldPoolSize(string connectionString) =>
        OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize();

    private static void AssertSnowflakeDbErrorCode(Exception? exception, int errorCode) =>
        Assert.Equal(errorCode, SnowflakeDbErrorCode(exception));

    private static void AssertPoolWaitTimeoutException(DbException exception)
    {
        Assert.Equal(InternalErrorCode, SnowflakeDbErrorCode(exception));
        Assert.Equal(ConnectionFailureSqlState, SnowflakeDbSqlState(exception));
        Assert.Contains(
            "Unable to connect. Could not obtain a connection from the pool within a given timeout",
            exception.Message);
    }

    private static int SnowflakeDbErrorCode(Exception? exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.IsType<OldSnowflakeDbException>(exception).ErrorCode
            : Assert.IsType<NewSnowflakeDbException>(exception).ErrorCode;

    private static string SnowflakeDbSqlState(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.IsType<OldSnowflakeDbException>(exception).SqlState
            : Assert.IsType<NewSnowflakeDbException>(exception).SqlState;

    private static void SetPoolingDisabled()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.SetPooling(false);
        else
            NewSnowflakeDbConnectionPool.SetPooling(false);
    }
}
