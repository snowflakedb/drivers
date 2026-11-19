using System.Collections.Generic;
using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using System.Net;
using System.Net.Http;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[Trait("Category", "Wiremock")]
public sealed class SFConnectionWiremockTest : E2ETestBase
{
    private const string OldDriverOktaRetryCountersAreMocked =
        "The old driver counts Okta retries from response headers that only its test-only MockOktaRestRequester sets; against an HTTP server both counters stay at zero";
    private const string DelayedLoginUser = "wiremock_login_delayed";
    private const int InternalErrorCode = 270001;
    private const int RequestTimeoutErrorCode = 270007;
    private const int DefaultLoginTimeoutSeconds = 300;

    public SFConnectionWiremockTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldTimeOutLoginAfterConnectionTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed Open leaves State as Connecting and ConnectionTimeout does not report the login timeout
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_delayed.json", "auth/login_success_any.json").ConfigureAwait(false);
        var timeoutSec = 5;
        var timeoutSetting = $"{LoginTimeoutKey()}={timeoutSec}";
        using var conn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(wiremock, $"user={DelayedLoginUser}", timeoutSetting, MaxHttpRetriesSetting(0)));

        Assert.Equal(ConnectionState.Closed, conn.State);
        var stopwatch = Stopwatch.StartNew();
        var exception = AssertThrowsSnowflakeDbException(conn.Open);
        stopwatch.Stop();

        AssertHasErrorCodeInExceptionChain(exception, RequestTimeoutErrorCode);
        Assert.InRange(stopwatch.ElapsedMilliseconds, timeoutSec * 1000 - 50, (timeoutSec + 1) * 1000);
        Assert.Equal(ConnectionState.Closed, conn.State);

        using var openedConn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(wiremock, timeoutSetting, MaxHttpRetriesSetting(0)));
        openedConn.Open();
        Assert.Equal(timeoutSec, openedConn.ConnectionTimeout);
    }

    [SnowflakeFact]
    public void ShouldUseDefaultLoginTimeout()
    {
        // TODO(SNOW-4382580): ConnectionTimeout reports the login timeout
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, string.Empty);

        Assert.Equal(DefaultLoginTimeoutSeconds, conn.ConnectionTimeout);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public void ShouldFailFastForNonRetried404OnLogin()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(
            Output,
            $"host=docs.snowflake.com/404;{LoginTimeoutKey()}=0;account=testFailFast;user=testFailFast;password=testFailFast;certRevocationCheckMode=enabled;");

        Assert.Equal(ConnectionState.Closed, conn.State);
        var exception = AssertThrowsSnowflakeDbException(conn.Open);

        AssertHasHttpErrorCodeInExceptionChain(exception, HttpStatusCode.NotFound);
        Assert.Contains(CollectExceptions(exception), e => e.Message.Contains("404 (Not Found)"));
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public void ShouldRetryLoginOn404WhenForceRetryOn404Enabled()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(
            Output,
            $"host=docs.snowflake.com/404;{LoginTimeoutKey()}=0;account=testFailFast;user=testFailFast;password=testFailFast;disableretry=true;forceretryon404=true;certRevocationCheckMode=enabled;");

        Assert.Equal(ConnectionState.Closed, conn.State);
        var exception = AssertThrowsSnowflakeDbException(conn.Open);

        Assert.Equal(InternalErrorCode, exception.ErrorCode);
        AssertHasHttpErrorCodeInExceptionChain(exception, HttpStatusCode.NotFound);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldFailOktaConnectionWhenRetryLimitsReachedAsync()
    {
        Skip.When(TestDriver.Current == TestDriver.Kind.Old, OldDriverOktaRetryCountersAreMocked);
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartHttpsWithMappingsAsync(
            ct, "auth/okta_token_cookie_token.json", "auth/okta_sso_missing_form_action.json").ConfigureAwait(false);
        var oktaBase = $"https://localhost:{wiremock.HttpsPort}";
        await wiremock.AddMappingAsync(
            "auth/authenticator_request_okta.json",
            placeholders: new Dictionary<string, string> { ["{{OKTA_BASE_URL}}"] = oktaBase },
            cancellationToken: ct).ConfigureAwait(false);
        var expectedMaxRetryCount = 15;
        var expectedMaxConnectionTimeout = 450;
        using var conn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(
                wiremock,
                $"authenticator={oktaBase}/okta",
                "user=test",
                "password=test",
                $"custom_root_store_path={wiremock.WiremockCaPemPath}",
                $"retry_max_attempts={expectedMaxRetryCount}",
                $"login_timeout={expectedMaxConnectionTimeout}"));

        var exception = AssertThrowsSnowflakeDbException(conn.Open);

        Assert.Equal(InternalErrorCode, exception.ErrorCode);
        Assert.Contains(
            $"The retry count has reached its limit of {expectedMaxRetryCount} and "
            + $"the timeout elapsed has reached its limit of {expectedMaxConnectionTimeout} "
            + "while trying to authenticate through Okta",
            exception.Message);
    }

    [SnowflakeTheory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task ShouldBeClosedIfConnectionFailsAsync(bool isAsync)
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        using var connection = Fixture.Factory.Create(
            Output,
            $"ACCOUNT=testaccount;user=testuser;password=fake;{LoginTimeoutKey()}=5;{MaxHttpRetriesSetting(0)};");
        if (isAsync)
            await AssertThrowsSnowflakeDbExceptionAsync(
                () => connection.OpenAsync(TestContext.Current.CancellationToken),
                TestContext.Current.CancellationToken).ConfigureAwait(false);
        else
            AssertThrowsSnowflakeDbException(connection.Open);
        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    private static DbException AssertThrowsSnowflakeDbException(Action action) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.Throws<OldSnowflakeDbException>(action)
            : Assert.Throws<NewSnowflakeDbException>(action);

    private static async Task<DbException> AssertThrowsSnowflakeDbExceptionAsync(
        Func<Task> action,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var exception = await Assert.ThrowsAnyAsync<Exception>(action).ConfigureAwait(false);
        return AssertIsSnowflakeDbException(exception);
    }

    private static DbException AssertIsSnowflakeDbException(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? Assert.IsType<OldSnowflakeDbException>(exception)
            : Assert.IsType<NewSnowflakeDbException>(exception);

    private static bool IsSnowflakeDbException(Exception exception) =>
        exception is OldSnowflakeDbException or NewSnowflakeDbException;

    private static void AssertHasHttpErrorCodeInExceptionChain(Exception exception, HttpStatusCode expected) =>
        Assert.Contains(
            CollectExceptions(exception),
            e => e switch
            {
                DbException se when IsSnowflakeDbException(se) => se.ErrorCode == (int)expected,
                HttpRequestException he => he.Message.Contains(((int)expected).ToString(CultureInfo.InvariantCulture)),
                _ => false,
            });

    private static string LoginTimeoutKey() =>
        BehaviorDifference.OldDriverOnly("BD#connection-timeout-key-not-recognized")
            ? "connection_timeout"
            : "login_timeout";

    private static string MaxHttpRetriesSetting(int retries) =>
        BehaviorDifference.OldDriverOnly("BD#max-http-retries-key-not-recognized")
            ? $"maxHttpRetries={retries}"
            : $"retry_max_attempts={retries + 1}";

    private static void AssertHasErrorCodeInExceptionChain(Exception exception, int expectedErrorCode) =>
        Assert.Contains(
            CollectExceptions(exception),
            e => e is DbException se && IsSnowflakeDbException(se) && se.ErrorCode == expectedErrorCode);

    private static List<Exception> CollectExceptions(Exception? exception)
    {
        var collected = new List<Exception>();
        switch (exception)
        {
            case null:
                break;
            case AggregateException aggregate:
                var inner = aggregate.Flatten().InnerExceptions;
                collected.AddRange(inner);
                collected.AddRange(inner
                    .Where(e => e.InnerException != null)
                    .SelectMany(e => CollectExceptions(e.InnerException)));
                break;
            default:
                collected.AddRange(CollectExceptions(exception.InnerException));
                collected.Add(exception);
                break;
        }

        return collected;
    }
}
