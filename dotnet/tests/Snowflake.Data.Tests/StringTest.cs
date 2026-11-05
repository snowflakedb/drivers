using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class StringTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    // Corner case string values matching the feature file definition
    private static readonly (string Sql, string? Expected)[] CornerCaseValues =
    [
        ("''", ""),
        ("'X'", "X"),
        ("'   '", "   "),
        ("'\t'", "\t"),
        ("'\n'", "\n"),
        ("'\u26c4'", "\u26c4"),
        ("'日本語テスト'", "日本語テスト"),
        ("''''", "'"),
        ("'\\\\'", "\\"),
        ("NULL", null),
        ("'y\u0306es'", "y\u0306es"),
        ("'\U0001D11E'", "\U0001D11E"),
    ];

    public StringTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast string values to appropriate type for string and synonyms
    [SnowflakeTheory]
    [InlineData("VARCHAR")]
    [InlineData("CHAR")]
    [InlineData("CHARACTER")]
    [InlineData("NCHAR")]
    [InlineData("STRING")]
    [InlineData("TEXT")]
    [InlineData("VARCHAR2")]
    [InlineData("NVARCHAR")]
    [InlineData("NVARCHAR2")]
    [InlineData("CHAR VARYING")]
    [InlineData("NCHAR VARYING")]
    public void ShouldCastStringValuesToAppropriateTypeForStringAndSynonyms(string stringType)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 'hello'::<type>, 'Hello World'::<type>, '日本語テスト'::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 'hello'::{stringType}, 'Hello World'::{stringType}, '日本語テスト'::{stringType}";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate type
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("hello", reader.GetString(0));
        Assert.Equal("Hello World", reader.GetString(1));
        Assert.Equal("日本語テスト", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select hardcoded string literals
    [SnowflakeFact]
    public void ShouldSelectHardcodedStringLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain:
        Assert.True(reader.Read(), "Expected one row");
        // | str1  | str2        | str3                  |
        // | hello | Hello World | Snowflake Driver Test |
        Assert.Equal("hello", reader.GetString(0));
        Assert.Equal("Hello World", reader.GetString(1));
        Assert.Equal("Snowflake Driver Test", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select string literals with corner case values
    [SnowflakeFact]
    public void ShouldSelectStringLiteralsWithCornerCaseValues()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query selecting corner case string literals is executed
        var selectExpressions = string.Join(", ", CornerCaseValues.Select((v, i) => $"{v.Sql} AS col{i}"));
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {selectExpressions}";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain expected corner case string values
        Assert.True(reader.Read(), "Expected one row");
        for (var i = 0; i < CornerCaseValues.Length; i++)
        {
            var (_, expected) = CornerCaseValues[i];
            if (expected is null)
                Assert.True(reader.IsDBNull(i), $"Expected NULL at column {i}");
            else
                Assert.Equal(expected, reader.GetString(i));
        }
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select hardcoded string values from table
    [SnowflakeFact]
    public void ShouldSelectHardcodedStringValuesFromTable()
    {
        Skip.FutureMilestone();

        string[] hardcodedValues = ["hello", "Hello World", "Snowflake Driver Test"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with VARCHAR column is created
        var tableName = $"UD_STR_HCV_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (ord INT, val VARCHAR)";
        createCmd.ExecuteNonQuery();

        // And The table is populated with string values
        var insertValues = string.Join(", ", hardcodedValues.Select((v, i) => $"({i}, '{v}')"));
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (ord, val) VALUES {insertValues}";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT val FROM {tableName} ORDER BY ord";
        using var reader = selectCmd.ExecuteReader();

        // Then the result should contain the inserted hardcoded string values
        for (var i = 0; i < hardcodedValues.Length; i++)
        {
            Assert.True(reader.Read(), $"Expected row {i}");
            Assert.Equal(hardcodedValues[i], reader.GetString(0));
        }
        Assert.False(reader.Read(), $"Expected exactly {hardcodedValues.Length} rows");
    }

    // Scenario: should select corner case string values from table
    [SnowflakeFact]
    public void ShouldSelectCornerCaseStringValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with VARCHAR column is created
        var tableName = $"UD_STR_CCV_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (ord INT, val VARCHAR)";
        createCmd.ExecuteNonQuery();

        // And The table is populated with corner case string values
        var insertPairs = CornerCaseValues
            .Select((v, i) => $"({i}, {v.Sql})")
            .ToArray();
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (ord, val) VALUES {string.Join(", ", insertPairs)}";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT val FROM {tableName} ORDER BY ord";
        using var reader = selectCmd.ExecuteReader();

        // Then the result should contain the inserted corner case string values
        for (var i = 0; i < CornerCaseValues.Length; i++)
        {
            var (_, expected) = CornerCaseValues[i];
            Assert.True(reader.Read(), $"Expected row {i}");
            if (expected is null)
                Assert.True(reader.IsDBNull(0), $"Expected NULL at row {i}");
            else
                Assert.Equal(expected, reader.GetString(0));
        }
        Assert.False(reader.Read(), $"Expected exactly {CornerCaseValues.Length} rows");
    }

    // Scenario: should insert and select back hardcoded string values using parameter binding
    [SnowflakeFact]
    public void ShouldInsertAndSelectBackHardcodedStringValuesUsingParameterBinding()
    {
        Skip.FutureMilestone();

        const string bindValue = "Test binding value 日本語";

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with VARCHAR column is created
        var tableName = $"UD_STR_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (val VARCHAR)";
        createCmd.ExecuteNonQuery();

        // When String value 'Test binding value 日本語' is inserted using parameter binding
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (val) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.String;
        param.Value = bindValue;
        insertCmd.Parameters.Add(param);
        insertCmd.ExecuteNonQuery();

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT val FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then the result should contain the bound string value 'Test binding value 日本語'
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(bindValue, reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select string literals using parameter binding
    [SnowflakeFact]
    public void ShouldSelectStringLiteralsUsingParameterBinding()
    {
        Skip.FutureMilestone();

        string[] bindValues = ["hello", "Hello World", "日本語テスト"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed with bound string values ['hello', 'Hello World', '日本語テスト']
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR";
        for (var i = 0; i < bindValues.Length; i++)
        {
            var p = cmd.CreateParameter();
            p.ParameterName = (i + 1).ToString();
            p.DbType = DbType.String;
            p.Value = bindValues[i];
            cmd.Parameters.Add(p);
        }
        using var reader = cmd.ExecuteReader();

        // Then the result should contain:
        Assert.True(reader.Read(), "Expected one row");
        // | col1  | col2        | col3       |
        // | hello | Hello World | 日本語テスト |
        Assert.Equal("hello", reader.GetString(0));
        Assert.Equal("Hello World", reader.GetString(1));
        Assert.Equal("日本語テスト", reader.GetString(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select corner case string values using parameter binding
    [SnowflakeTheory]
    [MemberData(nameof(CornerCaseBindingData))]
    public void ShouldSelectCornerCaseStringValuesUsingParameterBinding(string? expected)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::VARCHAR";
        var p = cmd.CreateParameter();
        p.ParameterName = "1";
        p.DbType = DbType.String;
        p.Value = expected ?? (object)DBNull.Value;
        cmd.Parameters.Add(p);
        using var reader = cmd.ExecuteReader();

        // Then the result should match the bound corner case value
        Assert.True(reader.Read(), "Expected one row");
        if (expected is null)
            Assert.True(reader.IsDBNull(0), "Expected NULL for null corner case");
        else
            Assert.Equal(expected, reader.GetString(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object?[]> CornerCaseBindingData() =>
        CornerCaseValues.Select(v => new object?[] { v.Expected });

    // Scenario: should download string data in multiple chunks
    [SnowflakeFact]
    public void ShouldDownloadStringDataInMultipleChunks()
    {
        const int rowCount = 10000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v ORDER BY id";
        using var reader = cmd.ExecuteReader();

        // Then there are 10000 rows returned and all string values should match the generated values in order
        var count = 0;
        while (reader.Read())
        {
            var id = reader.GetInt64(0);
            var strVal = reader.GetString(1);
            Assert.Equal(id.ToString(), strVal);
            count++;
        }
        Assert.Equal(rowCount, count);
    }
}
