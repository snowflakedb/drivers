extern alias OldDriver;
using System.Data.Common;
using System.Diagnostics;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[CollectionDefinition(nameof(SFConnectionAsyncIsolatedTestCollection), DisableParallelization = true)]
public sealed class SFConnectionAsyncIsolatedTestCollection
{
}

[Collection(nameof(SFConnectionAsyncIsolatedTestCollection))]
[Trait("Category", "Wiremock")]
public sealed class SFConnectionAsyncIsolatedTest : E2ETestBase
{
    private const string DelayedLoginUser = "wiremock_login_delayed";
    private const int RequestTimeoutErrorCode = 270007;

    public SFConnectionAsyncIsolatedTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldTimeOutLoginAfterRetryTimeoutWhenLesserThanConnectionTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed OpenAsync leaves State as Connecting
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_delayed.json", "auth/login_success_any.json").ConfigureAwait(false);
        var connectionTimeout = 5;
        var retryTimeout = 1;
        string[] timeoutSettings = [$"{LoginTimeoutKey()}={connectionTimeout}", $"retry_timeout={retryTimeout}", MaxHttpRetriesSetting(0)];
        using var conn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(wiremock, [$"user={DelayedLoginUser}", .. timeoutSettings]));

        Assert.Equal(ConnectionState.Closed, conn.State);

        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        stopwatch.Stop();

        AssertRequestTimedOut(exception);
        Assert.InRange(stopwatch.ElapsedMilliseconds, retryTimeout * 1000 - 10, connectionTimeout * 1000 - 1);
        Assert.Equal(ConnectionState.Closed, conn.State);

        using var openedConn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(wiremock, timeoutSettings));
        await openedConn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(retryTimeout, openedConn.ConnectionTimeout);
    }

    private static string LoginTimeoutKey() =>
        BehaviorDifference.OldDriverOnly("BD#connection-timeout-key-not-recognized")
            ? "connection_timeout"
            : "login_timeout";

    private static string MaxHttpRetriesSetting(int retries) =>
        BehaviorDifference.OldDriverOnly("BD#max-http-retries-key-not-recognized")
            ? $"maxHttpRetries={retries}"
            : $"retry_max_attempts={retries + 1}";

    private static void AssertRequestTimedOut(Exception exception)
    {
        var outer = AssertSnowflakeDbException(exception);
        var aggregate = Assert.IsType<AggregateException>(outer.InnerException);
        Assert.Equal(RequestTimeoutErrorCode, AssertSnowflakeDbException(aggregate.InnerException!).ErrorCode);
    }

    private static DbException AssertSnowflakeDbException(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return Assert.IsType<OldSnowflakeDbException>(exception);
        return Assert.IsType<NewSnowflakeDbException>(exception);
    }
}
