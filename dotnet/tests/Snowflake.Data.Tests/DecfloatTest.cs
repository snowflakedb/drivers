using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class DecfloatTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public DecfloatTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast decfloat values to appropriate type
    [SnowflakeFact]
    public void ShouldCastDecfloatValuesToAppropriateType()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '12345678901234567890123456789012345678'::DECFLOAT" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '12345678901234567890123456789012345678'::DECFLOAT";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.Equal(typeof(string), reader.GetFieldType(0));

        Skip.FutureMilestone();
        // And Values should maintain full 38-digit precision
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(0m, reader.GetDecimal(0));
        Assert.Equal(123.456m, reader.GetDecimal(1));
        // 1.23e37 exceeds System.Decimal range — read as string
        Assert.Equal("12300000000000000000000000000000000000", reader.GetString(2));
        // 38-digit integer exceeds System.Decimal range — read as string
        Assert.Equal("12345678901234567890123456789012345678", reader.GetString(3));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select decfloat literals
    [SnowflakeFact]
    public void ShouldSelectDecfloatLiterals()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(0m, reader.GetDecimal(0));
        Assert.Equal(1.5m, reader.GetDecimal(1));
        Assert.Equal(-1.5m, reader.GetDecimal(2));
        Assert.Equal(123.456789m, reader.GetDecimal(3));
        Assert.Equal(-987.654321m, reader.GetDecimal(4));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle full 38-digit precision values from literals
    [SnowflakeFact]
    public void ShouldHandleFull38DigitPrecisionValuesFromLiterals()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT, '1.2345678901234567890123456789012345678E+100'::DECFLOAT, '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT '12345678901234567890123456789012345678'::DECFLOAT, '1.2345678901234567890123456789012345678E+100'::DECFLOAT, '1.2345678901234567890123456789012345678E-100'::DECFLOAT";
        using var reader = cmd.ExecuteReader();

        // Then Result should preserve all 38 digits for each value
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("12345678901234567890123456789012345678", reader.GetString(0));
        Assert.Equal("1.2345678901234567890123456789012345678e100", reader.GetString(1));
        Assert.Equal("1.2345678901234567890123456789012345678e-100", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should handle <case> exponent values from literals
    [SnowflakeTheory]
    [InlineData("'1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT", "1e16384", "1e-16383")]
    [InlineData("'-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT", "-1.234e8000", "9.876e-8000")]
    public void ShouldHandleCaseExponentValuesFromLiterals(string queryValues, string expectedFirst, string expectedSecond)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValues}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [<expected_values>]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(expectedFirst, reader.GetString(0));
        Assert.Equal(expectedSecond, reader.GetString(1));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values from literals
    [SnowflakeFact]
    public void ShouldHandleNullValuesFromLiterals()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL, 42.5, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal(42.5m, reader.GetDecimal(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks from GENERATOR
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromGenerator()
    {
        Skip.FutureMilestone();

        const int rowCount = 20000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::DECFLOAT AS id FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v ORDER BY 1";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal((decimal)count, reader.GetDecimal(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select decfloats from table
    [SnowflakeFact]
    public void ShouldSelectDecfloatsFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
        var tableName = $"UD_DECF_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (0), (123.456), (-789.012), (1.23e20), (-9.87e-15)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
        var results = new List<decimal>();
        while (reader.Read())
        {
            results.Add(reader.GetDecimal(0));
        }
        Assert.Equal(5, results.Count);
        Assert.Contains(-789.012m, results);
        Assert.Contains(-0.00000000000009877m, results);
        Assert.Contains(0m, results);
        Assert.Contains(123.456m, results);
        Assert.Contains(123000000000000000000m, results);
    }

    // Scenario: should handle full 38-digit precision values from table
    [SnowflakeFact]
    public void ShouldHandleFull38DigitPrecisionValuesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists with values [12345678901234567890123456789012345678, 1.2345678901234567890123456789012345678E+100, 1.2345678901234567890123456789012345678E-100]
        var tableName = $"UD_DECF_38D_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('12345678901234567890123456789012345678'), ('1.2345678901234567890123456789012345678E+100'), ('1.2345678901234567890123456789012345678E-100')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should preserve all 38 digits for each value
        var results = new List<string>();
        while (reader.Read())
        {
            results.Add(reader.GetString(0));
        }
        Assert.Equal(3, results.Count);
        Assert.Contains("1.2345678901234567890123456789012345678e-100", results);
        Assert.Contains("12345678901234567890123456789012345678", results);
        Assert.Contains("1.2345678901234567890123456789012345678e100", results);
    }

    // Scenario: should handle extreme exponent values from table
    [SnowflakeFact]
    public void ShouldHandleExtremeExponentValuesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
        var tableName = $"UD_DECF_EXP_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES ('1E+16384'), ('1E-16383'), ('-1.234E+8000'), ('9.876E-8000')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
        var results = new List<string>();
        while (reader.Read())
        {
            results.Add(reader.GetString(0));
        }
        Assert.Equal(4, results.Count);
        Assert.Contains("-1.234e8000", results);
        Assert.Contains("9.876e-8000", results);
        Assert.Contains("1e-16383", results);
        Assert.Contains("1e16384", results);
    }

    // Scenario: should handle NULL values from table
    [SnowflakeFact]
    public void ShouldHandleNullValuesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
        var tableName = $"UD_DECF_NULL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (NULL), (123.456), (NULL), (-789.012)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [NULL, 123.456, NULL, -789.012]
        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal(-789.012m, reader.GetDecimal(0));
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(123.456m, reader.GetDecimal(0));
        Assert.True(reader.Read(), "Expected row 2");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 2");
        Assert.True(reader.Read(), "Expected row 3");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 3");
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should download large result set with multiple chunks from table
    [SnowflakeFact]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromTable()
    {
        Skip.FutureMilestone();

        const int rowCount = 20000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists with values from 0 to 19999
        var tableName = $"UD_DECF_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal((decimal)count, reader.GetDecimal(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select decfloat using parameter binding
    [SnowflakeFact]
    public void ShouldSelectDecfloatUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT values [123.456, -789.012, 42.0]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Decimal;
        param1.Value = 123.456m;
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Decimal;
        param2.Value = -789.012m;
        cmd.Parameters.Add(param2);

        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Decimal;
        param3.Value = 42.0m;
        cmd.Parameters.Add(param3);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [123.456, -789.012, 42.0]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(123.456m, reader.GetDecimal(0));
        Assert.Equal(-789.012m, reader.GetDecimal(1));
        Assert.Equal(42.0m, reader.GetDecimal(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should select <case> decfloat using parameter binding
    [SnowflakeTheory]
    [InlineData("1E+16384", "1e16384")]
    [InlineData("-1.234E+8000", "-1.234e8000")]
    public void ShouldSelectCaseDecfloatUsingParameterBinding(string value, string expected)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::DECFLOAT" is executed with bound value <value>
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::DECFLOAT";

        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.String;
        param.Value = value;
        cmd.Parameters.Add(param);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain [<expected>]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(expected, reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert decfloat using parameter binding
    [SnowflakeFact]
    public void ShouldInsertDecfloatUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists
        var tableName = $"UD_DECF_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        // When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
        decimal?[] values = [0m, 123.456m, -789.012m, null];
        foreach (var value in values)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Decimal;
            param.Value = value.HasValue ? (object)value.Value : DBNull.Value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // Then SELECT should return the same exact values
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal(-789.012m, reader.GetDecimal(0));
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(0m, reader.GetDecimal(0));
        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(123.456m, reader.GetDecimal(0));
        Assert.True(reader.Read(), "Expected row 3");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 3");
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should insert extreme decfloat values using parameter binding
    [SnowflakeFact]
    public void ShouldInsertExtremeDecfloatValuesUsingParameterBinding()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with DECFLOAT column exists
        var tableName = $"UD_DECF_XTRM_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col DECFLOAT)";
        createCmd.ExecuteNonQuery();

        // When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
        string[] extremeValues = ["1e+16384", "1e-16383", "-1.234e8000"];
        foreach (var value in extremeValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.String;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then SELECT should return the same exact values
        var results = new List<string>();
        while (reader.Read())
        {
            results.Add(reader.GetString(0));
        }
        Assert.Equal(3, results.Count);
        Assert.Contains("-1.234e8000", results);
        Assert.Contains("1e-16383", results);
        Assert.Contains("1e16384", results);
    }
}
