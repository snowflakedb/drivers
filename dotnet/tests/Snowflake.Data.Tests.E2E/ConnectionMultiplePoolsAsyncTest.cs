extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbConnectionPool = Snowflake.Data.Client.SnowflakeDbConnectionPool;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbConnectionPool = OldDriver::Snowflake.Data.Client.SnowflakeDbConnectionPool;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

[CollectionDefinition(nameof(ConnectionMultiplePoolsAsyncTestCollection), DisableParallelization = true)]
public sealed class ConnectionMultiplePoolsAsyncTestCollection
{
}

[Collection(nameof(ConnectionMultiplePoolsAsyncTestCollection))]
public sealed class ConnectionMultiplePoolsAsyncTest : LiveE2ETestBase
{
    private const int MissingConnectionPropertyErrorCode = 270006;
    private const int InternalErrorCode = 270001;
    private const string ConnectionFailureSqlState = "08006";

    public ConnectionMultiplePoolsAsyncTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldAddToPoolOnOpenAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("minPoolSize=1;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, GetOldPoolSize(connection.ConnectionString));
        await connection.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailForInvalidConnectionAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        const string invalidConnectionString = ";connection_timeout=123";
        using var connection = Fixture.Factory.Create(Output, invalidConnectionString);

        var thrownOnOpen = await Assert.ThrowsAnyAsync<DbException>(
            () => connection.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);
        AssertSnowflakeDbErrorCode(thrownOnOpen, MissingConnectionPropertyErrorCode);

        AssertPoolLookupThrowsMissingProperty(connection.ConnectionString, "Required property ACCOUNT is not provided");
    }

    [SnowflakeFact]
    public async Task ShouldRunQueryInSamePoolAfterAbandonedOpenAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connStr = AppendPoolSettings(
            "minPoolSize=0;maxPoolSize=10;application=conn_pool_test_invalid_openasync2;poolingEnabled=true");
        using (var connection = Fixture.Factory.Create(Output, connStr))
            _ = connection.OpenAsync(CancellationToken.None);

        using var connection1 = Fixture.Factory.Create(Output, connStr);
        await connection1.OpenAsync(ct).ConfigureAwait(false);
        using var command = connection1.CreateCommand();
        command.CommandText = "select 1, 2, 3";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(3, reader.FieldCount);
        for (var i = 0; i < reader.FieldCount; i++)
            Assert.Equal(i + 1, Convert.ToInt32(reader.GetFieldValue<object>(i), CultureInfo.InvariantCulture));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task ShouldFillPoolUpToMinPoolSizeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("application=TestMinPoolSizeAsync;minPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);

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

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("minPoolSize=0;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));

        connection.PreventPooling();
        await connection.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(0, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldPoolConcurrentConnectionsAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var connStr = AppendPoolSettings(
            "application=TestConcurrentConnectionPoolingAsync2;ExpirationTimeout=3;poolingEnabled=true");
        await ConcurrentPoolingHelperAsync(connStr, true, 5, 5, 3, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldPoolConcurrentConnectionsClosedByDisposeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var connStr = AppendPoolSettings(
            "application=TestConcurrentConnectionPoolingDisposeAsync2;ExpirationTimeout=3;poolingEnabled=true");
        await ConcurrentPoolingHelperAsync(connStr, false, 5, 5, 3, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnConnectionToPoolOnCloseAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("minPoolSize=0;maxPoolSize=1;poolingEnabled=true");
        using var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn1.State);
        await conn1.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(1, GetOldPoolSize(connectionString));
        Assert.Equal(1, GetOldPoolSizeWithoutPassword(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldReuseSessionInConnectionPoolAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("minPoolSize=1;poolingEnabled=true");
        using var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn1.State);
        await conn1.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));

        using var conn2 = Fixture.Factory.Create(Output, connectionString);
        await conn2.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn2.State);
        Assert.Equal(1, GetOldPoolSize(connectionString));

        await conn2.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));
        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(ConnectionState.Closed, conn2.State);
    }

    [SnowflakeFact]
    public async Task ShouldReuseSessionInConnectionPoolReachingMaxConnectionsAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("maxPoolSize=2;minPoolSize=1;poolingEnabled=true");
        using var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn1.State);

        using var conn2 = Fixture.Factory.Create(Output, connectionString);
        await conn2.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn2.State);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await conn2.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        using var conn3 = Fixture.Factory.Create(Output, connectionString);
        await conn3.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn3.State);

        using var conn4 = Fixture.Factory.Create(Output, connectionString);
        await conn4.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn4.State);

        await conn3.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(2, GetOldPoolSize(connectionString));
        await conn4.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(ConnectionState.Closed, conn2.State);
        Assert.Equal(ConnectionState.Closed, conn3.State);
        Assert.Equal(ConnectionState.Closed, conn4.State);
    }

    [SnowflakeFact]
    public async Task ShouldWaitForTheIdleConnectionWhenExceedingMaxConnectionsLimitAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=TestWaitForMaxSize2;waitingForIdleSessionTimeout=1s;maxPoolSize=2;minPoolSize=1;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));
        using var conn1 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        using var conn2 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        var started = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var thrown = await Assert.ThrowsAnyAsync<DbException>(
            () => OpenConnectionAsync(connectionString, ct)).ConfigureAwait(false);
        var elapsed = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - started;

        AssertPoolWaitTimeoutException(thrown);
        Assert.InRange(elapsed, 1000, long.MaxValue);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await conn2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldCountBusyAndIdleConnectionsInPoolSizeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("maxPoolSize=2;minPoolSize=1;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));
        await connection.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldNotPoolConnectionWhenPoolingDisabledAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var disabledPoolString = AppendPoolSettings("poolingEnabled=false");
        using var conn1 = Fixture.Factory.Create(Output);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn1.State);
        Assert.Equal(0, GetOldPoolSize(disabledPoolString));
        await conn1.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(0, GetOldPoolSize(disabledPoolString));
    }

    [SnowflakeFact]
    public async Task ShouldClearIdleConnectionsFromAllPoolsAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings("maxPoolSize=2;minPoolSize=1;poolingEnabled=true;");
        using var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn1.State);

        using var conn2 = Fixture.Factory.Create(Output, connectionString + "retryCount=1;");
        await conn2.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn2.State);

        using var conn3 = Fixture.Factory.Create(Output, connectionString + "retryCount=2;");
        await conn3.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn3.State);

        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await conn2.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(conn1.ConnectionString));
        Assert.Equal(1, GetOldPoolSize(conn2.ConnectionString));
        OldSnowflakeDbConnectionPool.ClearAllPools();
        Assert.Equal(0, GetOldPoolSize(conn1.ConnectionString));
        Assert.Equal(0, GetOldPoolSize(conn2.ConnectionString));
        await conn3.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(1, GetOldPoolSize(conn3.ConnectionString));

        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(ConnectionState.Closed, conn2.State);
        Assert.Equal(ConnectionState.Closed, conn3.State);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldReplaceExpiredSessionsInPoolAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with expiration
        Skip.FutureMilestone();

        const int expirationTimeoutInSeconds = 10;
        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            $"expirationTimeout={expirationTimeoutInSeconds};maxPoolSize=4;minPoolSize=2;poolingEnabled=true");
        Assert.Equal(0, GetOldPoolSize(connectionString));

        using var conn1 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        using var conn2 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        using var conn3 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        using var conn4 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        Assert.Equal(4, GetOldPoolSize(connectionString));
        var sessionIdsBeforeExpiration = new[]
        {
            await QueryCurrentSessionIdAsync(conn1, ct).ConfigureAwait(false),
            await QueryCurrentSessionIdAsync(conn2, ct).ConfigureAwait(false),
            await QueryCurrentSessionIdAsync(conn3, ct).ConfigureAwait(false),
            await QueryCurrentSessionIdAsync(conn4, ct).ConfigureAwait(false),
        };

        await Task.Delay(TimeSpan.FromSeconds(expirationTimeoutInSeconds), ct).ConfigureAwait(false);
        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await conn2.CloseAsync(ct).ConfigureAwait(false);
        await conn3.CloseAsync(ct).ConfigureAwait(false);
        await conn4.CloseAsync(ct).ConfigureAwait(false);
        Assert.Equal(2, GetOldPoolSize(connectionString));

        using var conn5 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        Assert.Equal(2, GetOldPoolSize(connectionString));
        Assert.Equal(ConnectionState.Open, conn5.State);
        var conn5SessionId = await QueryCurrentSessionIdAsync(conn5, ct).ConfigureAwait(false);
        Assert.DoesNotContain(conn5SessionId, sessionIdsBeforeExpiration);
        using var conn6 = await OpenConnectionAsync(connectionString, ct).ConfigureAwait(false);
        var conn6SessionId = await QueryCurrentSessionIdAsync(conn6, ct).ConfigureAwait(false);
        Assert.DoesNotContain(conn6SessionId, sessionIdsBeforeExpiration);
        Assert.NotEqual(conn5SessionId, conn6SessionId);
    }

    private string AppendPoolSettings(string settings)
    {
        using var probe = Fixture.Factory.Create(Output);
        return PoolConnectionString.Wrap(probe.ConnectionString, settings);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        return connection;
    }

    private static async Task<string> QueryCurrentSessionIdAsync(
        ISnowflakeDbConnection connection,
        CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT CURRENT_SESSION()";
        var sessionId = await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false);
        return Convert.ToString(sessionId, CultureInfo.InvariantCulture)!;
    }

    private async Task ConcurrentPoolingHelperAsync(
        string connectionString,
        bool closeConnection,
        int tasksCount,
        int connectionsInTask,
        int abandonedConnectionsCount,
        CancellationToken cancellationToken)
    {
        var tasks = new Task[tasksCount + 1];
        for (var i = 0; i < tasksCount; i++)
            tasks[i] = QueryExecutionTaskAsync(connectionString, closeConnection, connectionsInTask, cancellationToken);
        tasks[tasksCount] = InvalidConnectionTaskAsync(connectionString, abandonedConnectionsCount, cancellationToken);
        await Task.WhenAll(tasks).ConfigureAwait(false);
    }

    private async Task QueryExecutionTaskAsync(
        string connectionString,
        bool closeConnection,
        int times,
        CancellationToken cancellationToken)
    {
        for (var i = 0; i < times; i++)
        {
            using var conn = Fixture.Factory.Create(Output, connectionString);
            await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "select 1, 2, 3";
            try
            {
                using var reader = await cmd.ExecuteReaderAsync(cancellationToken).ConfigureAwait(false);
                while (await reader.ReadAsync(cancellationToken).ConfigureAwait(false))
                {
                    for (var j = 0; j < reader.FieldCount; j++)
                        await reader.GetFieldValueAsync<object>(j, cancellationToken).ConfigureAwait(false);
                }
            }
            catch (Exception e)
            {
                Assert.Fail("Caught unexpected exception: " + e);
            }

            if (closeConnection)
                await conn.CloseAsync(cancellationToken).ConfigureAwait(false);
        }
    }

    private async Task InvalidConnectionTaskAsync(string connectionString, int times, CancellationToken cancellationToken)
    {
        for (var i = 0; i < times; i++)
        {
            using (var conn = Fixture.Factory.Create(Output, connectionString))
                _ = conn.OpenAsync(CancellationToken.None);
            await Task.Delay(100, cancellationToken).ConfigureAwait(false);
        }
    }

    private static int GetOldPoolSize(string connectionString) =>
        OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize();

    private static int GetOldPoolSizeWithoutPassword(string connectionString) =>
        OldSnowflakeDbConnectionPool.GetPool(connectionString, null!).GetCurrentPoolSize();

    private static void AssertPoolLookupThrowsMissingProperty(string connectionString, string expectedMessage)
    {
        var thrown = TestDriver.Current == TestDriver.Kind.Old
            ? Record.Exception(() => OldSnowflakeDbConnectionPool.GetPool(connectionString))
            : Record.Exception(() => NewSnowflakeDbConnectionPool.GetCurrentPoolSize(connectionString));
        Assert.NotNull(thrown);
        AssertSnowflakeDbErrorCode(thrown, MissingConnectionPropertyErrorCode);
        Assert.Contains(expectedMessage, thrown.Message);
    }

    private static void AssertSnowflakeDbErrorCode(Exception exception, int errorCode) =>
        Assert.Equal(errorCode, SnowflakeDbErrorCode(exception));

    private static void AssertPoolWaitTimeoutException(DbException exception)
    {
        Assert.Equal(InternalErrorCode, SnowflakeDbErrorCode(exception));
        Assert.Equal(ConnectionFailureSqlState, SnowflakeDbSqlState(exception));
        Assert.Contains("Unable to connect", exception.Message);
        Assert.NotNull(exception.InnerException);
        Assert.Contains(
            "Could not obtain a connection from the pool within a given timeout",
            exception.InnerException.Message);
    }

    private static int SnowflakeDbErrorCode(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.IsType<OldSnowflakeDbException>(exception).ErrorCode
            : Assert.IsType<NewSnowflakeDbException>(exception).ErrorCode;

    private static string? SnowflakeDbSqlState(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.IsType<OldSnowflakeDbException>(exception).SqlState
            : Assert.IsType<NewSnowflakeDbException>(exception).SqlState;
}
