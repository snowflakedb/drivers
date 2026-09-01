using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class BasicExecuteQueryTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public BasicExecuteQueryTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should execute simple SELECT returning single value
    [SnowflakeFact]
    public void ShouldExecuteSimpleSelectReturningSingleValue()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 1 AS value" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS value";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain value 1
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.IsDBNull(0), "Expected non-NULL value");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should execute SELECT returning multiple columns
    [SnowflakeFact]
    public void ShouldExecuteSelectReturningMultipleColumns()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain: | col1=1 | col2=hello | col3=3.14 |
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal((string?)"hello", (string?)reader.GetString(1));
        Assert.Equal((string?)"3.14", (string?)reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should execute SELECT returning multiple rows
    [SnowflakeFact]
    public void ShouldExecuteSelectReturningMultipleRows()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id";
        using var reader = cmd.ExecuteReader();

        // Then there are 5 numbered sequentially rows returned
        for (long i = 0; i < 5; i++)
        {
            Assert.True(reader.Read(), $"Expected row {i}");
            Assert.Equal(i, reader.GetInt64(0));
        }
        Assert.False(reader.Read(), "Expected exactly 5 rows");
    }

    // Scenario: should execute SELECT returning empty result set
    [SnowflakeFact]
    public void ShouldExecuteSelectReturningEmptyResultSet()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 1 WHERE 1=0" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 WHERE 1=0";
        using var reader = cmd.ExecuteReader();

        // Then the result set should be empty
        Assert.False(reader.Read(), "Expected empty result set");
    }

    // Scenario: should execute SELECT returning NULL values
    [SnowflakeFact]
    public void ShouldExecuteSelectReturningNullValues()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL AS col1, 42 AS col2, NULL AS col3";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain NULL for col1 and col3 and 42 for col2
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for col1");
        Assert.Equal(42, reader.GetInt32(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for col3");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should execute CREATE and DROP TABLE statements
    [SnowflakeFact]
    public void ShouldExecuteCreateAndDropTableStatements()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        var tableName = $"UD_TEST_DDL_{Guid.NewGuid():N}".Substring(0, 30);

        // When CREATE TABLE statement is executed
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TABLE {tableName} (id INT, name VARCHAR(100))";
        createCmd.ExecuteNonQuery();

        try
        {
            // Then the table should be created successfully
            using var verifyCmd = connection.CreateCommand();
            verifyCmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
            var count = Convert.ToInt64((object?)verifyCmd.ExecuteScalar());
            Assert.Equal(0L, count);

            // And DROP TABLE statement should complete successfully
            using var dropCmd = connection.CreateCommand();
            dropCmd.CommandText = $"DROP TABLE {tableName}";
            dropCmd.ExecuteNonQuery();

            // Verify table no longer exists
            using var verifyDropCmd = connection.CreateCommand();
            verifyDropCmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
            var ex = Assert.ThrowsAny<Exception>(() => verifyDropCmd.ExecuteScalar());
            Assert.Contains(tableName, ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            // Cleanup on failure
            using var cleanupCmd = connection.CreateCommand();
            cleanupCmd.CommandText = $"DROP TABLE IF EXISTS {tableName}";
            cleanupCmd.ExecuteNonQuery();
            throw;
        }
    }

    // Scenario: should execute INSERT and retrieve inserted data
    [SnowflakeFact]
    public void ShouldExecuteInsertAndRetrieveInsertedData()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table is created
        var tableName = $"UD_TEST_DML_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, value VARCHAR(50))";
        createCmd.ExecuteNonQuery();

        // When INSERT statement is executed to add rows
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id, value) VALUES (1, 'one'), (2, 'two'), (3, 'three')";
        insertCmd.ExecuteNonQuery();

        // And Query "SELECT id, value FROM {table} ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, value FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then the inserted data should be correctly returned
        Assert.True(reader.Read());
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal((string?)"one", (string?)reader.GetString(1));

        Assert.True(reader.Read());
        Assert.Equal(2, reader.GetInt32(0));
        Assert.Equal((string?)"two", (string?)reader.GetString(1));

        Assert.True(reader.Read());
        Assert.Equal(3, reader.GetInt32(0));
        Assert.Equal((string?)"three", (string?)reader.GetString(1));

        Assert.False(reader.Read(), "Expected exactly 3 rows");
    }

    // Scenario: should return error for invalid SQL syntax
    [SnowflakeFact]
    public void ShouldReturnErrorForInvalidSqlSyntax()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Invalid SQL "SELCT INVALID SYNTAX" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELCT INVALID SYNTAX";

        // Then An error should be returned
        Assert.ThrowsAny<Exception>(() => cmd.ExecuteReader());
    }

    // Scenario: should return proper error for NULL in NOT NULL column
    [SnowflakeFact]
    public void ShouldReturnProperErrorForNullInNotNullColumn()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with a NOT NULL column is created
        var tableName = $"UD_TEST_NN_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT NOT NULL)";
        createCmd.ExecuteNonQuery();

        // When NULL is inserted into the NOT NULL column
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id) VALUES (NULL)";

        // Then A proper error should be raised with vendor code 100072
        var ex = Assert.ThrowsAny<Exception>(() => insertCmd.ExecuteNonQuery());
        Assert.Contains("100072", ex.Message);
    }

    // Scenario: should execute multiple queries sequentially on same connection
    [SnowflakeFact]
    public void ShouldExecuteMultipleQueriesSequentiallyOnSameConnection()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Multiple queries are executed sequentially
        using var cmd1 = connection.CreateCommand();
        cmd1.CommandText = "SELECT 1 AS val";
        using var reader1 = cmd1.ExecuteReader();

        // Then each query should return correct results
        Assert.True(reader1.Read());
        Assert.Equal(1, reader1.GetInt32(0));
        Assert.False(reader1.Read());
        reader1.Close();

        using var cmd2 = connection.CreateCommand();
        cmd2.CommandText = "SELECT 'hello' AS greeting";
        using var reader2 = cmd2.ExecuteReader();

        Assert.True(reader2.Read());
        Assert.Equal((string?)"hello", (string?)reader2.GetString(0));
        Assert.False(reader2.Read());
        reader2.Close();

        using var cmd3 = connection.CreateCommand();
        cmd3.CommandText = "SELECT 3.14 AS pi_approx";
        using var reader3 = cmd3.ExecuteReader();

        Assert.True(reader3.Read());
        Assert.Equal((string?)"3.14", (string?)reader3.GetString(0));
        Assert.False(reader3.Read());
    }
}
