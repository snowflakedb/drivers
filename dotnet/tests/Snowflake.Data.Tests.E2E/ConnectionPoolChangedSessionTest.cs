extern alias OldDriver;
using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbConnectionPool = OldDriver::Snowflake.Data.Client.SnowflakeDbConnectionPool;

namespace Snowflake.Data.Tests.E2E;

[CollectionDefinition(nameof(ConnectionPoolChangedSessionTestCollection), DisableParallelization = true)]
public sealed class ConnectionPoolChangedSessionTestCollection
{
}

[Collection(nameof(ConnectionPoolChangedSessionTestCollection))]
public sealed class ConnectionPoolChangedSessionTest : LiveE2ETestBase, IDisposable
{
    public ConnectionPoolChangedSessionTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
        ClearAllPoolsOnOldDriver();
    }

    public void Dispose() => ClearAllPoolsOnOldDriver();

    [SnowflakeFact]
    public async Task ShouldDestroyPooledConnectionWhenSessionPropertiesChangeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=Destroy;ChangedSession=Destroy;minPoolSize=0;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        // SNOWFLAKE is the shared database that exists in every account.
        await ExecuteNonQueryAsync(connection, "USE DATABASE SNOWFLAKE", ct).ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(0, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldReturnConnectionToPoolWhenSessionPropertiesUnchangedAsync()
    {
        // TODO(SNOW-4382598): real connection pooling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=NoSessionChanges;ChangedSession=Destroy;minPoolSize=0;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, GetOldPoolSize(connectionString));
    }

    [SnowflakeFact]
    public async Task ShouldReuseSessionWhenSessionPropertiesChangeInOriginalPoolModeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=OriginalPoolMode;ChangedSession=OriginalPool;minPoolSize=0;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, "USE SCHEMA INFORMATION_SCHEMA", ct).ConfigureAwait(false);
        var sessionId = await CurrentSessionIdAsync(connection, ct).ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, GetOldPoolSize(connectionString));
        await connection.CloseAsync(ct).ConfigureAwait(false);

        using var connection2 = Fixture.Factory.Create(Output, connectionString);
        await connection2.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(sessionId, await CurrentSessionIdAsync(connection2, ct).ConfigureAwait(false));
        await connection2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldDestroySessionWhenSessionPropertiesChangeInDefaultPoolModeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=DefaultPoolMode;minPoolSize=0;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, "USE SCHEMA INFORMATION_SCHEMA", ct).ConfigureAwait(false);
        var sessionId = await CurrentSessionIdAsync(connection, ct).ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(0, GetOldPoolSize(connectionString));

        using var connection2 = Fixture.Factory.Create(Output, connectionString);
        await connection2.OpenAsync(ct).ConfigureAwait(false);
        Assert.NotEqual(sessionId, await CurrentSessionIdAsync(connection2, ct).ConfigureAwait(false));
        await connection2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldRecreateDestroyedSessionToKeepMinPoolSizeAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=DestroyRecreateSession;ChangedSession=Destroy;minPoolSize=1;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        var sessionId = await CurrentSessionIdAsync(connection, ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, "USE SCHEMA INFORMATION_SCHEMA", ct).ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, GetOldPoolSize(connectionString));

        using var connection2 = Fixture.Factory.Create(Output, connectionString);
        await connection2.OpenAsync(ct).ConfigureAwait(false);
        Assert.NotEqual(sessionId, await CurrentSessionIdAsync(connection2, ct).ConfigureAwait(false));
        await connection2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReuseSessionWhenUnquotedSessionPropertiesDifferOnlyInCaseAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var connectionString = AppendPoolSettings(
            "application=CompareCaseInsensitive;ChangedSession=Destroy;minPoolSize=1;maxPoolSize=3;poolingEnabled=true");
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        var sessionId = await CurrentSessionIdAsync(connection, ct).ConfigureAwait(false);
        var database = await QueryStringAsync(connection, "SELECT CURRENT_DATABASE()", ct).ConfigureAwait(false);
        var schema = await QueryStringAsync(connection, "SELECT CURRENT_SCHEMA()", ct).ConfigureAwait(false);
        var warehouse = await QueryStringAsync(connection, "SELECT CURRENT_WAREHOUSE()", ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"USE DATABASE {database.ToLowerInvariant()}", ct)
            .ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"USE SCHEMA {schema.ToUpperInvariant()}", ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"USE WAREHOUSE {warehouse.ToLowerInvariant()}", ct)
            .ConfigureAwait(false);
        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, GetOldPoolSize(connectionString));

        using var connection2 = Fixture.Factory.Create(Output, connectionString);
        await connection2.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(sessionId, await CurrentSessionIdAsync(connection2, ct).ConfigureAwait(false));
        await connection2.CloseAsync(ct).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldDestroySessionWhenQuotedSessionPropertyChangesAsync()
    {
        // TODO(SNOW-4382598): real connection pooling with changed-session handling
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var quotedSchema = $"\"SomeQuotedSchema_{Guid.NewGuid():N}\"";
        await ExecuteOnSeparateSessionAsync($"CREATE SCHEMA {quotedSchema}", ct).ConfigureAwait(false);
        try
        {
            var connectionString = AppendPoolSettings(
                "application=CompareCaseSensitive;ChangedSession=Destroy;minPoolSize=1;maxPoolSize=3;poolingEnabled=true");
            using var connection = Fixture.Factory.Create(Output, connectionString);
            await connection.OpenAsync(ct).ConfigureAwait(false);
            var sessionId = await CurrentSessionIdAsync(connection, ct).ConfigureAwait(false);
            await ExecuteNonQueryAsync(connection, $"USE SCHEMA {quotedSchema}", ct).ConfigureAwait(false);
            await connection.CloseAsync(ct).ConfigureAwait(false);

            Assert.Equal(1, GetOldPoolSize(connectionString));

            using var connection2 = Fixture.Factory.Create(Output, connectionString);
            await connection2.OpenAsync(ct).ConfigureAwait(false);
            Assert.NotEqual(sessionId, await CurrentSessionIdAsync(connection2, ct).ConfigureAwait(false));
            await connection2.CloseAsync(ct).ConfigureAwait(false);
        }
        finally
        {
            await ExecuteOnSeparateSessionAsync($"DROP SCHEMA IF EXISTS {quotedSchema}", ct).ConfigureAwait(false);
        }
    }

    private static async Task ExecuteNonQueryAsync(
        ISnowflakeDbConnection connection,
        string commandText,
        CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = commandText;
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task<string> QueryStringAsync(
        ISnowflakeDbConnection connection,
        string commandText,
        CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = commandText;
        var value = await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false);
        return Assert.IsType<string>(value);
    }

    private static Task<string> CurrentSessionIdAsync(
        ISnowflakeDbConnection connection,
        CancellationToken cancellationToken) =>
        QueryStringAsync(connection, "SELECT CURRENT_SESSION()", cancellationToken);

    private async Task ExecuteOnSeparateSessionAsync(string commandText, CancellationToken cancellationToken)
    {
        using var connection = Fixture.Factory.Create(Output, AppendPoolSettings("poolingEnabled=false"));
        await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, commandText, cancellationToken).ConfigureAwait(false);
        await connection.CloseAsync(cancellationToken).ConfigureAwait(false);
    }

    private string AppendPoolSettings(string settings)
    {
        using var probe = Fixture.Factory.Create(Output);
        return $"{probe.ConnectionString};{settings}";
    }

    private static void ClearAllPoolsOnOldDriver()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.ClearAllPools();
    }

    private static int GetOldPoolSize(string connectionString) =>
        OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize();
}
