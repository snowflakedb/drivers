using System.Data.Common;
using System.Net.Http;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Wiremock;

[Trait("Category", "Wiremock")]
public sealed class DataReaderChunkFetchTest : E2ETestBase
{
    private const string FirstChunkNotFoundQuery = "SELECT_FIRST_CHUNK_NOT_FOUND";
    private const string FirstChunkNotFoundQueryId = "01b2c3d4-0000-0000-0000-000000000001";
    private const string ChunkDelayedQuery = "SELECT_CHUNK_DELAYED";

    public DataReaderChunkFetchTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(false)]
    [InlineData(true)]
    public async Task AChunkFetchFailureIsQueryResultFetchFailedOnEveryLaterReadAsync(bool readAsync)
    {
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockAsync("query/select_first_chunk_not_found.json", ct).ConfigureAwait(false);
        using var connection = await OpenAsync(wiremock, ct).ConfigureAwait(false);
        using var reader = ExecuteReader(connection, FirstChunkNotFoundQuery);
        Func<Task<bool>> read = readAsync ? () => reader.ReadAsync(ct) : () => Task.FromResult(reader.Read());

        Assert.True(await read().ConfigureAwait(false));
        Assert.Equal(1L, reader.GetInt64(0));
        Assert.True(await read().ConfigureAwait(false));
        Assert.Equal(2L, reader.GetInt64(0));

        if (BehaviorDifference.OldDriverOnly("BD#reader-chunk-fetch-failure-error-code"))
        {
            var failure = await Assert.ThrowsAnyAsync<Exception>(read).ConfigureAwait(false);
            if (readAsync)
                Assert.IsType<HttpRequestException>(failure);
            else
                Assert.IsType<HttpRequestException>(Assert.IsType<AggregateException>(failure).InnerException);

            Assert.True(await read().ConfigureAwait(false));
            Assert.Equal(5L, reader.GetInt64(0));
            Assert.True(await read().ConfigureAwait(false));
            Assert.Equal(6L, reader.GetInt64(0));
            Assert.False(await read().ConfigureAwait(false));
            return;
        }

        var failures = new List<Exception?>();
        for (var attempt = 0; attempt < 3; attempt++)
            failures.Add(await Record.ExceptionAsync(read).ConfigureAwait(false));

        Assert.All(failures, AssertMappedAtPublicBoundary);
        Skip.UntilPublicExceptionsExist();
        Assert.All(failures, failure =>
        {
            var exception = Assert.IsType<NewSnowflakeDbException>(failure);
            Assert.Equal(270075, exception.ErrorCode);
            Assert.Equal(FirstChunkNotFoundQueryId, exception.QueryId);
        });
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(false)]
    [InlineData(true)]
    public async Task AReadWhileReadAsyncIsPendingIsDataReaderFetchInProgressAsync(bool overlappingReadAsync)
    {
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockAsync("query/select_chunk_delayed.json", ct).ConfigureAwait(false);
        using var connection = await OpenAsync(wiremock, ct).ConfigureAwait(false);
        using var reader = ExecuteReader(connection, ChunkDelayedQuery);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Func<Task<bool>> overlappingRead = overlappingReadAsync
            ? () => reader.ReadAsync(ct)
            : () => Task.FromResult(reader.Read());

        var pending = reader.ReadAsync(ct);
        Assert.False(pending.IsCompleted);

        var oldDriver = BehaviorDifference.OldDriverOnly("BD#reader-concurrent-fetch-throws-fetch-in-progress");
        bool? overlappingResult = null;
        Exception? overlappingFailure = null;
        if (oldDriver)
            overlappingResult = await overlappingRead().ConfigureAwait(false);
        else
            overlappingFailure = await Record.ExceptionAsync(overlappingRead).ConfigureAwait(false);
        Assert.False(pending.IsCompleted);

        Assert.True(await pending.ConfigureAwait(false));
        Assert.Equal(3L, reader.GetInt64(0));
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(4L, reader.GetInt64(0));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false));

        if (oldDriver)
        {
            Assert.False(overlappingResult);
            return;
        }

        AssertMappedAtPublicBoundary(overlappingFailure);
        Skip.UntilPublicExceptionsExist();
        Assert.Equal(270074, Assert.IsType<NewSnowflakeDbException>(overlappingFailure).ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CloseWhileReadAsyncIsPendingReturnsAtOnceAndFailsThePendingReadAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockAsync("query/select_chunk_delayed.json", ct).ConfigureAwait(false);
        using var connection = await OpenAsync(wiremock, ct).ConfigureAwait(false);
        using var reader = ExecuteReader(connection, ChunkDelayedQuery);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));

        var pending = reader.ReadAsync(ct);
        reader.Close();

        Assert.True(reader.IsClosed);
        Assert.False(pending.IsCompleted);
        if (BehaviorDifference.OldDriverOnly("BD#reader-close-during-pending-read-fails-the-read"))
        {
            Assert.True(await pending.ConfigureAwait(false));
            return;
        }

        var failure = await Record.ExceptionAsync(() => pending).ConfigureAwait(false);
        AssertMappedAtPublicBoundary(failure);
        Skip.UntilPublicExceptionsExist();
        Assert.Equal(270010, Assert.IsType<NewSnowflakeDbException>(failure).ErrorCode);
    }

    private static Task<WiremockClient> StartWiremockAsync(string queryMapping, CancellationToken cancellationToken) =>
        E2EWiremock.StartWithMappingsAsync(
            cancellationToken,
            "auth/login_success_any.json",
            "session/logout_success.json",
            queryMapping);

    // TODO(SNOW-4382591): every SnowflakeDbException constructor throws NotImplementedException, so the public boundary surfaces each carrier as that exception.
    private static void AssertMappedAtPublicBoundary(Exception? failure) =>
        Assert.IsType<NotImplementedException>(failure);

    private static DbDataReader ExecuteReader(ISnowflakeDbConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        return command.ExecuteReader();
    }

    private async Task<ISnowflakeDbConnection> OpenAsync(WiremockClient wiremock, CancellationToken cancellationToken)
    {
        var connection = Fixture.Factory.Create(Output, E2EWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));
        try
        {
            await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
            return connection;
        }
        catch
        {
            connection.Dispose();
            throw;
        }
    }
}
