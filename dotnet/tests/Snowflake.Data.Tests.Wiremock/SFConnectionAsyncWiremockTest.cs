extern alias OldDriver;
using System.Collections.Generic;
using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using System.Net;
using System.Net.Http;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[Trait("Category", "Wiremock")]
public sealed class SFConnectionAsyncWiremockTest : E2ETestBase
{
    private const string ConnectionFailureSqlState = "08006";
    private const int InternalErrorCode = 270001;
    private const int UnknownAuthenticatorErrorCode = 270011;
    private const int QueryCancelledErrorCode = 270005;
    private const string DelayedLoginUser = "wiremock_login_delayed";
    private const string UnavailableLoginUser = "wiremock_login_503_always";
    private const string OldDriverOktaRetryCountersAreMocked =
        "The old driver counts Okta retries from response headers that only its test-only MockOktaRestRequester sets; against an HTTP server both counters stay at zero";
    private const string OldDriverRejectsHostWithPathOnLinux =
        "On Linux the old driver fails to build the login URI for a host containing a path segment, so the failure is an invalid-URI internal error rather than the HTTP 404";

    public SFConnectionAsyncWiremockTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldSendApplicationPathDuringAuthenticationAsync()
    {
        Skip.For(SkipCondition.RunOnlyOnCI, "Test runs only on CI.");
        Skip.OnNewDriver("BD#application-path-not-sent", "The .NET wrapper identity does not set application_path");

        var applicationPath = await ReadLoginApplicationPathAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var lowerPath = applicationPath.ToLowerInvariant();
        Assert.True(
            lowerPath.Contains("snowflake.data.tests")
            && lowerPath.Contains("bin")
            && (lowerPath.EndsWith(".dll", StringComparison.Ordinal) || lowerPath.EndsWith(".exe", StringComparison.Ordinal)),
            $"APPLICATION_PATH should contain 'snowflake.data.tests' and 'bin' and end with .dll or .exe. Got: {applicationPath}");
    }

    [SnowflakeFact]
    public async Task ShouldSendUnknownApplicationPathDuringAuthenticationAsync()
    {
        Skip.OnOldDriver("BD#application-path-not-sent", "The old driver sends the path of the running executable");

        var applicationPath = await ReadLoginApplicationPathAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal("UNKNOWN", applicationPath);
    }

    [SnowflakeFact]
    public async Task ShouldFailLoginWhenMaxRetryReachedAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(ct, "auth/login_503_always.json").ConfigureAwait(false);
        using var conn = CreateConnection(
            E2EWiremock.BuildHttpConnectionString(wiremock, $"user={UnavailableLoginUser}", MaxHttpRetriesSetting(7)));

        Assert.Equal(ConnectionState.Closed, conn.State);
        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        stopwatch.Stop();

        AssertSnowflakeDbException(exception);
        // Seven retries start with a 1 s backoff that can randomly drop to 0, so the total backoff
        // ranges from 1 + 0 + 0 + 0 + 0 + 0 + 0 = 1 s to 1 + 2 + 5 + 10 + 21 + 42 + 85 = 166 s.
        Assert.InRange(stopwatch.ElapsedMilliseconds, 1 * 1000, 166 * 1000);
    }

    [SnowflakeFact]
    public async Task ShouldRejectInvalidConnectionStringAsync()
    {
        Skip.When(true, "TODO Bug: SNOW-3552882");

        string[] invalidStrings =
        [
            "ACCOUNT=testaccount;user=testuser",
            "ACCOUNT=A=C;USER=testuser;password=123;key",
            "complete_invalid_string",
        ];
        int[] expectedErrorCode = [270006, 270008, 270008];

        using var conn = CreateConnection(string.Empty);
        for (var i = 0; i < invalidStrings.Length; i++)
        {
            conn.ConnectionString = invalidStrings[i];
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Equal(expectedErrorCode[i], AssertSnowflakeDbException(exception).ErrorCode);
        }
    }

    [SnowflakeFact]
    public async Task ShouldRejectUnknownAuthenticatorAsync()
    {
        // TODO(SNOW-4382591): unknown authenticators surface as ServiceException, not SnowflakeDbException
        Skip.FutureMilestone();

        string[] wrongAuthenticators =
        [
            "http://snowflakecomputing.okta.com",
            "https://snowflake.com",
            "unknown",
        ];

        foreach (var wrongAuthenticator in wrongAuthenticators)
        {
            using var conn = CreateConnection(
                "scheme=http;host=snowflake.com;port=8080;user=test;password=test;account=test;authenticator=" + wrongAuthenticator);
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Equal(UnknownAuthenticatorErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        }
    }

    [SnowflakeFact]
    public async Task ShouldRetryOktaConnectionUntilMaxTimeoutAsync()
    {
        Skip.When(TestDriver.Current == TestDriver.Kind.Old, OldDriverOktaRetryCountersAreMocked);
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var expectedMaxRetryCount = 15;
        var exception = await OpenWithOktaSamlMissingPostbackAsync(
            expectedMaxRetryCount, 450, TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        var messages = string.Join("\n", CollectExceptions(exception).Select(e => e.Message));
        Assert.Contains($"The retry count has reached its limit of {expectedMaxRetryCount}", messages);
        Assert.Contains("while trying to authenticate through Okta", messages);
    }

    [SnowflakeFact]
    public async Task ShouldCancelLoginBeforeTimeoutAsync()
    {
        // TODO(SNOW-4382580): a cancelled OpenAsync throws ServiceException instead of TaskCanceledException and leaves State as Connecting
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(ct, "auth/login_delayed.json").ConfigureAwait(false);
        var timeoutSec = 0;
        using var conn = CreateConnection(
            E2EWiremock.BuildHttpConnectionString(
                wiremock, $"user={DelayedLoginUser}", $"{LoginTimeoutKey()}={timeoutSec}", MaxHttpRetriesSetting(0)));

        Assert.Equal(ConnectionState.Closed, conn.State);

        using var connectionCancelToken = CancellationTokenSource.CreateLinkedTokenSource(ct);
        var openTask = conn.OpenAsync(connectionCancelToken.Token);

        Assert.Equal(ConnectionState.Connecting, conn.State);

        connectionCancelToken.Cancel();
        await Assert.ThrowsAsync<TaskCanceledException>(() => openTask).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldTimeOutLoginAfterConnectionTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed OpenAsync leaves State as Connecting and ConnectionTimeout does not report the login timeout
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_delayed.json", "auth/login_success_any.json").ConfigureAwait(false);
        var timeoutSec = 5;
        var timeoutSetting = $"{LoginTimeoutKey()}={timeoutSec}";
        using var conn = CreateConnection(
            E2EWiremock.BuildHttpConnectionString(wiremock, $"user={DelayedLoginUser}", timeoutSetting, MaxHttpRetriesSetting(0)));

        Assert.Equal(ConnectionState.Closed, conn.State);

        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        stopwatch.Stop();

        AssertHasErrorCodeInExceptionChain(exception, InternalErrorCode);
        // The retry count is infinite, so the login times out after the connection timeout.
        Assert.InRange(stopwatch.ElapsedMilliseconds, timeoutSec * 1000 - 50, long.MaxValue);
        Assert.Equal(ConnectionState.Closed, conn.State);

        using var openedConn = CreateConnection(
            E2EWiremock.BuildHttpConnectionString(wiremock, timeoutSetting, MaxHttpRetriesSetting(0)));
        await openedConn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(timeoutSec, openedConn.ConnectionTimeout);
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelWithinCleanupWaitWhenPostHangsAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "cleanup_wait has no effect");

        const int CleanupWaitSeconds = 2;
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, $"cleanup_wait={CleanupWaitSeconds}", ct).ConfigureAwait(false);

        Assert.Contains(await E2EWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false), IsAbortRequest);
        // The bound allows 1 s for the external token, 2 s of cleanup_wait and a generous tolerance.
        Assert.True(
            elapsed.TotalSeconds < CleanupWaitSeconds + 30,
            $"Cancel() should return within cleanup_wait ({CleanupWaitSeconds}s) but took {elapsed.TotalSeconds:F1}s");
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelImmediatelyWhenCleanupWaitIsZeroAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "cleanup_wait has no effect");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, "cleanup_wait=0", ct).ConfigureAwait(false);

        Assert.True(
            elapsed.TotalSeconds < 5,
            $"Cancel() with cleanup_wait=0 should return immediately but took {elapsed.TotalSeconds:F1}s");
    }

    [SnowflakeFact]
    public async Task ShouldApplyAbortRequestTimeoutToCancelRequestAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "abort_request_timeout has no effect");

        const int AbortTimeoutSeconds = 3;
        const int CleanupWaitSeconds = 5;
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(
            wiremock, $"abort_request_timeout={AbortTimeoutSeconds};cleanup_wait={CleanupWaitSeconds}", ct).ConfigureAwait(false);

        Assert.Contains(await E2EWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false), IsAbortRequest);
        // The token fires after 1 s and the abort request hangs, so the cancel returns when abort_request_timeout
        // expires (about 1 + AbortTimeoutSeconds); cleanup_wait alone would return after 1 + CleanupWaitSeconds.
        Assert.InRange(elapsed.TotalSeconds, AbortTimeoutSeconds, CleanupWaitSeconds);
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelAfterFixedCleanupWaitWhenAbortHangsAsync()
    {
        Skip.OnOldDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "The old driver honors cleanup_wait");
        // TODO(SNOW-4382591): SnowflakeDbException constructors throw NotImplementedException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, "cleanup_wait=0", ct).ConfigureAwait(false);

        Assert.InRange(elapsed.TotalSeconds, 1 + 5, 1 + 30);
    }

    [SnowflakeFact]
    public async Task ShouldFailFastForNonRetried404OnLoginAsync()
    {
        // TODO(SNOW-4382591): HTTP login failures surface as ServiceException, not SnowflakeDbException
        Skip.FutureMilestone();
        Skip.When(TestDriver.Current == TestDriver.Kind.Old && RuntimeInformation.IsOSPlatform(OSPlatform.Linux), OldDriverRejectsHostWithPathOnLinux);

        using var conn = CreateConnection(
            $"host=docs.snowflake.com/404;{LoginTimeoutKey()}=0;account=testFailFast;user=testFailFast;password=testFailFast;certRevocationCheckMode=enabled;");
        Assert.Equal(ConnectionState.Closed, conn.State);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        AssertSnowflakeDbException(exception);
        var chain = CollectExceptions(exception);
        Assert.Contains(chain, e => e switch
        {
            DbException se when IsDriverSnowflakeDbException(se) => se.ErrorCode == (int)HttpStatusCode.NotFound,
            HttpRequestException he => he.Message.Contains(((int)HttpStatusCode.NotFound).ToString(CultureInfo.InvariantCulture)),
            _ => false,
        });
        Assert.Contains(chain, e => e.Message.Contains("404 (Not Found)"));
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldBreakConnectionWhenCloseFailsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed Close leaves State as Open instead of Broken
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_400_always.json").ConfigureAwait(false);
        using var conn = CreateConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        var exception = await Assert.ThrowsAnyAsync<Exception>(() => conn.CloseAsync(ct)).ConfigureAwait(false);
        AssertIsLogoutFailure(exception);
        Assert.Equal(ConnectionState.Broken, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldReportOktaRetryLimitInExceptionChainAsync()
    {
        Skip.When(TestDriver.Current == TestDriver.Kind.Old, OldDriverOktaRetryCountersAreMocked);
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var expectedMaxRetryCount = 15;
        var exception = await OpenWithOktaSamlMissingPostbackAsync(
            expectedMaxRetryCount, 450, TestContext.Current.CancellationToken).ConfigureAwait(false);

        AssertHasErrorCodeInExceptionChain(exception, InternalErrorCode);
        var chain = CollectExceptions(exception);
        Assert.Contains(chain, e => e.Message.Contains($"The retry count has reached its limit of {expectedMaxRetryCount}"));
        Assert.Contains(chain, e => e.Message.Contains("while trying to authenticate through Okta"));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData("connection_timeout=5;", false)]
    [InlineData("", true)]
    public async Task ShouldThrowWhenOpenAsyncConnectsToUnreachableHostAsync(string extraParameters, bool expectCancellation)
    {
        // TODO(SNOW-4382580): proxy options are ignored and a failed OpenAsync leaves State as Connecting
        Skip.FutureMilestone();

        using var connection = CreateConnection(
            "account=testAccount;user=testUser;password=testPassword;useProxy=true;proxyHost=no.such.pro.xy;proxyPort=8080;certRevocationCheckMode=enabled;"
            + extraParameters);

        if (expectCancellation)
        {
            await Assert.ThrowsAsync<TaskCanceledException>(() => connection.OpenAsync(CancellationToken.None)).ConfigureAwait(false);
        }
        else
        {
            var thrown = await Assert.ThrowsAnyAsync<DbException>(() => connection.OpenAsync(CancellationToken.None)).ConfigureAwait(false);

            Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(thrown).ErrorCode);
            AssertIsConnectionFailure(thrown);
        }

        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenOpenAsyncIsCancelledAsync()
    {
        // TODO(SNOW-4382580): proxy options are ignored, so the login fails before the cancellation fires
        Skip.FutureMilestone();

        using var connection = CreateConnection(
            "account=testAccount;user=testUser;password=testPassword;useProxy=true;proxyHost=no.such.pro.xy;proxyPort=8080;certRevocationCheckMode=enabled;");
        using var shortCancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));

        await Assert.ThrowsAsync<TaskCanceledException>(() => connection.OpenAsync(shortCancellation.Token)).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task ShouldNotBlockOnHangingCloseAsync()
    {
        Skip.OnNewDriver("BD#session-not-closed-on-gc", "A finalized connection does not send a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_delayed.json").ConfigureAwait(false);
        OpenAndAbandonConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));

        var watchClose = Stopwatch.StartNew();
        GC.Collect();
        GC.WaitForPendingFinalizers();
        watchClose.Stop();
        var logoutRequests = await WaitForLogoutRequestsAsync(wiremock, TimeSpan.FromSeconds(15), ct).ConfigureAwait(false);

        Assert.Single(logoutRequests);
        Assert.True(watchClose.Elapsed.Duration() < TimeSpan.FromSeconds(5));
    }

    [SnowflakeFact]
    public async Task ShouldStayOpenWhenCloseIsCancelledAsync()
    {
        // TODO(SNOW-4382580): CloseAsync does not observe cancellation once the logout request is in flight
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_delayed.json").ConfigureAwait(false);
        using var conn = CreateConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var closeCancellation = CancellationTokenSource.CreateLinkedTokenSource(ct);
        closeCancellation.CancelAfter(TimeSpan.FromSeconds(1));

        await Assert.ThrowsAsync<TaskCanceledException>(() => conn.CloseAsync(closeCancellation.Token)).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldCancelQueryQuicklyWithDefaultCleanupSettingsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException constructors throw NotImplementedException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct,
            "auth/login_success_any.json",
            "query/query_hanging.json",
            "query/abort_request_success.json",
            "session/logout_success.json").ConfigureAwait(false);
        using var conn = CreateConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));

        await conn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT SLOW_QUERY()";
        cmd.CommandTimeout = 0;

        cts.CancelAfter(TimeSpan.FromSeconds(2));

        var stopwatch = Stopwatch.StartNew();
        var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
            () => cmd.ExecuteScalarAsync(cts.Token)).ConfigureAwait(false);

        Assert.Equal(QueryCancelledErrorCode, AssertSnowflakeDbException(thrown.InnerException!).ErrorCode);
        Assert.True(
            stopwatch.Elapsed.TotalSeconds < 30,
            $"Default cancel should complete quickly. Elapsed: {stopwatch.Elapsed.TotalSeconds:F2}s");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldCloseSessionWhenGarbageCollectorFinalizesConnectionAsync()
    {
        Skip.OnNewDriver("BD#session-not-closed-on-gc", "A finalized connection does not send a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        OpenAndAbandonConnection(
            E2EWiremock.BuildHttpConnectionString(
                wiremock, "poolingEnabled=false", "application=TestGarbageCollectorCloseSession"));

        GC.Collect();
        var logoutRequests = await WaitForLogoutRequestsAsync(wiremock, TimeSpan.FromSeconds(30), ct).ConfigureAwait(false);

        Assert.Single(logoutRequests);
    }

    [SnowflakeFact]
    public async Task ShouldNotCloseSessionWhenGarbageCollectorCollectsConnectionAsync()
    {
        Skip.OnOldDriver("BD#session-not-closed-on-gc", "The old driver's finalizer sends a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        OpenAndAbandonConnection(
            E2EWiremock.BuildHttpConnectionString(
                wiremock, "poolingEnabled=false", "application=TestGarbageCollectorCloseSession"));

        GC.Collect();
        GC.WaitForPendingFinalizers();
        await Task.Delay(TimeSpan.FromSeconds(2), ct).ConfigureAwait(false);

        Assert.DoesNotContain(
            await E2EWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false),
            IsLogoutRequest);
    }

    private ISnowflakeDbConnection CreateConnection(string connectionString) =>
        Fixture.Factory.Create(Output, connectionString);

    private static bool IsDriverSnowflakeDbException(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? exception is OldSnowflakeDbException
            : exception is NewSnowflakeDbException;

    private static DbException AssertSnowflakeDbException(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return Assert.IsType<OldSnowflakeDbException>(exception);
        return Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static void AssertIsConnectionFailure(Exception exception)
    {
        var snowflakeException = AssertSnowflakeDbException(exception);
        var sqlState = snowflakeException is OldSnowflakeDbException old
            ? old.SqlState
            : ((NewSnowflakeDbException)snowflakeException).SqlState;
        Assert.Equal(ConnectionFailureSqlState, sqlState);
    }

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
            e => e is DbException se && IsDriverSnowflakeDbException(se) && se.ErrorCode == expectedErrorCode);

    private static void AssertIsLogoutFailure(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            Assert.IsType<HttpRequestException>(exception);
        else
            Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static bool IsAbortRequest(RecordedRequest request) =>
        request.Method == "POST" && request.Url.StartsWith("/queries/v1/abort-request", StringComparison.Ordinal);

    private static bool IsLogoutRequest(RecordedRequest request) =>
        request.Method == "POST" && request.Url.StartsWith("/session?delete=true", StringComparison.Ordinal);

    private static async Task<IReadOnlyList<RecordedRequest>> WaitForLogoutRequestsAsync(
        WiremockClient wiremock,
        TimeSpan timeout,
        CancellationToken cancellationToken)
    {
        var stopwatch = Stopwatch.StartNew();
        while (true)
        {
            var logoutRequests = (await E2EWiremock.GetRecordedRequestsAsync(wiremock, cancellationToken).ConfigureAwait(false))
                .Where(IsLogoutRequest)
                .ToList();
            if (logoutRequests.Count > 0 || stopwatch.Elapsed > timeout)
                return logoutRequests;
            await Task.Delay(TimeSpan.FromMilliseconds(200), cancellationToken).ConfigureAwait(false);
        }
    }

    private static Task<WiremockClient> StartWiremockWithHangingAbortAsync(CancellationToken cancellationToken) =>
        E2EWiremock.StartWithMappingsAsync(
            cancellationToken,
            "auth/login_success_any.json",
            "query/query_hanging.json",
            "query/abort_request_hanging.json",
            "session/logout_success.json");

    private async Task<TimeSpan> CancelHangingQueryAsync(
        WiremockClient wiremock,
        string cancelSettings,
        CancellationToken cancellationToken)
    {
        using var conn = CreateConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false", cancelSettings));
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT LONG_RUNNING_QUERY()";
        cmd.CommandTimeout = 0;

        cts.CancelAfter(TimeSpan.FromSeconds(1));

        var stopwatch = Stopwatch.StartNew();
        await Assert.ThrowsAsync<OperationCanceledException>(() => cmd.ExecuteScalarAsync(cts.Token)).ConfigureAwait(false);
        stopwatch.Stop();
        return stopwatch.Elapsed;
    }

    private async Task<string> ReadLoginApplicationPathAsync(CancellationToken cancellationToken)
    {
        using var wiremock = await E2EWiremock.StartWithMappingsAsync(
            cancellationToken, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        using (var conn = CreateConnection(E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false")))
        {
            await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
            await conn.CloseAsync(cancellationToken).ConfigureAwait(false);
        }

        var requests = await E2EWiremock.GetRecordedRequestsAsync(wiremock, cancellationToken).ConfigureAwait(false);
        var login = Assert.Single(requests, r => r.Url.StartsWith("/session/v1/login-request", StringComparison.Ordinal));
        using var body = JsonDocument.Parse(login.Body);
        return body.RootElement.GetProperty("data").GetProperty("CLIENT_ENVIRONMENT").GetProperty("APPLICATION_PATH").GetString()!;
    }

    private async Task<Exception> OpenWithOktaSamlMissingPostbackAsync(
        int maxRetryCount,
        int maxRetryTimeoutSeconds,
        CancellationToken cancellationToken)
    {
        using var wiremock = await E2EWiremock.StartHttpsWithMappingsAsync(
            cancellationToken, "auth/okta_token_cookie_token.json", "auth/okta_sso_missing_form_action.json").ConfigureAwait(false);
        var oktaBase = $"https://localhost:{wiremock.HttpsPort}";
        await wiremock.AddMappingAsync(
            "auth/authenticator_request_okta.json",
            placeholders: new Dictionary<string, string> { ["{{OKTA_BASE_URL}}"] = oktaBase },
            cancellationToken: cancellationToken).ConfigureAwait(false);
        using var conn = CreateConnection(
            E2EWiremock.BuildHttpConnectionString(
                wiremock,
                $"authenticator={oktaBase}/okta",
                "user=test",
                "password=test",
                $"custom_root_store_path={wiremock.WiremockCaPemPath}",
                $"retry_max_attempts={maxRetryCount}",
                $"login_timeout={maxRetryTimeoutSeconds}"));

        return await Assert.ThrowsAnyAsync<Exception>(() => conn.OpenAsync(cancellationToken)).ConfigureAwait(false);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private void OpenAndAbandonConnection(string connectionString)
    {
        var conn = CreateConnection(connectionString);
        conn.Open();
        Assert.Equal(ConnectionState.Open, conn.State);
    }

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
                foreach (var e in inner)
                    collected.AddRange(CollectExceptions(e.InnerException));
                break;
            default:
                collected.AddRange(CollectExceptions(exception.InnerException));
                collected.Add(exception);
                break;
        }

        return collected;
    }
}
