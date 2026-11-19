using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbCommand = Snowflake.Data.SnowflakeDbCommand;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFDbCommandTest : ReferenceTestBase
{
    private const int ObjectNotFoundErrorCode = 2003;
    private const int ServerQueryCancelledErrorCode = 604;
    private const int QueryCancelledErrorCode = 270005;
    private const string InvalidQueryIdMessage = "Invalid query id format. Expected a UUID.";
    private const string NoDataMaxRetryMessage = "Max retry for no data is reached";

    public SFDbCommandTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public void ShouldThrowDataSourceError()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        conn.Open();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select * from table_not_exists";

        var thrown = AssertSnowflakeDbException(Record.Exception(() => cmd.ExecuteReader()));
        Assert.Equal(ObjectNotFoundErrorCode, thrown.ErrorCode);
        Assert.NotEqual("", QueryIdOf(thrown));

        conn.Close();
    }

    [SnowflakeFact]
    public async Task ShouldCancelQueryAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        conn.Open();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 20)) v";
        var execution = Task.Run(() => cmd.ExecuteScalar(), TestContext.Current.CancellationToken);

        await Task.Delay(8000, TestContext.Current.CancellationToken).ConfigureAwait(false);
        cmd.Cancel();

        var thrown = await Record.ExceptionAsync(() => execution).ConfigureAwait(false);
        if (thrown is not TaskCanceledException)
            Assert.Equal(ServerQueryCancelledErrorCode, AssertSnowflakeDbException(thrown).ErrorCode);

        conn.Close();
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldPropagateExternalCancellationDuringBindStageUploadAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = $"SFDBCOMMANDTEST_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"create or replace table {tableName}(cola INTEGER)");

            using (var prepCmd = conn.CreateCommand())
            {
                prepCmd.CommandText = $"insert into {tableName} values (?)";
                AddParameter(prepCmd, "1", DbType.Int16, Enumerable.Range(0, 2).ToArray());

                ExecuteNonQuery(conn, "alter session set CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 1");
                await prepCmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"insert into {tableName} values (?)";
            cmd.CommandTimeout = 0;
            AddParameter(cmd, "1", DbType.Int16, Enumerable.Range(0, 5_000_000).ToArray());

            const int CancelDelaySeconds = 3;
            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(CancelDelaySeconds));

            var sw = Stopwatch.StartNew();
            var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
                () => cmd.ExecuteNonQueryAsync(cts.Token)).ConfigureAwait(false);
            sw.Stop();

            var detail = AssertSnowflakeDbException(thrown.InnerException);
            Assert.Equal(QueryCancelledErrorCode, detail.ErrorCode);
            Assert.True(
                sw.Elapsed.TotalSeconds < CancelDelaySeconds + 30,
                "Expected cancellation to propagate into the stage PUT upload. " +
                $"Elapsed: {sw.Elapsed.TotalSeconds.ToString("F1", CultureInfo.InvariantCulture)}s, CancelAfter: {CancelDelaySeconds}s.");
        }
        finally
        {
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldExecuteQueryOnAsyncOpenedConnectionAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using (var cmd = conn.CreateCommand())
        {
            cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 3)) v";
            using var reader = cmd.ExecuteReader();
            Assert.True(reader.Read());
            var queryResult = reader.GetInt64(0);
            Assert.False(reader.Read());
            Assert.NotEqual(0, queryResult);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldExecuteQueriesInParallelOnOneConnectionAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        var tasks = new Task[5];
        for (var i = 0; i < tasks.Length; i++)
        {
            tasks[i] = Task.Run(
                () =>
                {
                    using var cmd = conn.CreateCommand();
                    cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 3)) v";
                    using var reader = cmd.ExecuteReader();
                    Assert.True(reader.Read());
                    var queryResult = reader.GetInt64(0);
                    Assert.False(reader.Read());
                    Assert.NotEqual(0, queryResult);
                },
                TestContext.Current.CancellationToken);
        }

        await Task.WhenAll(tasks).ConfigureAwait(false);
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
            var thrown = Assert.Throws<Exception>(() => GetQueryStatus(cmd, "fakeQueryId"));

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
            var thrown = AssertSnowflakeDbException(Record.Exception(() => GetQueryStatus(cmd, "fakeQueryId")));

            Assert.Contains(InvalidQueryIdMessage, thrown.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenGettingResultsOfInvalidQueryIdAsync()
    {
        Skip.OnNewDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The new driver throws SnowflakeDbException instead of an AggregateException for a malformed query id");

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = Assert.Throws<AggregateException>(() => GetResultsFromQueryId(cmd, "fakeQueryId"));

            Assert.NotNull(thrown.InnerException);
            Assert.Contains(InvalidQueryIdMessage, thrown.InnerException.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowSnowflakeDbExceptionWhenGettingResultsOfInvalidQueryIdAsync()
    {
        Skip.OnOldDriver(
            "BD#invalid-query-id-throws-snowflake-db-exception",
            "The old driver wraps the malformed query id failure in an AggregateException");
        // TODO(SNOW-4382591): SnowflakeDbException construction, ErrorCode and QueryId
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var cmd = conn.CreateCommand())
        {
            var thrown = AssertSnowflakeDbException(Record.Exception(() => GetResultsFromQueryId(cmd, "fakeQueryId")));

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
            var queryStatus = GetQueryStatus(cmd, "ab123cde-1cba-789a-987f-1234a56b789c");

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
            var thrown = Assert.Throws<AggregateException>(
                () => GetResultsFromQueryId(cmd, "ba987def-1abc-987f-987f-1234a56b789c"));

            Assert.NotNull(thrown.InnerException);
            Assert.Contains(NoDataMaxRetryMessage, thrown.InnerException.Message);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    private string NonPooledConnectionString()
    {
        using var probe = Fixture.Factory.Create(Output);
        return $"{probe.ConnectionString.TrimEnd(';')};poolingEnabled=false";
    }

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

    private static QueryStatus GetQueryStatus(DbCommand cmd, string queryId) => cmd switch
    {
        OldSnowflakeDbCommand old => (QueryStatus)old.GetQueryStatus(queryId),
        NewSnowflakeDbCommand current => current.GetQueryStatus(queryId),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

    private static DbDataReader GetResultsFromQueryId(DbCommand cmd, string queryId) => cmd switch
    {
        OldSnowflakeDbCommand old => old.GetResultsFromQueryId(queryId),
        NewSnowflakeDbCommand current => current.GetResultsFromQueryId(queryId),
        _ => throw new InvalidOperationException($"Unexpected command type {cmd.GetType()}"),
    };

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
