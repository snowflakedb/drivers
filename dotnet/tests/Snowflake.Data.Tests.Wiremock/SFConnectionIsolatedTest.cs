extern alias OldDriver;
using System.Data.Common;
using System.Diagnostics;
using System.Reflection;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[CollectionDefinition(nameof(SFConnectionIsolatedTestCollection), DisableParallelization = true)]
public sealed class SFConnectionIsolatedTestCollection
{
}

[Collection(nameof(SFConnectionIsolatedTestCollection))]
[Trait("Category", "Wiremock")]
public sealed class SFConnectionIsolatedTest : E2ETestBase, IDisposable
{
    private const string DelayedLoginUser = "wiremock_login_delayed";
    private const int RequestTimeoutErrorCode = 270007;

    private readonly object? _originalOldDriverMinRetryTimeout;

    // The old driver raises any retry_timeout below SFSessionHttpClientProperties.DefaultRetryTimeout
    // (300 s) to that value, so the floor is lowered to 1 s for the duration of each test.
    public SFConnectionIsolatedTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
        if (TestDriver.Current != TestDriver.Kind.Old)
            return;
        var property = GetOldDriverMinRetryTimeoutProperty();
        _originalOldDriverMinRetryTimeout = property.GetValue(null);
        property.SetValue(null, TimeSpan.FromSeconds(1));
    }

    public void Dispose()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            GetOldDriverMinRetryTimeoutProperty().SetValue(null, _originalOldDriverMinRetryTimeout);
    }

    [SnowflakeFact]
    public async Task ShouldTimeOutLoginAfterRetryTimeoutWhenLessThanConnectionTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed Open leaves State as Connecting
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
        var exception = AssertThrowsSnowflakeDbException(conn.Open);
        stopwatch.Stop();

        AssertRequestTimedOutOrWasCancelled(exception);
        Assert.InRange(stopwatch.ElapsedMilliseconds, retryTimeout * 1000 - 50, connectionTimeout * 1000 - 1);
        Assert.Equal(ConnectionState.Closed, conn.State);

        using var openedConn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(wiremock, timeoutSettings));
        openedConn.Open();
        Assert.Equal(retryTimeout, openedConn.ConnectionTimeout);
    }

    private static PropertyInfo GetOldDriverMinRetryTimeoutProperty()
    {
        var propertiesType = typeof(OldSnowflakeDbConnection).Assembly.GetType("Snowflake.Data.Core.SFSessionHttpClientProperties");
        Assert.NotNull(propertiesType);
        var property = propertiesType.GetProperty(
            "DefaultRetryTimeout",
            BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public);
        Assert.NotNull(property);
        return property;
    }

    private static string LoginTimeoutKey() =>
        BehaviorDifference.OldDriverOnly("BD#connection-timeout-key-not-recognized")
            ? "connection_timeout"
            : "login_timeout";

    private static string MaxHttpRetriesSetting(int retries) =>
        BehaviorDifference.OldDriverOnly("BD#max-http-retries-key-not-recognized")
            ? $"maxHttpRetries={retries}"
            : $"retry_max_attempts={retries + 1}";

    private static DbException AssertThrowsSnowflakeDbException(Action action) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.Throws<OldSnowflakeDbException>(action)
            : Assert.Throws<NewSnowflakeDbException>(action);

    private static void AssertRequestTimedOutOrWasCancelled(DbException exception)
    {
        var aggregate = Assert.IsType<AggregateException>(exception.InnerException);
        var cause = aggregate.InnerException;
        Assert.True(
            cause is TaskCanceledException
            || (cause is DbException se && IsSnowflakeDbException(se) && se.ErrorCode == RequestTimeoutErrorCode),
            cause?.ToString());
    }

    private static bool IsSnowflakeDbException(Exception exception) =>
        exception is OldSnowflakeDbException or NewSnowflakeDbException;
}
