using System.Data.Common;
using System.Globalization;
using System.Reflection;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbConnection = Snowflake.Data.SnowflakeDbConnection;
using NewSnowflakeDbConnectionPool = Snowflake.Data.SnowflakeDbConnectionPool;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbConnectionPool = Snowflake.Data.Client.SnowflakeDbConnectionPool;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

[Collection(nameof(ConnectionPoolCommonTest))]
public sealed class ConnectionPoolCommonTest : ReferenceTestBase, IDisposable
{
    private const int InvalidConnectionStringErrorCode = 270008;

    public ConnectionPoolCommonTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
        ClearAllPoolsOnOldDriver();
    }

    public void Dispose() => ClearAllPoolsOnOldDriver();

    [SnowflakeFact]
    public async Task ShouldOpenAndQueryPooledConnectionsFromMultipleThreadsAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        var connectionString = BaseConnectionString();

        await Task.WhenAll(
            Task.Run(() => OpenAndCloseConnectionAsync(connectionString, ct), ct),
            Task.Run(() => OpenAndQueryConnectionAsync(connectionString, ct), ct)).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task ShouldNotPoolConnectionThatFailedToOpenAsync(bool useOpenAsync)
    {
        Skip.When(useOpenAsync, "TODO Bug: SNOW-3552882");
        // TODO(SNOW-4382598): SnowflakeDbConnectionPool.GetPool and per-pool size lookups
        Skip.FutureMilestone();
        var ct = TestContext.Current.CancellationToken;
        var conn1 = Fixture.Factory.Create(Output, "bad connection string");

        if (useOpenAsync)
        {
            var thrownOnOpen = await Assert.ThrowsAnyAsync<DbException>(() => conn1.OpenAsync(ct)).ConfigureAwait(false);
            AssertSnowflakeDbErrorCode(thrownOnOpen, InvalidConnectionStringErrorCode);
        }
        else
        {
            AssertSnowflakeDbErrorCode(Assert.ThrowsAny<DbException>(() => conn1.Open()), InvalidConnectionStringErrorCode);
        }
        await conn1.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, conn1.State);
        var thrown = Assert.ThrowsAny<DbException>(() => GetPoolCurrentPoolSize(conn1.ConnectionString));
        AssertSnowflakeDbErrorCode(thrown, InvalidConnectionStringErrorCode);
        Assert.Contains("Connection string is invalid", thrown.Message);
    }

    [SnowflakeFact]
    public void ShouldFailWhenPreventingFromReturningToPoolNotOpenedConnection()
    {
        using var connection = Fixture.Factory.Create(Output, BaseConnectionString());

        var thrown = Assert.Throws<Exception>(() => connection.PreventPooling());

        Assert.Contains(
            "Session not yet created for this connection. Unable to prevent the session from pooling",
            thrown.Message);
    }

    [SnowflakeFact]
    public async Task ShouldRollBackTransactionOfPooledSessionWhenCommandFailsAsync()
    {
        // TODO(SNOW-4382598): pooled sessions and SnowflakeDbConnectionPool.GetCurrentPoolSize
        Skip.FutureMilestone();
        var ct = TestContext.Current.CancellationToken;
        var connectionString = SetPoolWithOneElement();
        string firstOpenedSessionId;
        using (var connection = CreateRawConnection(connectionString))
        {
            await connection.OpenAsync(ct).ConfigureAwait(false);
            firstOpenedSessionId = CurrentSessionId(connection);
            connection.BeginTransaction();
            Assert.True(HasActiveExplicitTransaction(connection));
            var thrown = await Assert.ThrowsAnyAsync<DbException>(async () =>
            {
                using var command = connection.CreateCommand();
                command.CommandText =
                    "invalid command will throw exception and leave session with an unfinished transaction";
                await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            }).ConfigureAwait(false);
            AssertIsSnowflakeDbException(thrown);
        }

        using (var connectionWithSessionReused = CreateRawConnection(connectionString))
        {
            await connectionWithSessionReused.OpenAsync(ct).ConfigureAwait(false);

            Assert.Equal(firstOpenedSessionId, CurrentSessionId(connectionWithSessionReused));
            Assert.False(HasActiveExplicitTransaction(connectionWithSessionReused));
            using var cmd = connectionWithSessionReused.CreateCommand();
            cmd.CommandText = "SELECT CURRENT_TRANSACTION()";
            Assert.Equal(DBNull.Value, await cmd.ExecuteScalarAsync(ct).ConfigureAwait(false));
        }

        Assert.Equal(1, GetCurrentPoolSize());
    }

    [SnowflakeFact]
    public async Task ShouldNotTrackTransactionStatusForNonExplicitTransactionCallsAsync()
    {
        // TODO(SNOW-4382598): pool configuration through maxPoolSize/minPoolSize/poolingEnabled
        Skip.FutureMilestone();
        var ct = TestContext.Current.CancellationToken;
        var connectionString = SetPoolWithOneElement();
        using var connection = CreateRawConnection(connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var command = connection.CreateCommand();
        command.CommandText = "BEGIN";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        Assert.False(HasActiveExplicitTransaction(connection));
    }

    [SnowflakeFact]
    public async Task ShouldRollBackTransactionOfPooledSessionWhenConnectionClosesAsync()
    {
        // TODO(SNOW-4382598): pooled sessions and SnowflakeDbConnectionPool.GetCurrentPoolSize
        Skip.FutureMilestone();
        var ct = TestContext.Current.CancellationToken;
        var connectionString = SetPoolWithOneElement();
        Assert.Equal(0, GetCurrentPoolSize());

        string firstOpenedSessionId;
        using (var connection1 = CreateRawConnection(connectionString))
        {
            await connection1.OpenAsync(ct).ConfigureAwait(false);
            Assert.Equal(1, GetCurrentPoolSize());
            connection1.BeginTransaction();
            Assert.True(HasActiveExplicitTransaction(connection1));
            firstOpenedSessionId = CurrentSessionId(connection1);
            using var command = connection1.CreateCommand();
            command.CommandText = "SELECT CURRENT_TRANSACTION()";
            Assert.NotEqual(DBNull.Value, await command.ExecuteScalarAsync(ct).ConfigureAwait(false));
        }
        Assert.Equal(1, GetCurrentPoolSize());

        using (var connection2 = CreateRawConnection(connectionString))
        {
            await connection2.OpenAsync(ct).ConfigureAwait(false);
            Assert.Equal(1, GetCurrentPoolSize());
            Assert.False(HasActiveExplicitTransaction(connection2));
            Assert.Equal(firstOpenedSessionId, CurrentSessionId(connection2));
            using var command = connection2.CreateCommand();
            command.CommandText = "SELECT CURRENT_TRANSACTION()";
            Assert.Equal(DBNull.Value, await command.ExecuteScalarAsync(ct).ConfigureAwait(false));
        }
        Assert.Equal(1, GetCurrentPoolSize());
    }

    private async Task OpenAndCloseConnectionAsync(string connectionString, CancellationToken ct)
    {
        var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);
        await conn1.CloseAsync(ct).ConfigureAwait(false);
        await Task.Delay(100, ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn1.State);
    }

    private async Task OpenAndQueryConnectionAsync(string connectionString, CancellationToken ct)
    {
        var conn1 = Fixture.Factory.Create(Output, connectionString);
        await conn1.OpenAsync(ct).ConfigureAwait(false);

        await Task.Delay(100, ct).ConfigureAwait(false);
        using (var command = conn1.CreateCommand())
        {
            command.CommandText = "select 1";
            using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
            Assert.Equal("1", reader.GetString(0));
        }
        await conn1.CloseAsync(ct).ConfigureAwait(false);
    }

    private string BaseConnectionString()
    {
        using var connection = Fixture.Factory.Create(Output);
        return connection.ConnectionString;
    }

    private string SetPoolWithOneElement() =>
        BaseConnectionString() + ";maxPoolSize=1;minPoolSize=0;poolingEnabled=true";

    private static string CurrentSessionId(IDbConnection connection)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT CURRENT_SESSION()";
        return Convert.ToString(command.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static void AssertIsSnowflakeDbException(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            Assert.IsType<OldSnowflakeDbException>(exception);
        else
            Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static void AssertSnowflakeDbErrorCode(Exception exception, int errorCode)
    {
        AssertIsSnowflakeDbException(exception);
        Assert.Equal(errorCode, SnowflakeDbErrorCode(exception));
    }

    private static int SnowflakeDbErrorCode(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? ((OldSnowflakeDbException)exception).ErrorCode
            : ((NewSnowflakeDbException)exception).ErrorCode;

    private static DbConnection CreateRawConnection(string connectionString) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? new OldSnowflakeDbConnection(connectionString)
            : new NewSnowflakeDbConnection(connectionString);

    private static void ClearAllPoolsOnOldDriver()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.ClearAllPools();
    }

    private static int GetCurrentPoolSize() =>
        TestDriver.Current == TestDriver.Kind.Old
            ? OldSnowflakeDbConnectionPool.GetCurrentPoolSize()
            : NewSnowflakeDbConnectionPool.GetCurrentPoolSize();

    private static int GetPoolCurrentPoolSize(string connectionString) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize()
            : NewSnowflakeDbConnectionPool.GetCurrentPoolSize(connectionString);

    // The old driver exposes its explicit-transaction tracking only through an internal member.
    private static bool HasActiveExplicitTransaction(DbConnection connection) =>
        (bool)connection.GetType()
            .GetMethod("HasActiveExplicitTransaction", BindingFlags.NonPublic | BindingFlags.Instance)!
            .Invoke(connection, null)!;

    [CollectionDefinition(nameof(ConnectionPoolCommonTest), DisableParallelization = true)]
    public sealed class SerialCollection
    {
    }
}
