using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Tests.Assertions;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class BasicExecuteQueryTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const string QueryErrorGap =
        "Query errors surface as NotImplementedException because SnowflakeDbException is not implemented (SNOW-4382591)";

    public BasicExecuteQueryTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should execute simple SELECT returning single value
    [SnowflakeFact]
    public async Task ShouldExecuteSimpleSelectReturningSingleValueAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT 1 AS value" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS value";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain value 1
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.IsDBNull(0), "Expected non-NULL value");
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should execute SELECT returning multiple columns
    [SnowflakeFact]
    public async Task ShouldExecuteSelectReturningMultipleColumnsAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain:
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal((string?)"hello", (string?)reader.GetString(1));
        Assert.Equal((string?)"3.14", (string?)reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should execute SELECT returning multiple rows
    [SnowflakeFact]
    public async Task ShouldExecuteSelectReturningMultipleRowsAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then there are 5 numbered sequentially rows returned
        for (long i = 0; i < 5; i++)
        {
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), $"Expected row {i}");
            Assert.Equal(i, reader.GetInt64(0));
        }
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly 5 rows");
    }

    // Scenario: should execute SELECT returning empty result set
    [SnowflakeFact]
    public async Task ShouldExecuteSelectReturningEmptyResultSetAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT 1 WHERE 1=0" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 WHERE 1=0";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result set should be empty
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected empty result set");
    }

    // Scenario: should execute SELECT returning NULL values
    [SnowflakeFact]
    public async Task ShouldExecuteSelectReturningNullValuesAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL AS col1, 42 AS col2, NULL AS col3";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain NULL for col1 and col3 and 42 for col2
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for col1");
        Assert.Equal(42, reader.GetInt32(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for col3");
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should execute CREATE and DROP TABLE statements
    [SnowflakeFact]
    public async Task ShouldExecuteCreateAndDropTableStatementsAsync()
    {
        Skip.FutureMilestone(QueryErrorGap);
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        var tableName = $"UD_TEST_DDL_{Guid.NewGuid():N}";
        try
        {
            // When CREATE TABLE statement is executed
            using var createCmd = connection.CreateCommand();
            createCmd.CommandText = $"CREATE TABLE {tableName} (id INT, name VARCHAR(100))";
            await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

            // Then the table should be created successfully
            using var verifyCmd = connection.CreateCommand();
            verifyCmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
            Assert.Equal(0L, Convert.ToInt64(await verifyCmd.ExecuteScalarAsync(ct).ConfigureAwait(false), CultureInfo.InvariantCulture));

            // And DROP TABLE statement should complete successfully
            using var dropCmd = connection.CreateCommand();
            dropCmd.CommandText = $"DROP TABLE {tableName}";
            await dropCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

            using var verifyDropCmd = connection.CreateCommand();
            verifyDropCmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
            var exception = await Assert.ThrowsAnyAsync<DbException>(async () => await verifyDropCmd.ExecuteScalarAsync(ct).ConfigureAwait(false)).ConfigureAwait(false);
            DbExceptionAssert.HasDiagnostics(exception, sqlState: "42S02",
                errorCode: 2003,
                messageContains: tableName.ToUpperInvariant());
        }
        finally
        {
            using var cleanupCmd = connection.CreateCommand();
            cleanupCmd.CommandText = $"DROP TABLE IF EXISTS {tableName}";
            await cleanupCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }
    }

    // Scenario: should execute INSERT and retrieve inserted data
    [SnowflakeFact]
    public async Task ShouldExecuteInsertAndRetrieveInsertedDataAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And A temporary table is created
        var tableName = $"UD_TEST_DML_{Guid.NewGuid():N}";
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, value VARCHAR(50))";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When INSERT statement is executed to add rows
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id, value) VALUES (1, 'one'), (2, 'two'), (3, 'three')";
        await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Query "SELECT id, value FROM {table} ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, value FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the inserted data should be correctly returned
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal((string?)"one", (string?)reader.GetString(1));

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(2, reader.GetInt32(0));
        Assert.Equal((string?)"two", (string?)reader.GetString(1));

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        Assert.Equal(3, reader.GetInt32(0));
        Assert.Equal((string?)"three", (string?)reader.GetString(1));

        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly 3 rows");
    }

    // Scenario: should return error for invalid SQL syntax
    [SnowflakeFact]
    public async Task ShouldReturnErrorForInvalidSqlSyntaxAsync()
    {
        Skip.FutureMilestone(QueryErrorGap);
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Invalid SQL "SELCT INVALID SYNTAX" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELCT INVALID SYNTAX";

        // Then An error should be returned
        var exception = await Assert.ThrowsAnyAsync<DbException>(async () => await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false)).ConfigureAwait(false);
        DbExceptionAssert.HasDiagnostics(exception, sqlState: "42000", errorCode: 1003);
    }

    // Scenario: should return proper error for NULL in NOT NULL column
    [SnowflakeFact]
    public async Task ShouldReturnProperErrorForNullInNotNullColumnAsync()
    {
        Skip.FutureMilestone(QueryErrorGap);
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And A temporary table with a NOT NULL column is created
        var tableName = $"UD_TEST_NN_{Guid.NewGuid():N}";
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT NOT NULL)";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When NULL is inserted into the NOT NULL column
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id) VALUES (NULL)";

        // Then A proper error should be raised with vendor code 100072
        var exception = await Assert.ThrowsAnyAsync<DbException>(async () => await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false)).ConfigureAwait(false);
        DbExceptionAssert.HasDiagnostics(exception, errorCode: 100072);
    }

    // Scenario: should execute multiple queries sequentially on same connection
    [SnowflakeFact]
    public async Task ShouldExecuteMultipleQueriesSequentiallyOnSameConnectionAsync()
    {
        Skip.FutureMilestone("GetString on a scaled NUMBER column drops the decimal point (3.14 reads as \"314\") (SNOW-4483010)");
        var ct = TestContext.Current.CancellationToken;
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Multiple queries are executed sequentially
        using var cmd1 = connection.CreateCommand();
        cmd1.CommandText = "SELECT 1 AS val";
        using var reader1 = await cmd1.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then each query should return correct results
        Assert.True(reader1.Read());
        Assert.Equal(1, reader1.GetInt32(0));
        Assert.False(reader1.Read());
        reader1.Close();

        using var cmd2 = connection.CreateCommand();
        cmd2.CommandText = "SELECT 'hello' AS greeting";
        using var reader2 = await cmd2.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader2.Read());
        Assert.Equal((string?)"hello", (string?)reader2.GetString(0));
        Assert.False(reader2.Read());
        reader2.Close();

        using var cmd3 = connection.CreateCommand();
        cmd3.CommandText = "SELECT 3.14 AS pi_approx";
        using var reader3 = await cmd3.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader3.Read());
        Assert.Equal((string?)"3.14", (string?)reader3.GetString(0));
        Assert.False(reader3.Read());
    }
}
