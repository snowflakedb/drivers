using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class BooleanTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public BooleanTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast boolean values to appropriate type
    [SnowflakeFact]
    public void ShouldCastBooleanValuesToAppropriateType()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.Equal(typeof(bool), reader.GetFieldType(0));
        Assert.Equal(typeof(bool), reader.GetFieldType(1));
        Assert.Equal(typeof(bool), reader.GetFieldType(2));

        // And Values should match [TRUE, FALSE, TRUE]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True((bool)reader.GetValue(0));
        Assert.False((bool)reader.GetValue(1));
        Assert.True((bool)reader.GetValue(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select boolean literals
    [SnowflakeFact]
    public void ShouldSelectBooleanLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [TRUE, FALSE]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True((bool)reader.GetValue(0));
        Assert.False((bool)reader.GetValue(1));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values from literals
    [SnowflakeFact]
    public void ShouldHandleNullValuesFromLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [FALSE, NULL, TRUE, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.False((bool)reader.GetValue(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL for column 1");
        Assert.True((bool)reader.GetValue(2));
        Assert.True(reader.IsDBNull(3), "Expected NULL for column 3");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks from GENERATOR
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromGenerator()
    {
        const int rowCount = 1000000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT (seq8() % 2 = 0)::BOOLEAN AS val FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 500000 TRUE and 500000 FALSE values
        var trueCount = 0;
        var falseCount = 0;
        while (reader.Read())
        {
            var val = (bool)reader.GetValue(0);
            if (val)
                trueCount++;
            else
                falseCount++;
        }
        Assert.Equal(500000, trueCount);
        Assert.Equal(500000, falseCount);
    }

    // Scenario: should select boolean values from table
    [SnowflakeFact]
    public void ShouldSelectBooleanValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
        var tableName = $"UD_BOOL_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col1 BOOLEAN, col2 BOOLEAN, col3 BOOLEAN)";
        createCmd.ExecuteNonQuery();

        // And Row (TRUE, FALSE, TRUE) is inserted
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col1, col2, col3) VALUES (TRUE, FALSE, TRUE)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [TRUE, FALSE, TRUE]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True((bool)reader.GetValue(0));
        Assert.False((bool)reader.GetValue(1));
        Assert.True((bool)reader.GetValue(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values from table
    [SnowflakeFact]
    public void ShouldHandleNullValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BOOLEAN column exists
        var tableName = $"UD_BOOL_NULL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BOOLEAN)";
        createCmd.ExecuteNonQuery();

        // And Rows [NULL, TRUE, FALSE] are inserted
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (NULL), (TRUE), (FALSE)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col NULLS FIRST";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [NULL, TRUE, FALSE] in any order
        var results = new List<bool?>();
        while (reader.Read())
        {
            results.Add(reader.IsDBNull(0) ? null : (bool)reader.GetValue(0));
        }
        Assert.Equal(3, results.Count);
        Assert.Contains(null, results);
        Assert.Contains(true, results);
        Assert.Contains(false, results);
    }

    // Scenario: should download large result set with multiple chunks from table
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromTable()
    {
        const int rowCount = 1000000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
        var tableName = $"UD_BOOL_LARGE_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BOOLEAN)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT col FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 500000 TRUE and 500000 FALSE values
        var trueCount = 0;
        var falseCount = 0;
        while (reader.Read())
        {
            var val = (bool)reader.GetValue(0);
            if (val)
                trueCount++;
            else
                falseCount++;
        }
        Assert.Equal(500000, trueCount);
        Assert.Equal(500000, falseCount);
    }

    // Scenario: should select boolean using parameter binding
    [SnowflakeFact]
    public void ShouldSelectBooleanUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values [TRUE, FALSE, TRUE]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Boolean;
        param1.Value = true;
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Boolean;
        param2.Value = false;
        cmd.Parameters.Add(param2);

        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Boolean;
        param3.Value = true;
        cmd.Parameters.Add(param3);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [TRUE, FALSE, TRUE]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True((bool)reader.GetValue(0));
        Assert.False((bool)reader.GetValue(1));
        Assert.True((bool)reader.GetValue(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select null boolean using parameter binding
    [SnowflakeFact]
    public void ShouldSelectNullBooleanUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::BOOLEAN";

        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Boolean;
        param.Value = DBNull.Value;
        cmd.Parameters.Add(param);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert boolean using parameter binding
    [SnowflakeFact]
    public void ShouldInsertBooleanUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BOOLEAN column exists
        var tableName = $"UD_BOOL_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BOOLEAN)";
        createCmd.ExecuteNonQuery();

        // When Boolean values [TRUE, FALSE, NULL] are bulk-inserted using multirow binding
        bool?[] values = [true, false, null];
        foreach (var value in values)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Boolean;
            param.Value = value.HasValue ? value.Value : DBNull.Value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // Then SELECT should return the same values in any order
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        var results = new List<bool?>();
        while (reader.Read())
        {
            results.Add(reader.IsDBNull(0) ? null : (bool)reader.GetValue(0));
        }
        Assert.Equal(3, results.Count);
        Assert.Contains(null, results);
        Assert.Contains(true, results);
        Assert.Contains(false, results);
    }
}
