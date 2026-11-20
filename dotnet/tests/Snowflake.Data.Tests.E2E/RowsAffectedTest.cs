using System.Data.Common;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class RowsAffectedTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public RowsAffectedTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    public enum Execution
    {
        Sync,
        Async,
    }

    // Scenario: should report the row count of a SELECT as rows affected
    [SnowflakeTheory]
    [InlineData(Execution.Sync)]
    [InlineData(Execution.Async)]
    public async Task ShouldReportTheRowCountOfASelectAsRowsAffectedAsync(Execution execution)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 5)) v" is executed as a non-query and as a reader
        const string sql = "SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 5)) v";
        var nonQueryResult = await ExecuteNonQueryAsync(connection.CreateCommand(), sql, execution, CancellationToken.None)
            .ConfigureAwait(false);
        var recordsAffected = await ReaderRecordsAffectedAsync(connection.CreateCommand(), sql, execution, CancellationToken.None)
            .ConfigureAwait(false);

        // Then the non-query result is 5
        var oldDriver = BehaviorDifference.OldDriverOnly("BD#select-rows-affected-reports-row-count");
        Assert.Equal(oldDriver ? 0 : 5, nonQueryResult);

        // And the reader reports 5 records affected
        Assert.Equal(oldDriver ? -1 : 5, recordsAffected);
    }

    // Scenario: should report the row count of a DESCRIBE as rows affected
    [SnowflakeTheory]
    [InlineData(Execution.Sync)]
    [InlineData(Execution.Async)]
    public async Task ShouldReportTheRowCountOfADescribeAsRowsAffectedAsync(Execution execution)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And a temporary table with 3 columns exists
        var tableName = $"UD_ROWS_AFFECTED_DESC_{Guid.NewGuid():N}";
        RunSetup(connection, $"CREATE TEMPORARY TABLE {tableName} (a INT, b STRING, c DATE)");

        // When Query "DESCRIBE TABLE <table>" is executed as a non-query and as a reader
        var sql = $"DESCRIBE TABLE {tableName}";
        var nonQueryResult = await ExecuteNonQueryAsync(connection.CreateCommand(), sql, execution, CancellationToken.None)
            .ConfigureAwait(false);
        var recordsAffected = await ReaderRecordsAffectedAsync(connection.CreateCommand(), sql, execution, CancellationToken.None)
            .ConfigureAwait(false);

        // Then the non-query result is 3
        var expected = BehaviorDifference.OldDriverOnly("BD#row-returning-statements-rows-affected-report-row-count") ? 0 : 3;
        Assert.Equal(expected, nonQueryResult);

        // And the reader reports 3 records affected
        Assert.Equal(expected, recordsAffected);
    }

    // Scenario: should report minus one records affected for DDL statement
    [SnowflakeTheory]
    [InlineData(Execution.Sync)]
    [InlineData(Execution.Async)]
    public async Task ShouldReportMinusOneRecordsAffectedForDDLStatementAsync(Execution execution)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When a CREATE TEMPORARY TABLE statement is executed as a non-query and another as a reader
        var nonQuerySql = $"CREATE TEMPORARY TABLE UD_ROWS_AFFECTED_DDL_{Guid.NewGuid():N} (a INT)";
        var readerSql = $"CREATE TEMPORARY TABLE UD_ROWS_AFFECTED_DDL_{Guid.NewGuid():N} (a INT)";
        var nonQueryResult = await ExecuteNonQueryAsync(connection.CreateCommand(), nonQuerySql, execution, CancellationToken.None)
            .ConfigureAwait(false);
        var recordsAffected = await ReaderRecordsAffectedAsync(connection.CreateCommand(), readerSql, execution, CancellationToken.None)
            .ConfigureAwait(false);

        // Then the non-query result is 0
        Assert.Equal(0, nonQueryResult);

        // And the reader reports -1 records affected
        Assert.Equal(
            BehaviorDifference.OldDriverOnly("BD#no-result-statements-records-affected-minus-one") ? 0 : -1,
            recordsAffected);
    }

    // Scenario: should report the number of unload status rows for COPY INTO a stage
    [SnowflakeTheory]
    [InlineData(Execution.Sync)]
    [InlineData(Execution.Async)]
    public async Task ShouldReportTheNumberOfUnloadStatusRowsForCopyIntoAStageAsync(Execution execution)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And a temporary stage and a temporary table with 1000 rows exist
        var suffix = Guid.NewGuid().ToString("N");
        var stageName = $"UD_ROWS_AFFECTED_STAGE_{suffix}";
        var tableName = $"UD_ROWS_AFFECTED_SRC_{suffix}";
        RunSetup(connection, $"CREATE TEMPORARY STAGE {stageName}");
        RunSetup(
            connection,
            $"CREATE TEMPORARY TABLE {tableName} AS SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v");

        // When the table is unloaded into one file of the stage as a non-query
        var nonQueryResult = await ExecuteNonQueryAsync(
            connection.CreateCommand(),
            $"COPY INTO @{stageName}/unload FROM {tableName} FILE_FORMAT = (TYPE = CSV) SINGLE = TRUE",
            execution,
            CancellationToken.None).ConfigureAwait(false);

        // Then the non-query result is 1
        Assert.Equal(
            BehaviorDifference.OldDriverOnly("BD#unload-rows-affected-counts-status-rows") ? 1000 : 1,
            nonQueryResult);
    }

    // Scenario: should report the number of load status rows for COPY INTO a table
    [SnowflakeTheory]
    [InlineData(Execution.Sync)]
    [InlineData(Execution.Async)]
    public async Task ShouldReportTheNumberOfLoadStatusRowsForCopyIntoATableAsync(Execution execution)
    {
        Skip.FutureMilestone("TODO SNOW-4483010 next milestone");
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And a temporary stage holding one file with 1000 rows and an empty temporary table exist
        var suffix = Guid.NewGuid().ToString("N");
        var stageName = $"UD_ROWS_AFFECTED_STAGE_{suffix}";
        var tableName = $"UD_ROWS_AFFECTED_DST_{suffix}";
        RunSetup(connection, $"CREATE TEMPORARY STAGE {stageName}");
        RunSetup(
            connection,
            $"COPY INTO @{stageName}/load FROM (SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v) " +
            "FILE_FORMAT = (TYPE = CSV) SINGLE = TRUE");
        RunSetup(connection, $"CREATE TEMPORARY TABLE {tableName} (id NUMBER)");

        // When the file is copied into the table as a non-query
        var nonQueryResult = await ExecuteNonQueryAsync(
            connection.CreateCommand(),
            $"COPY INTO {tableName} FROM @{stageName}/load FILE_FORMAT = (TYPE = CSV)",
            execution,
            CancellationToken.None).ConfigureAwait(false);

        // Then the non-query result is 1
        Assert.Equal(
            BehaviorDifference.OldDriverOnly("BD#copy-into-table-rows-affected-counts-status-rows") ? 1000 : 1,
            nonQueryResult);

        // And the table holds 1000 rows
        using var count = connection.CreateCommand();
        count.CommandText = $"SELECT COUNT(*) FROM {tableName}";
        Assert.Equal(1000L, Convert.ToInt64(count.ExecuteScalar()));
    }

    private static void RunSetup(ISnowflakeDbConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        command.ExecuteNonQuery();
    }

    private static async Task<int> ExecuteNonQueryAsync(
        DbCommand command, string sql, Execution execution, CancellationToken cancelToken)
    {
        using (command)
        {
            command.CommandText = sql;
            return execution == Execution.Async
                ? await command.ExecuteNonQueryAsync(cancelToken).ConfigureAwait(false)
                : command.ExecuteNonQuery();
        }
    }

    private static async Task<int> ReaderRecordsAffectedAsync(
        DbCommand command, string sql, Execution execution, CancellationToken cancelToken)
    {
        using (command)
        {
            command.CommandText = sql;
            using var reader = execution == Execution.Async
                ? await command.ExecuteReaderAsync(cancelToken).ConfigureAwait(false)
                : command.ExecuteReader();
            return reader.RecordsAffected;
        }
    }
}
