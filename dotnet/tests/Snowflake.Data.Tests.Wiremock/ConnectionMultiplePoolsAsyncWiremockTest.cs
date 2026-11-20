extern alias OldDriver;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbConnectionPool = OldDriver::Snowflake.Data.Client.SnowflakeDbConnectionPool;

namespace Snowflake.Data.Tests.Wiremock;

[Trait("Category", "Wiremock")]
public sealed class ConnectionMultiplePoolsAsyncWiremockTest : E2ETestBase
{
    private const string BeginQueryRegex = @"(?s)(?i).*\bbegin\b.*";
    private const string RollbackQueryRegex = "(?s)(?i).*rollback.*";

    public ConnectionMultiplePoolsAsyncWiremockTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldReleaseConnectionWhenRollbackFailsAsync()
    {
        // TODO(SNOW-4382598): real connection pooling and transactions
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.ClearAllPools();

        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct,
            "auth/login_success_any.json",
            "query/begin_success.json",
            "query/rollback_fails.json",
            "session/logout_success.json").ConfigureAwait(false);
        var connectionString = E2EWiremock.BuildHttpConnectionString(
            wiremock,
            "minPoolSize=0;poolingEnabled=true");
        Assert.Equal(0, OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize());
        using var connection = Fixture.Factory.Create(Output, connectionString);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        using var transaction = connection.BeginTransaction();
        Assert.Equal(1, await E2EWiremock.CountQueryRequestsAsync(wiremock, BeginQueryRegex, ct).ConfigureAwait(false));
        Assert.Equal(1, OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize());

        await connection.CloseAsync(ct).ConfigureAwait(false);

        Assert.Equal(1, await E2EWiremock.CountQueryRequestsAsync(wiremock, RollbackQueryRegex, ct).ConfigureAwait(false));
        Assert.Equal(0, OldSnowflakeDbConnectionPool.GetPool(connectionString).GetCurrentPoolSize());
    }
}
