using System.Diagnostics;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[Trait("Category", "Wiremock")]
public sealed class SFDbCommandSlowWiremockTest : E2ETestBase
{
    private const int MaxHttpRetries = 8;
    // Backoff between the MaxHttpRetries + 1 query attempts: 1, 2, 4, 8, 16, 16, 16 and 16 seconds.
    private const int TotalBackoffSeconds = 1 + 2 + 4 + 8 + 16 + 16 + 16 + 16;

    public SFDbCommandSlowWiremockTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldTimeOutWhenMaxRetryIsReachedAsync()
    {
        Skip.OnNewDriver(
            "BD#max-http-retries-key-not-recognized",
            "The new driver ignores maxHttpRetries and reads the limit from retry_max_attempts");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockResettingQueryRequestsAsync(ct).ConfigureAwait(false);
        using var conn = Fixture.Factory.Create(Output, MaxRetryConnectionString(wiremock));
        await conn.OpenAsync(ct).ConfigureAwait(false);

        var stopwatch = Stopwatch.StartNew();
        var thrown = Record.Exception(() => ExecuteScalar(conn, "select 1;"));
        stopwatch.Stop();

        var aggregate = Assert.IsType<AggregateException>(thrown);
        Assert.IsAssignableFrom<TaskCanceledException>(aggregate.InnerException);
        Assert.InRange(stopwatch.ElapsedMilliseconds, TotalBackoffSeconds * 1000L, (TotalBackoffSeconds + 30) * 1000L);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldThrowTaskCanceledWithinRetryBudgetWhenMaxRetryIsReachedAsync()
    {
        Skip.OnNewDriver(
            "BD#max-http-retries-key-not-recognized",
            "The new driver ignores maxHttpRetries and reads the limit from retry_max_attempts");

        const int MillisecondsDifferenceToAccept = 5;
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockResettingQueryRequestsAsync(ct).ConfigureAwait(false);
        using var conn = Fixture.Factory.Create(Output, MaxRetryConnectionString(wiremock));
        await conn.OpenAsync(ct).ConfigureAwait(false);

        var stopwatch = Stopwatch.StartNew();
        var thrown = Record.Exception(() => ExecuteScalar(conn, "select 1;"));
        stopwatch.Stop();

        var aggregate = Assert.IsType<AggregateException>(thrown);
        Assert.IsType<TaskCanceledException>(aggregate.InnerException);
        Assert.InRange(
            stopwatch.ElapsedMilliseconds,
            TotalBackoffSeconds * 1000L - MillisecondsDifferenceToAccept,
            (TotalBackoffSeconds + 20) * 1000L - 1);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldThrowSnowflakeDbExceptionWithinRetryBudgetWhenRetryMaxAttemptsIsReachedAsync()
    {
        Skip.OnOldDriver(
            "BD#max-http-retries-key-not-recognized",
            "The old driver reads the HTTP retry limit from maxHttpRetries");
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockResettingQueryRequestsAsync(ct).ConfigureAwait(false);
        // sf_core waits base * factor before the first retry, so a 500 ms base reproduces the 1 s first backoff.
        using var conn = Fixture.Factory.Create(
            Output,
            E2EWiremock.BuildHttpConnectionString(
                wiremock,
                $"retry_max_attempts={MaxHttpRetries + 1}",
                "retry_backoff_base_ms=500",
                "retry_backoff_factor=2",
                "retry_backoff_cap_ms=16000",
                "retry_backoff_jitter=none",
                "poolingEnabled=false"));
        await conn.OpenAsync(ct).ConfigureAwait(false);

        var stopwatch = Stopwatch.StartNew();
        var thrown = Record.Exception(() => ExecuteScalar(conn, "select 1;"));
        stopwatch.Stop();

        Assert.IsType<NewSnowflakeDbException>(thrown);
        Assert.InRange(stopwatch.ElapsedMilliseconds, TotalBackoffSeconds * 1000L, (TotalBackoffSeconds + 30) * 1000L);
    }

    private static Task<WiremockClient> StartWiremockResettingQueryRequestsAsync(CancellationToken cancellationToken) =>
        E2EWiremock.StartWithMappingsAsync(
            cancellationToken,
            "auth/login_success_any.json",
            "session/query_connection_reset_always.json",
            "session/logout_success.json");

    private static string MaxRetryConnectionString(WiremockClient wiremock) =>
        E2EWiremock.BuildHttpConnectionString(wiremock, $"maxHttpRetries={MaxHttpRetries}", "poolingEnabled=false");

    private static object? ExecuteScalar(ISnowflakeDbConnection conn, string commandText)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        return cmd.ExecuteScalar();
    }
}
