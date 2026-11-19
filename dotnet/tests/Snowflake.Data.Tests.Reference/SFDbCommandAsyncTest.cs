using System.Collections.Concurrent;
using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewActivityStarter = Snowflake.Data.SnowflakeActivityStarter;
using NewSnowflakeDbCommand = Snowflake.Data.SnowflakeDbCommand;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldActivityStarter = Snowflake.Data.Telemetry.ActivityStarter;
using OldSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbDataReader = Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFDbCommandAsyncTest : ReferenceTestBase
{
    private const int ObjectNotFoundErrorCode = 2003;
    private const int ServerQueryCancelledErrorCode = 604;
    private const int QueryCancelledErrorCode = 270005;
    private const int RequestTimeoutErrorCode = 270007;
    private const int UnsupportedFeatureErrorCode = 270009;
    private const string InvalidQueryIdMessage = "Invalid query id format. Expected a UUID.";
    private const string NoDataMaxRetryMessage = "Max retry for no data is reached";
    private const string PutGetInAsyncModeMessage = "Get and Put are not supported in async execution mode";
    private const string InternalActivitySourceName = "Snowflake_dotnet_activity";
    private const string ClientDefinedActivitySourceName = "Client_custom_activity";
    private const string SessionIdTag = "snowflake.session.id";
    private const string StatusCodeTag = "status.code";
    private const string ExceptionTypeTag = "exception.type";
    private const string DbSystemTag = "db.system";
    private const string DbNameTag = "db.namespace";
    private const string DbWarehouseTag = "snowflake.warehouse";
    private const string DbRoleTag = "snowflake.role";

    public SFDbCommandAsyncTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldCancelExecuteAsyncOnExternalTokenAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var externalCancel = new CancellationTokenSource(TimeSpan.FromSeconds(8));
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 20)) v";
        cmd.CommandTimeout = 10;
        var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
            () => cmd.ExecuteScalarAsync(externalCancel.Token)).ConfigureAwait(false);

        var detail = AssertSnowflakeDbException(thrown.InnerException);
        Assert.Equal(QueryCancelledErrorCode, detail.ErrorCode);
        Assert.True(externalCancel.IsCancellationRequested);
        await Task.Delay(2000, TestContext.Current.CancellationToken).ConfigureAwait(false);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowRequestTimeoutOnCommandTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 60)) v";
        cmd.CommandTimeout = 10;

        var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
            () => cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        var detail = AssertSnowflakeDbException(thrown.InnerException);
        Assert.Equal(RequestTimeoutErrorCode, detail.ErrorCode);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteInAsyncModeAsync()
    {
        // TODO(SNOW-4382584): ExecuteInAsyncMode and ExecuteAsyncInAsyncMode block until the query finishes
        Skip.FutureMilestone();

        const int ExpectedWaitTime = 5;
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            cmd.CommandText = $"CALL SYSTEM$WAIT({ExpectedWaitTime}, 'SECONDS');";

            var queryId = await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
            var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(conn.IsStillRunning(queryStatus), $"Expected query to still be running but status was: {queryStatus}");
            Assert.False(conn.IsAnError(queryStatus), $"Expected query to not be an error but status was: {queryStatus}");

            using var reader = GetResultsFromQueryId(cmd, queryId);
            queryStatus = GetQueryStatus(cmd, queryId);

            Assert.True(reader.Read());
            Assert.Equal($"waited {ExpectedWaitTime} seconds", reader.GetString(0));
            Assert.Equal(QueryStatus.Success, queryStatus);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Once)]
    public async Task ShouldExecuteNormalQueryWhileAsyncExecQueryIsRunningAsync()
    {
        // TODO(SNOW-4382584): ExecuteInAsyncMode and ExecuteAsyncInAsyncMode block until the query finishes
        Skip.FutureMilestone();

        const int ExpectedWaitTime = 5;
        var connections = new ISnowflakeDbConnection[3];
        try
        {
            for (var i = 0; i < connections.Length; i++)
            {
                connections[i] = Fixture.Factory.Create(Output, NonPooledConnectionString());
                await connections[i].OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }

            string queryId;
            using (var cmd = connections[0].CreateCommand())
            {
                cmd.CommandText = $"CALL SYSTEM$WAIT({ExpectedWaitTime}, 'SECONDS');";

                queryId = await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
                var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);

                Assert.True(connections[0].IsStillRunning(queryStatus), $"Expected query to still be running but status was: {queryStatus}");
            }

            using (var cmd = connections[1].CreateCommand())
            {
                cmd.CommandText = "select 1;";

                var row = cmd.ExecuteScalar();

                Assert.Equal(1L, row);
            }

            using (var cmd = connections[2].CreateCommand())
            {
                using var reader = await GetResultsFromQueryIdAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);
                var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);

                Assert.True(reader.Read());
                Assert.Equal($"waited {ExpectedWaitTime} seconds", reader.GetString(0));
                Assert.Equal(QueryStatus.Success, queryStatus);
            }

            foreach (var connection in connections)
                await connection.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
        finally
        {
            foreach (var connection in connections)
                connection?.Dispose();
        }
    }

    [SnowflakeFact]
    public async Task ShouldCancelWhileGettingAsyncExecResultsAsync()
    {
        // TODO(SNOW-4382584): ExecuteInAsyncMode and ExecuteAsyncInAsyncMode block until the query finishes
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            using var cancelToken = new CancellationTokenSource();
            cmd.CommandText = "CALL SYSTEM$WAIT(60, 'SECONDS');";

            var queryId = await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
            var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(conn.IsStillRunning(queryStatus), $"Expected query to still be running but status was: {queryStatus}");

            cancelToken.Cancel();
            var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
                () => GetResultsFromQueryIdAsync(cmd, queryId, cancelToken.Token)).ConfigureAwait(false);

            Assert.Contains("The operation was canceled", thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldAbortQueryOnServerWhenGettingAsyncExecResultsIsCancelledAsync()
    {
        Skip.OnNewDriver(
            "BD#get-results-from-query-id-wait-stop",
            "Cancelling GetResultsFromQueryIdAsync leaves the query running on the server");

        string queryId;
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            using var cancelToken = new CancellationTokenSource();
            cmd.CommandText = "CALL SYSTEM$WAIT(60, 'SECONDS');";

            queryId = await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
            var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(conn.IsStillRunning(queryStatus), $"Expected query to still be running but status was: {queryStatus}");

            cancelToken.CancelAfter(TimeSpan.FromSeconds(3));
            await Assert.ThrowsAnyAsync<OperationCanceledException>(
                () => GetResultsFromQueryIdAsync(cmd, queryId, cancelToken.Token)).ConfigureAwait(false);
        }

        using (var checkCmd = conn.CreateCommand())
        {
            const int MaxRetries = 30;
            var retryCount = 0;
            QueryStatus queryStatus;
            do
            {
                await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken).ConfigureAwait(false);
                queryStatus = await GetQueryStatusAsync(checkCmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);
                retryCount++;
            }
            while (retryCount < MaxRetries && (conn.IsStillRunning(queryStatus) || queryStatus == QueryStatus.Aborting));

            Assert.Contains(queryStatus, new[] { QueryStatus.FailedWithError, QueryStatus.Aborted });
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowErrorForFailedAsyncExecQueryAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            const int StatusMaxRetryCount = 5;
            var statusRetryCount = 0;
            cmd.CommandText = "SELECT * FROM FAKE_TABLE;";

            var queryId = await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
            var queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);
            while (statusRetryCount < StatusMaxRetryCount && conn.IsStillRunning(queryStatus))
            {
                await Task.Delay(1000, TestContext.Current.CancellationToken).ConfigureAwait(false);
                queryStatus = await GetQueryStatusAsync(cmd, queryId, TestContext.Current.CancellationToken).ConfigureAwait(false);
                statusRetryCount++;
            }

            Assert.Equal(QueryStatus.FailedWithError, queryStatus);

            var thrown = AssertSnowflakeDbException(
                await Record.ExceptionAsync(() => GetResultsFromQueryIdAsync(cmd, queryId, TestContext.Current.CancellationToken)).ConfigureAwait(false));

            Assert.Contains("'FAKE_TABLE' does not exist", thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenGettingStatusOfInvalidQueryIdAsync()
    {
        Skip.OnNewDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The new driver throws SnowflakeDbException instead of a plain Exception for a malformed query id");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = await Assert.ThrowsAsync<Exception>(
                () => GetQueryStatusAsync(cmd, "fakeQueryId", TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Contains(InvalidQueryIdMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowSnowflakeDbExceptionWhenGettingStatusOfInvalidQueryIdAsync()
    {
        Skip.OnOldDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The old driver throws a plain Exception for a malformed query id");
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = AssertSnowflakeDbException(await Record.ExceptionAsync(
                () => GetQueryStatusAsync(cmd, "fakeQueryId", TestContext.Current.CancellationToken)).ConfigureAwait(false));

            Assert.Contains(InvalidQueryIdMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenGettingResultsOfInvalidQueryIdAsync()
    {
        Skip.OnNewDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The new driver throws SnowflakeDbException instead of a plain Exception for a malformed query id");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = await Assert.ThrowsAsync<Exception>(
                () => GetResultsFromQueryIdAsync(cmd, "fakeQueryId", TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Contains(InvalidQueryIdMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowSnowflakeDbExceptionWhenGettingResultsOfInvalidQueryIdAsync()
    {
        Skip.OnOldDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The old driver throws a plain Exception for a malformed query id");
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = AssertSnowflakeDbException(await Record.ExceptionAsync(
                () => GetResultsFromQueryIdAsync(cmd, "fakeQueryId", TestContext.Current.CancellationToken)).ConfigureAwait(false));

            Assert.Contains(InvalidQueryIdMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnNoDataStatusForUnknownQueryIdAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var queryStatus = await GetQueryStatusAsync(cmd, "ba321edc-1abc-123e-987f-1234a56b789c", TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.Equal(QueryStatus.NoData, queryStatus);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenGettingResultsOfUnknownQueryIdWithDefaultRetryAsync()
    {
        Skip.When(true, "The test takes too long to finish when using the default retry");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = await Assert.ThrowsAsync<Exception>(
                () => GetResultsFromQueryIdAsync(cmd, "ab123fed-1abc-987f-987f-1234a56b789c", TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Contains(NoDataMaxRetryMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteSimpleCommandAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select 1";

        Assert.Equal(CommandType.Text, cmd.CommandType);
        var thrown = AssertSnowflakeDbException(Record.Exception(() => cmd.CommandType = CommandType.StoredProcedure));
        Assert.Equal(UnsupportedFeatureErrorCode, thrown.ErrorCode);

        Assert.Equal(UpdateRowSource.None, cmd.UpdatedRowSource);
        thrown = AssertSnowflakeDbException(Record.Exception(() => cmd.UpdatedRowSource = UpdateRowSource.FirstReturnedRecord));
        Assert.Equal(UnsupportedFeatureErrorCode, thrown.ErrorCode);

        Assert.Same(UnderlyingConnection(conn), cmd.Connection);
        thrown = AssertSnowflakeDbException(Record.Exception(() => cmd.Connection = null));
        Assert.Equal(UnsupportedFeatureErrorCode, thrown.ErrorCode);

        Assert.False(cmd.DesignTimeVisible);
        thrown = AssertSnowflakeDbException(Record.Exception(() => cmd.DesignTimeVisible = true));
        Assert.Equal(UnsupportedFeatureErrorCode, thrown.ErrorCode);

        var val = cmd.ExecuteScalar();
        Assert.Equal(1L, val);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReadSimpleLargeResultSetAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select seq4(), uniform(1, 10, 42) from table(generator(rowcount => 200000)) v order by 1";
        using (var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            var counter = 0;
            while (await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                Assert.Equal(counter.ToString(CultureInfo.InvariantCulture), reader.GetString(0));
                counter++;
            }

            Assert.Equal(200000, counter);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReadTenThousandRowsInOrderAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select seq4(), uniform(1, 10, 42) from table(generator(rowcount => 10000)) v order by 1";
        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var counter = 0;
        while (await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            Assert.Equal(counter.ToString(CultureInfo.InvariantCulture), reader.GetString(0));
            counter++;
        }

        Assert.Equal(10000, counter);
    }

    [SnowflakeFact]
    public async Task ShouldReadMultiChunkResultSetInOrderAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select seq4(), uniform(1, 10, 42) from table(generator(rowcount => 10000)) v order by 1";
        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var counter = 0;
        while (await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            Assert.Equal(counter.ToString(CultureInfo.InvariantCulture), reader.GetString(0));
            counter++;
        }

        Assert.Equal(10000, counter);
    }

    [SnowflakeTheory]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(4)]
    public async Task ShouldReadMultiChunkResultSetAfterSettingPrefetchThreadsAsync(int prefetchThreads)
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"alter session set CLIENT_PREFETCH_THREADS = {prefetchThreads}";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.CommandText = "select seq4(), uniform(1, 10, 42) from table(generator(rowcount => 10000)) v order by 1";
        using (var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            var counter = 0;
            while (await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                Assert.Equal(counter.ToString(CultureInfo.InvariantCulture), reader.GetString(0));
                counter++;
            }

            Assert.Equal(10000, counter);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowDataSourceErrorAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select * from table_not_exists";
        var thrown = AssertSnowflakeDbException(
            await Record.ExceptionAsync(() => cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false));

        Assert.Equal(ObjectNotFoundErrorCode, thrown.ErrorCode);
        Assert.NotEqual("", QueryIdOf(thrown));

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldCancelQueryAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 20)) v";
        var execution = Task.Run(() => cmd.ExecuteScalar(), TestContext.Current.CancellationToken);

        await Task.Delay(8000, TestContext.Current.CancellationToken).ConfigureAwait(false);
        cmd.Cancel();

        var thrown = await Record.ExceptionAsync(() => execution).ConfigureAwait(false);
        if (thrown?.GetType() != typeof(TaskCanceledException))
            Assert.Equal(ServerQueryCancelledErrorCode, AssertSnowflakeDbException(thrown).ErrorCode);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldCancelQueryOnCommandTimeoutLongerThanRestTimeoutAsync()
    {
        Skip.When(true, "This test case takes too much time so run it manually");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 1020)) v";
        cmd.CommandTimeout = 16 * 60;

        var thrown = AssertSnowflakeDbException(Record.Exception(() =>
        {
            var stopwatch = Stopwatch.StartNew();
            cmd.ExecuteScalar();
            stopwatch.Stop();
            Assert.True(stopwatch.ElapsedMilliseconds < 17 * 60 * 1000);
            Assert.True(stopwatch.ElapsedMilliseconds >= 16 * 60 * 1000);
            Assert.Fail();
        }));
        Assert.Equal(ServerQueryCancelledErrorCode, thrown.ErrorCode);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldCommitAndRollbackTransactionsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        // TODO(SNOW-4382589): transactions
        Skip.FutureMilestone();

        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            var dbConnection = UnderlyingConnection(conn)!;
            var thrown = AssertSnowflakeDbException(await Record.ExceptionAsync(async () =>
                await dbConnection.BeginTransactionAsync(IsolationLevel.ReadUncommitted, TestContext.Current.CancellationToken).ConfigureAwait(false)).ConfigureAwait(false));
            Assert.Equal(UnsupportedFeatureErrorCode, thrown.ErrorCode);

            var tran = await dbConnection.BeginTransactionAsync(IsolationLevel.ReadCommitted, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var command = conn.CreateCommand();
            command.Transaction = tran;
            command.CommandText = $"create or replace table {tableName}(cola string)";
            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            await tran.CommitAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"show tables like '{tableName}'";
            using (var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            }

            tran = await dbConnection.BeginTransactionAsync(IsolationLevel.ReadCommitted, TestContext.Current.CancellationToken).ConfigureAwait(false);
            command.Transaction = tran;
            command.CommandText = $"insert into {tableName} values('test')";
            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"select * from {tableName}";
            using (var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                Assert.Equal("test", reader.GetString(0));
            }

            await tran.RollbackAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"select * from {tableName}";
            using (var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            }
        }
        finally
        {
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldReturnRowsAffectedAsync()
    {
        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        string[] testCommands =
        {
            $"create or replace table {tableName}(cola int, colb string)",
            $"insert into {tableName} values(1, 'a'),(2, 'b')",
            $"merge into {tableName} using (select 1 as cola, 'c' as colb) m on " +
            $"{tableName}.cola = m.cola when matched then update set {tableName}.colb='update' " +
            "when not matched then insert (cola, colb) values (3, 'd')",
            $"drop table if exists {tableName}",
        };
        int[] expectedResult = { 0, 2, 1, 0 };

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            using var command = conn.CreateCommand();
            for (var i = 0; i < testCommands.Length; i++)
            {
                command.CommandText = testCommands[i];
                var rowsAffected = await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

                Assert.Equal(expectedResult[i], rowsAffected);
            }
        }
        finally
        {
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
        }
    }

    [SnowflakeFact]
    public async Task ShouldReturnDbNullFromExecuteScalarWithoutRowsAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var command = conn.CreateCommand())
        {
            command.CommandText = "select 1 where 2 > 3";
            var val = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.Equal(DBNull.Value, val);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteCommandCreatedBeforeOpeningConnectionAsync()
    {
        using var conn = Fixture.Factory.Create(Output);

        using var command = conn.CreateCommand();
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        command.CommandText = "select 1";
        await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnRowsAffectedForUnloadAsync()
    {
        Skip.OnNewDriver(
            "BD#unload-rows-affected-counts-status-rows",
            "The new driver returns the number of unload status rows instead of the number of unloaded rows");

        var affected = await ExecuteUnloadIntoStageAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(6, affected);
    }

    [SnowflakeFact]
    public async Task ShouldReturnStatusRowCountAsRowsAffectedForUnloadAsync()
    {
        Skip.OnOldDriver(
            "BD#unload-rows-affected-counts-status-rows",
            "The old driver returns the number of unloaded rows");

        var affected = await ExecuteUnloadIntoStageAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(1, affected);
    }

    [SnowflakeFact]
    public async Task ShouldInsertWithArrayBindingAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        await ArrayBindTestAsync(NonPooledConnectionString(), tableName, 7500, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldInsertWithArrayBindingFromMultipleThreadsAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var t1TableName = $"SFDBCOMMANDASYNCTEST_1_{Guid.NewGuid():N}";
        var t2TableName = $"SFDBCOMMANDASYNCTEST_2_{Guid.NewGuid():N}";
        var connectionString = NonPooledConnectionString();

        await Task.WhenAll(
            Task.Run(() => ArrayBindTestAsync(connectionString, t1TableName, 15000, TestContext.Current.CancellationToken), TestContext.Current.CancellationToken),
            Task.Run(() => ArrayBindTestAsync(connectionString, t2TableName, 15000, TestContext.Current.CancellationToken), TestContext.Current.CancellationToken)).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnInsertedRowCountFromExecuteScalarAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        using var externalCancel = new CancellationTokenSource();
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(cola INTEGER)");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";
            const int Total = 1000;
            AddParameter(cmd, "1", DbType.Int16, Enumerable.Range(0, Total).ToArray());
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            cmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
            var result = await cmd.ExecuteScalarAsync(externalCancel.Token).ConfigureAwait(false);

            Assert.Equal((long)Total, result);
        }
        finally
        {
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldExecuteLargeQueryWithGcsDownscopedTokenAsync()
    {
        Skip.For(SkipCondition.SkipOnCloudAWS | SkipCondition.SkipOnCloudAzure, "GCS downscoped credentials apply to GCP accounts only");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString() + ";GCS_USE_DOWNSCOPED_CREDENTIAL=true");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        const long RowCount = 100000L;
        using (var command = conn.CreateCommand())
        {
            command.CommandText = $"SELECT COUNT(*) FROM (select seq4() from table(generator(rowcount => {RowCount})))";
            Assert.Equal(RowCount, command.ExecuteScalar());
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnQueryIdOfLastExecutionAsync()
    {
        // TODO(SNOW-4382582): SnowflakeDbDataReader.GetQueryId
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var command = conn.CreateCommand();
        var queryId = GetQueryId(command);
        Assert.Null(queryId);

        command.CommandText = "create or replace temporary table testgetqueryid(cola string)";
        command.ExecuteNonQuery();
        queryId = GetQueryId(command);
        Assert.NotEmpty(queryId!);

        command.CommandText = "show tables like 'testgetqueryid'";
        using (var reader = command.ExecuteReader())
        {
            queryId = GetQueryId(command);
            Assert.NotEmpty(queryId!);
            Assert.Equal(queryId, ReaderQueryId(reader));
            Assert.True(reader.Read());
        }

        command.CommandText = "insert into testgetqueryid values('test')";
        command.ExecuteNonQuery();
        queryId = GetQueryId(command);
        Assert.NotEmpty(queryId!);

        command.CommandText = "select * from testgetqueryid";
        using (var reader = command.ExecuteReader())
        {
            queryId = GetQueryId(command);
            Assert.NotEmpty(queryId!);
            Assert.Equal(queryId, ReaderQueryId(reader));
            Assert.True(reader.Read());
            Assert.Equal("test", reader.GetString(0));
        }

        using var command2 = conn.CreateCommand();
        var queryId2 = GetQueryId(command2);
        Assert.Null(queryId2);
        command2.CommandText = "select 'test2'";
        using (var reader2 = await command2.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            queryId2 = GetQueryId(command2);
            Assert.NotEmpty(queryId2!);
            Assert.Equal(queryId2, ReaderQueryId(reader2));
            Assert.NotEqual(queryId2, queryId);
            Assert.True(reader2.Read());
            Assert.Equal("test2", reader2.GetString(0));
        }

        command.CommandText = $"select * from table(result_scan('{queryId}'))";
        using (var reader = command.ExecuteReader())
        {
            Assert.True(reader.Read());
            Assert.Equal("test", reader.GetString(0));
        }

        command2.CommandText = $"select * from table(result_scan('{queryId2}'))";
        using (var reader2 = await command2.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))
        {
            Assert.True(reader2.Read());
            Assert.Equal("test2", reader2.GetString(0));
        }

        command.CommandText = "select * from table_not_exists";
        var thrown = AssertSnowflakeDbException(Record.Exception(() => command.ExecuteReader()));
        Assert.Equal(ObjectNotFoundErrorCode, thrown.ErrorCode);

        queryId = GetQueryId(command);
        Assert.NotEmpty(queryId!);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteInAsyncModeWithSyncCallsAsync()
    {
        // TODO(SNOW-4382584): ExecuteInAsyncMode and ExecuteAsyncInAsyncMode block until the query finishes
        Skip.FutureMilestone();

        const int ExpectedWaitTime = 5;
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            cmd.CommandText = $"CALL SYSTEM$WAIT({ExpectedWaitTime}, 'SECONDS');";

            var queryId = ExecuteInAsyncMode(cmd);
            var queryStatus = GetQueryStatus(cmd, queryId);

            Assert.True(conn.IsStillRunning(queryStatus), $"Expected query to still be running but status was: {queryStatus}");
            Assert.False(conn.IsAnError(queryStatus), $"Expected query to not be an error but status was: {queryStatus}");

            using var reader = GetResultsFromQueryId(cmd, queryId);

            Assert.True(reader.Read());
            Assert.Equal($"waited {ExpectedWaitTime} seconds", reader.GetString(0));
            Assert.Equal(QueryStatus.Success, GetQueryStatus(cmd, queryId));
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteNormalQueryWhileAsyncExecQueryIsRunningWithSyncCallsAsync()
    {
        // TODO(SNOW-4382584): ExecuteInAsyncMode and ExecuteAsyncInAsyncMode block until the query finishes
        Skip.FutureMilestone();

        const int ExpectedWaitTime = 5;
        var connections = new ISnowflakeDbConnection[3];
        try
        {
            for (var i = 0; i < connections.Length; i++)
            {
                connections[i] = Fixture.Factory.Create(Output, NonPooledConnectionString());
                connections[i].Open();
            }

            string queryId;
            using (var cmd = connections[0].CreateCommand())
            {
                cmd.CommandText = $"CALL SYSTEM$WAIT({ExpectedWaitTime}, 'SECONDS');";

                queryId = ExecuteInAsyncMode(cmd);

                Assert.True(connections[0].IsStillRunning(GetQueryStatus(cmd, queryId)));
            }

            using (var cmd = connections[1].CreateCommand())
            {
                cmd.CommandText = "select 1;";

                var row = cmd.ExecuteScalar();

                Assert.Equal(1L, row);
            }

            using (var cmd = connections[2].CreateCommand())
            {
                using var reader = GetResultsFromQueryId(cmd, queryId);

                Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                Assert.Equal($"waited {ExpectedWaitTime} seconds", reader.GetString(0));
                Assert.Equal(QueryStatus.Success, GetQueryStatus(cmd, queryId));
            }

            foreach (var connection in connections)
                connection.Close();
        }
        finally
        {
            foreach (var connection in connections)
                connection?.Dispose();
        }
    }

    [SnowflakeFact]
    public async Task ShouldThrowErrorForFailedAsyncExecQueryWithSyncCallsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            const int StatusMaxRetryCount = 5;
            var statusRetryCount = 0;
            cmd.CommandText = "SELECT * FROM FAKE_TABLE;";

            var queryId = ExecuteInAsyncMode(cmd);
            while (statusRetryCount < StatusMaxRetryCount && conn.IsStillRunning(GetQueryStatus(cmd, queryId)))
            {
                await Task.Delay(1000, TestContext.Current.CancellationToken).ConfigureAwait(false);
                statusRetryCount++;
            }

            Assert.Equal(QueryStatus.FailedWithError, GetQueryStatus(cmd, queryId));

            var thrown = AssertSnowflakeDbException(Record.Exception(() => GetResultsFromQueryId(cmd, queryId)));

            Assert.Contains("'FAKE_TABLE' does not exist", thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowNotImplementedForPutGetInAsyncModeAsync()
    {
        Skip.OnNewDriver(
            "BD#async-mode-put-get-runs-to-completion",
            "The new driver runs PUT and GET in async mode as blocking executions");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            cmd.CommandText = "PUT file://non_existent_file.csv @~;";

            var thrown = Assert.Throws<NotImplementedException>(() => ExecuteInAsyncMode(cmd));

            Assert.Contains(PutGetInAsyncModeMessage, thrown.Message);

            cmd.CommandText = "GET @~ file://C:\\tmp\\;";

            thrown = Assert.Throws<NotImplementedException>(() => ExecuteInAsyncMode(cmd));

            Assert.Contains(PutGetInAsyncModeMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("ExecuteNonQuery")]
    [InlineData("ExecuteScalar")]
    [InlineData("ExecuteReader")]
    public void ShouldEmitTelemetryForSyncCommandExecution(string method)
    {
        // TODO(SNOW-4382577): telemetry activities
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, TelemetryEnabledConnectionString());
        conn.Open();
        var sessionId = CurrentSessionId(conn);

        var capturedActivities = new ConcurrentQueue<Activity>();
        using var listener = CreateSessionActivityListener(sessionId, capturedActivities, InternalActivitySourceName);
        ActivitySource.AddActivityListener(listener);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1";

        switch (method)
        {
            case "ExecuteNonQuery":
                cmd.ExecuteNonQuery();
                break;
            case "ExecuteScalar":
                cmd.ExecuteScalar();
                break;
            case "ExecuteReader":
                cmd.ExecuteReader().Dispose();
                break;
        }

        var expectedOp = method switch
        {
            "ExecuteNonQuery" => "ExecuteNonQuery",
            "ExecuteScalar" => "ExecuteScalar",
            "ExecuteReader" => "ExecuteDbDataReader",
            _ => throw new ArgumentException(method),
        };
        SpinWait.SpinUntil(() => !capturedActivities.IsEmpty, TimeSpan.FromSeconds(30));
        AssertSingleTelemetryActivity(capturedActivities, expectedOp);
    }

    [SnowflakeTheory]
    [InlineData("ExecuteNonQueryAsync")]
    [InlineData("ExecuteScalarAsync")]
    [InlineData("ExecuteReaderAsync")]
    public async Task ShouldEmitTelemetryForAsyncCommandExecutionAsync(string method)
    {
        // TODO(SNOW-4382577): telemetry activities
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, TelemetryEnabledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var sessionId = CurrentSessionId(conn);

        var capturedActivities = new ConcurrentQueue<Activity>();
        using var listener = CreateSessionActivityListener(sessionId, capturedActivities, InternalActivitySourceName);
        ActivitySource.AddActivityListener(listener);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1";

        switch (method)
        {
            case "ExecuteNonQueryAsync":
                await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                break;
            case "ExecuteScalarAsync":
                await cmd.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                break;
            case "ExecuteReaderAsync":
                await (await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false)).DisposeAsync().ConfigureAwait(false);
                break;
        }

        var expectedOp = method switch
        {
            "ExecuteNonQueryAsync" => "ExecuteNonQueryAsync",
            "ExecuteScalarAsync" => "ExecuteScalarAsync",
            "ExecuteReaderAsync" => "ExecuteDbDataReaderAsync",
            _ => throw new ArgumentException(method),
        };
        SpinWait.SpinUntil(() => !capturedActivities.IsEmpty, TimeSpan.FromSeconds(30));
        AssertSingleTelemetryActivity(capturedActivities, expectedOp);
    }

    [SnowflakeTheory]
    [InlineData("ExecuteInAsyncMode")]
    [InlineData("ExecuteAsyncInAsyncMode")]
    public async Task ShouldEmitTelemetryForAsyncModeExecutionAsync(string method)
    {
        // TODO(SNOW-4382577): telemetry activities
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, TelemetryEnabledConnectionString());
        conn.Open();
        var sessionId = CurrentSessionId(conn);

        var capturedActivities = new ConcurrentQueue<Activity>();
        using var listener = CreateSessionActivityListener(sessionId, capturedActivities, InternalActivitySourceName);
        ActivitySource.AddActivityListener(listener);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1";

        switch (method)
        {
            case "ExecuteInAsyncMode":
                ExecuteInAsyncMode(cmd);
                break;
            case "ExecuteAsyncInAsyncMode":
                await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);
                break;
            default:
                throw new ArgumentException(method);
        }

        SpinWait.SpinUntil(() => !capturedActivities.IsEmpty, TimeSpan.FromSeconds(30));
        AssertSingleTelemetryActivity(capturedActivities, method);
    }

    [SnowflakeFact]
    public async Task ShouldEmitTelemetryWithCustomEventsFromClientAsync()
    {
        Skip.OnNewDriver(
            "BD#activity-helpers-public",
            "The new driver exposes the activity helpers publicly on SnowflakeActivityStarter");

        using var conn = Fixture.Factory.Create(Output, TelemetryEnabledConnectionString());
        conn.Open();
        var sessionId = CurrentSessionId(conn);

        var capturedActivities = new ConcurrentQueue<Activity>();
        using var listener = CreateSessionActivityListener(
            sessionId,
            capturedActivities,
            InternalActivitySourceName,
            ClientDefinedActivitySourceName);
        ActivitySource.AddActivityListener(listener);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1";
        using (var activity1 = StartClientActivity(cmd, "Custom activity!"))
        {
            AddClientEvent(activity1, "event1");
            using (var activity1Nested = StartClientActivity(cmd, "Custom activity 2!"))
            {
                AddClientEvent(activity1, "event2");
                using (var activity1NestedNested = StartClientActivity(cmd, "Custom activity 3!"))
                {
                    AddClientEvent(activity1NestedNested, "event2");
                    activity1NestedNested?.SetTag("custom tag?", "value!");
                    activity1Nested?.SetTag("custom tag?2", "value!2");
                    MarkClientActivityFailed(activity1Nested, new AbandonedMutexException("AAAMutex Yellow Pages"));
                }

                MarkClientActivitySucceeded(activity1);
            }
        }

        await ExecuteInAsyncModeAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);

        SpinWait.SpinUntil(() => capturedActivities.Count > 3, TimeSpan.FromSeconds(30));
        Assert.Equal(4, capturedActivities.Count);
        var capturedActivity1 = capturedActivities.Single(x => x.DisplayName == "Custom activity!");
        var capturedActivity2 = capturedActivities.Single(x => x.DisplayName == "Custom activity 2!");
        var capturedActivity3 = capturedActivities.Single(x => x.DisplayName == "Custom activity 3!");

        Assert.Equal(ActivityKind.Client, capturedActivity1.Kind);
        Assert.Equal(ActivityKind.Client, capturedActivity2.Kind);
        Assert.Equal(ActivityKind.Client, capturedActivity3.Kind);

        Assert.Equal(2, capturedActivity1.Events.Count());
        Assert.Single(capturedActivity2.Events);
        Assert.Single(capturedActivity3.Events);

        Assert.Equal(sessionId, capturedActivity1.GetTagItem(SessionIdTag));
        Assert.Equal(sessionId, capturedActivity2.GetTagItem(SessionIdTag));
        Assert.Equal(sessionId, capturedActivity3.GetTagItem(SessionIdTag));

        Assert.Equal("OK", capturedActivity1.GetTagItem(StatusCodeTag));
        Assert.Equal("ERROR", capturedActivity2.GetTagItem(StatusCodeTag));
        Assert.Null(capturedActivity3.GetTagItem(StatusCodeTag));
    }

    [SnowflakeFact]
    public async Task ShouldEmitTelemetryWithCustomEventsFromPublicActivityHelpersAsync()
    {
        Skip.OnOldDriver(
            "BD#activity-helpers-public",
            "The old driver keeps AddTelemetryEvent, SetException and SetSuccess internal");
        // TODO(SNOW-4382577): telemetry activities
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, TelemetryEnabledConnectionString());
        conn.Open();
        var sessionId = CurrentSessionId(conn);

        var capturedActivities = new ConcurrentQueue<Activity>();
        using var listener = CreateSessionActivityListener(
            sessionId,
            capturedActivities,
            NewActivityStarter.ActivitySourceName,
            NewActivityStarter.ClientDefinedTelemetrySourceName);
        ActivitySource.AddActivityListener(listener);

        using var cmd = (NewSnowflakeDbCommand)conn.CreateCommand();
        cmd.CommandText = "SELECT 1";
        using (var activity1 = cmd.StartActivity("Custom activity!"))
        {
            activity1.AddTelemetryEvent("event1");
            using (var activity1Nested = cmd.StartActivity("Custom activity 2!"))
            {
                activity1.AddTelemetryEvent("event2");
                using (var activity1NestedNested = cmd.StartActivity("Custom activity 3!"))
                {
                    activity1NestedNested.AddTelemetryEvent("event2");
                    activity1NestedNested?.SetTag("custom tag?", "value!");
                    activity1Nested?.SetTag("custom tag?2", "value!2");
                    activity1Nested.SetException(new AbandonedMutexException("AAAMutex Yellow Pages"));
                }

                activity1.SetSuccess();
            }
        }

        await cmd.ExecuteAsyncInAsyncMode(TestContext.Current.CancellationToken).ConfigureAwait(false);

        SpinWait.SpinUntil(() => capturedActivities.Count > 3, TimeSpan.FromSeconds(30));
        Assert.Equal(4, capturedActivities.Count);
        var capturedActivity1 = capturedActivities.Single(x => x.DisplayName == "Custom activity!");
        var capturedActivity2 = capturedActivities.Single(x => x.DisplayName == "Custom activity 2!");
        var capturedActivity3 = capturedActivities.Single(x => x.DisplayName == "Custom activity 3!");

        Assert.Equal(ActivityKind.Client, capturedActivity1.Kind);
        Assert.Equal(ActivityKind.Client, capturedActivity2.Kind);
        Assert.Equal(ActivityKind.Client, capturedActivity3.Kind);

        Assert.Equal(2, capturedActivity1.Events.Count());
        Assert.Single(capturedActivity2.Events);
        Assert.Single(capturedActivity3.Events);

        Assert.Equal(sessionId, capturedActivity1.GetTagItem(SessionIdTag));
        Assert.Equal(sessionId, capturedActivity2.GetTagItem(SessionIdTag));
        Assert.Equal(sessionId, capturedActivity3.GetTagItem(SessionIdTag));

        Assert.Equal("OK", capturedActivity1.GetTagItem(StatusCodeTag));
        Assert.Equal("ERROR", capturedActivity2.GetTagItem(StatusCodeTag));
        Assert.Null(capturedActivity3.GetTagItem(StatusCodeTag));
    }

    [SnowflakeFact]
    public async Task ShouldOverrideConnectionStringQueryTagWithCommandQueryTagAsync()
    {
        const string ExpectedQueryTag = "Test QUERY_TAG 12345";
        const string ConnectQueryTag = "Test 123";
        using var conn = Fixture.Factory.Create(Output, $"{BaseConnectionString()};query_tag={ConnectQueryTag}");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var command = conn.CreateCommand();
        SetQueryTag(command, ExpectedQueryTag);
        command.CommandText = "SELECT QUERY_TAG FROM table(information_schema.query_history_by_session())";
        var queryTag = command.ExecuteScalar();

        Assert.Equal(ExpectedQueryTag, queryTag);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteCommandWithCommentEmbeddedWithSyncCallsAsync()
    {
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var command = conn.CreateCommand();

        command.CommandText = "\r\nselect '--'\r\n";
        using var reader = command.ExecuteReader();

        Assert.True(reader.Read());
        Assert.Equal("--", reader.GetString(0));
    }

    [SnowflakeFact]
    public async Task ShouldExecuteCommandWithCommentEmbeddedAsync()
    {
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var command = conn.CreateCommand();

        command.CommandText = "\r\nselect '--'\r\n";
        using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.Equal("--", reader.GetString(0));
    }

    [SnowflakeFact]
    public async Task ShouldReturnRowCountFromExecuteNonQueryForUploadWithMultipleFilesAsync()
    {
        Skip.OnNewDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The new driver returns the number of COPY status rows instead of the number of loaded rows");

        const int NumberOfFiles = 5;
        const int NumberOfRows = 3;
        const int ExpectedRowCount = NumberOfFiles * NumberOfRows;

        var actualRowCount = await ExecuteCopyIntoFromGeneratedCsvFilesAsync(
            NumberOfFiles,
            NumberOfRows,
            TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ExpectedRowCount, actualRowCount);
    }

    [SnowflakeFact]
    public async Task ShouldReturnStatusRowCountFromExecuteNonQueryForUploadWithMultipleFilesAsync()
    {
        Skip.OnOldDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The old driver returns the number of loaded rows");

        const int NumberOfFiles = 5;
        const int NumberOfRows = 3;
        const int ExpectedRowCount = NumberOfFiles;

        var actualRowCount = await ExecuteCopyIntoFromGeneratedCsvFilesAsync(
            NumberOfFiles,
            NumberOfRows,
            TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ExpectedRowCount, actualRowCount);
    }

    [SnowflakeFact]
    public async Task ShouldReturnRowCountFromExecuteNonQueryAsyncForUploadWithMultipleFilesAsync()
    {
        Skip.OnNewDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The new driver returns the number of COPY status rows instead of the number of loaded rows");

        const int NumberOfFiles = 5;
        const int NumberOfRows = 3;
        const int ExpectedRowCount = NumberOfFiles * NumberOfRows;

        var actualRowCount = await ExecuteCopyIntoFromGeneratedCsvFilesAsync(
            NumberOfFiles,
            NumberOfRows,
            TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ExpectedRowCount, actualRowCount);
    }

    [SnowflakeFact]
    public async Task ShouldReturnStatusRowCountFromExecuteNonQueryAsyncForUploadWithMultipleFilesAsync()
    {
        Skip.OnOldDriver(
            "BD#copy-into-table-rows-affected-counts-status-rows",
            "The old driver returns the number of loaded rows");

        const int NumberOfFiles = 5;
        const int NumberOfRows = 3;
        const int ExpectedRowCount = NumberOfFiles;

        var actualRowCount = await ExecuteCopyIntoFromGeneratedCsvFilesAsync(
            NumberOfFiles,
            NumberOfRows,
            TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ExpectedRowCount, actualRowCount);
    }

    private async Task<int> ExecuteCopyIntoFromGeneratedCsvFilesAsync(
        int numberOfFiles,
        int numberOfRows,
        CancellationToken cancellationToken)
    {
        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        var tempFolder = $"{Path.GetTempPath()}Temp_{Guid.NewGuid()}";
        try
        {
            Directory.CreateDirectory(tempFolder);
            var data = string.Concat(Enumerable.Repeat(string.Join(",", "TestData") + "\n", numberOfRows));
            for (var i = 0; i < numberOfFiles; i++)
                File.WriteAllText(Path.Combine(tempFolder, $"{GetType().Name}_{i}.csv"), data);

            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(COL1 STRING)");
            cmd.CommandText = $"PUT file://{Path.Combine(tempFolder, "*.csv")} @%{tableName} AUTO_COMPRESS=FALSE";
            using (cmd.ExecuteReader())
            {
            }

            cmd.CommandText = $"COPY INTO {tableName} FROM @%{tableName} PATTERN='.*.csv' FILE_FORMAT=(TYPE=CSV)";
            return await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
        }
        finally
        {
            Directory.Delete(tempFolder, true);
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
        }
    }

    private async Task<int> ExecuteUnloadIntoStageAsync(CancellationToken cancellationToken)
    {
        var tableName = $"SFDBCOMMANDASYNCTEST_{Guid.NewGuid():N}";
        var stageName = $"SFDBCOMMANDASYNCTEST_STAGE_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            using var command = conn.CreateCommand();
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(c1 NUMBER)");

            command.CommandText = $"insert into {tableName} values(1), (2), (3), (4), (5), (6)";
            await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

            command.CommandText = $"drop stage if exists {stageName}";
            await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

            command.CommandText = $"create stage if not exists {stageName}";
            await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

            command.CommandText = $"copy into @{stageName}/unload/ from {tableName};";
            var affected = await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

            command.CommandText = $"drop stage if exists {stageName}";
            await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

            return affected;
        }
        finally
        {
            ExecuteNonQuery(conn, $"drop stage if exists {stageName}");
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
            await conn.CloseAsync(cancellationToken).ConfigureAwait(false);
        }
    }

    private async Task ArrayBindTestAsync(string connectionString, string tableName, int size, CancellationToken cancellationToken)
    {
        const int TimeoutSeconds = 150;
        using var externalCancel = new CancellationTokenSource(TimeSpan.FromSeconds(TimeoutSeconds));
        using var conn = Fixture.Factory.Create(Output, connectionString);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        // The old driver formats stage-bound DATE values with the current culture, which the server rejects
        // for cultures whose short date pattern is not month-first.
        var originalCulture = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        try
        {
            ExecuteNonQuery(
                conn,
                $"CREATE OR REPLACE TABLE {tableName}(cola INTEGER, colb STRING, colc DATE, cold TIME, cole TIMESTAMP_NTZ, colf TIMESTAMP_TZ)");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?, ?, ?, ?, ?, ?)";

            var arrint = new List<int>();
            for (var i = 0; i < size; i++)
            {
                arrint.Add(i * 10 + 1);
                arrint.Add(i * 10 + 2);
                arrint.Add(i * 10 + 3);
            }

            AddParameter(cmd, "1", DbType.Int16, arrint.ToArray());
            AddParameter(cmd, "2", DbType.String, Triplicate(size, "str1", "str2", "str3\"test\""));

            var date1 = DateTime.ParseExact("2000-01-01 00:00:00.0000000", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var date2 = DateTime.ParseExact("2020-05-11 23:59:59.9999999", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var date3 = DateTime.ParseExact("2021-07-22 23:59:59.9999999", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            AddParameter(cmd, "3", DbType.Date, Triplicate(size, date1, date2, date3));

            var time1 = DateTime.ParseExact("00:00:00.0000000", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var time2 = DateTime.ParseExact("23:59:59.9999999", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var time3 = DateTime.ParseExact("12:35:41.3333333", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            AddParameter(cmd, "4", DbType.Time, Triplicate(size, time1, time2, time3));

            var ntz1 = DateTime.ParseExact("2017-01-01 12:00:00", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            var ntz2 = DateTime.ParseExact("2020-12-31 23:59:59", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            var ntz3 = DateTime.ParseExact("2022-04-01 00:00:00", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            AddParameter(cmd, "5", DbType.DateTime2, Triplicate(size, ntz1, ntz2, ntz3));

            var tz1 = DateTimeOffset.Now;
            var tz2 = DateTimeOffset.UtcNow;
            var tz3 = new DateTimeOffset(2007, 1, 1, 12, 0, 0, new TimeSpan(4, 0, 0));
            AddParameter(cmd, "6", DbType.DateTimeOffset, Triplicate(size, tz1, tz2, tz3));

            var insert = cmd.ExecuteNonQueryAsync(externalCancel.Token);
            var completed = await Task.WhenAny(insert, Task.Delay(TimeSpan.FromSeconds(TimeoutSeconds), cancellationToken)).ConfigureAwait(false);
            Assert.Same(insert, completed);
            Assert.Equal(size * 3, await insert.ConfigureAwait(false));

            cmd.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(cancellationToken).ConfigureAwait(false);
            Assert.True(await reader.ReadAsync(externalCancel.Token).ConfigureAwait(false));
        }
        finally
        {
            CultureInfo.CurrentCulture = originalCulture;
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
            await conn.CloseAsync(cancellationToken).ConfigureAwait(false);
        }
    }

    private static T[] Triplicate<T>(int times, T first, T second, T third)
    {
        var values = new List<T>(times * 3);
        for (var i = 0; i < times; i++)
        {
            values.Add(first);
            values.Add(second);
            values.Add(third);
        }

        return values.ToArray();
    }

    private string BaseConnectionString()
    {
        using var probe = Fixture.Factory.Create(Output);
        return probe.ConnectionString.TrimEnd(';');
    }

    private string NonPooledConnectionString() => $"{BaseConnectionString()};poolingEnabled=false";

    private string TelemetryEnabledConnectionString() => $"{NonPooledConnectionString()};client_telemetry_enabled=true";

    private static int ExecuteNonQuery(ISnowflakeDbConnection conn, string commandText)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        return cmd.ExecuteNonQuery();
    }

    private static void AddParameter(DbCommand cmd, string name, DbType dbType, object value)
    {
        var parameter = cmd.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        cmd.Parameters.Add(parameter);
    }

    private static DbConnection? UnderlyingConnection(ISnowflakeDbConnection conn)
    {
        if (conn is DbConnection dbConnection)
            return dbConnection;

        using var sibling = conn.CreateCommand();
        return sibling.Connection;
    }

    private static string CurrentSessionId(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select current_session()";
        return Convert.ToString(cmd.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static ActivityListener CreateSessionActivityListener(
        string sessionId,
        ConcurrentQueue<Activity> capturedActivities,
        params string[] sourceNames) =>
        new()
        {
            ShouldListenTo = source => sourceNames.Contains(source.Name),
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData,
            ActivityStopped = activity =>
            {
                if (activity.GetTagItem(SessionIdTag)?.Equals(sessionId) == true)
                    capturedActivities.Enqueue(activity);
            },
        };

    private static Activity? StartClientActivity(DbCommand cmd, string name) => cmd switch
    {
        OldSnowflakeDbCommand old => OldActivityStarter.StartActivity(old, name),
        NewSnowflakeDbCommand current => current.StartActivity(name),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static void AddClientEvent(Activity? activity, string name) =>
        activity?.AddEvent(new ActivityEvent(name));

    private static void MarkClientActivitySucceeded(Activity? activity)
    {
        activity?.SetTag(StatusCodeTag, "OK");
        activity?.Stop();
    }

    private static void MarkClientActivityFailed(Activity? activity, Exception exception)
    {
        activity?.SetTag(StatusCodeTag, "ERROR");
        activity?.AddEvent(new ActivityEvent(
            "exception",
            tags: new ActivityTagsCollection { { ExceptionTypeTag, exception.GetType().FullName } }));
        activity?.Stop();
    }

    private void AssertSingleTelemetryActivity(ConcurrentQueue<Activity> capturedActivities, string expectedOperationName)
    {
        Output.WriteLine($"Captured activities: [{string.Join(", ", capturedActivities.Select(a => a.OperationName))}]");
        var activity = Assert.Single(capturedActivities, a => a.OperationName == expectedOperationName);

        Assert.Equal(ActivityKind.Client, activity.Kind);
        Assert.Equal("OK", activity.GetTagItem(StatusCodeTag));
        Assert.Equal("snowflake", activity.GetTagItem(DbSystemTag));
        Assert.NotNull(activity.GetTagItem(SessionIdTag));
        Assert.NotNull(activity.GetTagItem(DbWarehouseTag));
        Assert.NotNull(activity.GetTagItem(DbRoleTag));
        Assert.NotNull(activity.GetTagItem(DbNameTag));
    }

    private static string ExecuteInAsyncMode(DbCommand cmd) => cmd switch
    {
        OldSnowflakeDbCommand old => old.ExecuteInAsyncMode(),
        NewSnowflakeDbCommand current => current.ExecuteInAsyncMode(),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static Task<string> ExecuteInAsyncModeAsync(DbCommand cmd, CancellationToken cancellationToken) => cmd switch
    {
        OldSnowflakeDbCommand old => old.ExecuteAsyncInAsyncMode(cancellationToken),
        NewSnowflakeDbCommand current => current.ExecuteAsyncInAsyncMode(cancellationToken),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static QueryStatus GetQueryStatus(DbCommand cmd, string queryId) => cmd switch
    {
        OldSnowflakeDbCommand old => (QueryStatus)old.GetQueryStatus(queryId),
        NewSnowflakeDbCommand current => current.GetQueryStatus(queryId),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static Task<QueryStatus> GetQueryStatusAsync(DbCommand cmd, string queryId, CancellationToken cancellationToken) => cmd switch
    {
        OldSnowflakeDbCommand old => ToQueryStatusAsync(old.GetQueryStatusAsync(queryId, cancellationToken), cancellationToken),
        NewSnowflakeDbCommand current => current.GetQueryStatusAsync(queryId, cancellationToken),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static async Task<QueryStatus> ToQueryStatusAsync<TStatus>(Task<TStatus> statusTask, CancellationToken cancellationToken)
        where TStatus : struct, Enum
    {
        cancellationToken.ThrowIfCancellationRequested();
        var status = await statusTask.ConfigureAwait(false);
        return (QueryStatus)Convert.ToInt32(status, CultureInfo.InvariantCulture);
    }

    private static DbDataReader GetResultsFromQueryId(DbCommand cmd, string queryId) => cmd switch
    {
        OldSnowflakeDbCommand old => old.GetResultsFromQueryId(queryId),
        NewSnowflakeDbCommand current => current.GetResultsFromQueryId(queryId),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static Task<DbDataReader> GetResultsFromQueryIdAsync(DbCommand cmd, string queryId, CancellationToken cancellationToken) => cmd switch
    {
        OldSnowflakeDbCommand old => old.GetResultsFromQueryIdAsync(queryId, cancellationToken),
        NewSnowflakeDbCommand current => current.GetResultsFromQueryIdAsync(queryId, cancellationToken),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static string? GetQueryId(DbCommand cmd) => cmd switch
    {
        OldSnowflakeDbCommand old => old.GetQueryId(),
        NewSnowflakeDbCommand current => current.GetQueryId(),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static string ReaderQueryId(DbDataReader reader) => reader switch
    {
        OldSnowflakeDbDataReader old => old.GetQueryId(),
        _ => throw new NotSupportedException($"{reader.GetType()} does not expose the query id"),
    };

    private static void SetQueryTag(DbCommand cmd, string queryTag)
    {
        switch (cmd)
        {
            case OldSnowflakeDbCommand old:
                old.QueryTag = queryTag;
                break;
            case NewSnowflakeDbCommand current:
                current.QueryTag = queryTag;
                break;
            default:
                throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}");
        }
    }

    private static DbException AssertSnowflakeDbException(Exception? exception)
    {
        Assert.NotNull(exception);
        var expectedType = TestDriver.Current == TestDriver.Kind.Old
            ? typeof(OldSnowflakeDbException)
            : typeof(NewSnowflakeDbException);
        Assert.IsType(expectedType, exception);
        return (DbException)exception;
    }

    private static string QueryIdOf(DbException exception) => exception switch
    {
        OldSnowflakeDbException old => old.QueryId,
        NewSnowflakeDbException current => current.QueryId,
        _ => throw new InvalidOperationException($"Unexpected exception type {exception.GetType()}"),
    };
}
