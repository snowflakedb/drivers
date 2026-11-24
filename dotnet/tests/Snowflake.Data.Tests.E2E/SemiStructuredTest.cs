using System.Data.Common;
using System.Text.Json.Nodes;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class SemiStructuredTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const int LargeResultSetSize = 20_000;

    public SemiStructuredTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast semi-structured values to appropriate type
    [SnowflakeFact]
    public async Task ShouldCastSemiStructuredValuesToAppropriateTypeAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then All values should be returned as appropriate type
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        for (var i = 0; i < 3; i++)
        {
            Assert.Equal(typeof(string), reader.GetFieldType(i));
            Assert.IsType<string>(reader.GetValue(i));
        }
        AssertJson("{\"a\":1}", reader.GetString(0));
        AssertJson("[1,2,3]", reader.GetString(1));
        AssertJson("{\"key\":\"val\"}", reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select semi-structured literals
    [SnowflakeFact]
    public async Task ShouldSelectSemiStructuredLiteralsAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("{\"key\":\"value\"}", reader.GetString(0));
        AssertJson("[10,20,30]", reader.GetString(1));
        AssertJson("{\"a\":1,\"b\":2}", reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select deeply nested semi-structured literals
    [SnowflakeFact]
    public async Task ShouldSelectDeeplyNestedSemiStructuredLiteralsAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain the expected nested value
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("{\"a\":{\"b\":[1,2,{\"c\":true}]}}", reader.GetString(0));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle NULL semi-structured values from literals
    [SnowflakeFact]
    public async Task ShouldHandleNullSemiStructuredValuesFromLiteralsAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then All columns should return null indicators
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        for (var i = 0; i < 3; i++)
        {
            Assert.True(reader.IsDBNull(i), $"Expected NULL at column {i}");
            Assert.Equal(DBNull.Value, reader.GetValue(i));
        }
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select semi-structured values from table
    [SnowflakeFact]
    public async Task ShouldSelectSemiStructuredValuesFromTableAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
        var tableName = await CreateTemporaryTableAsync(connection, "var_col VARIANT, obj_col OBJECT, arr_col ARRAY", ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"INSERT INTO {tableName} SELECT PARSE_JSON('{{\"x\":42}}'), OBJECT_CONSTRUCT('key', 'value'), ARRAY_CONSTRUCT(1, 2, 3)", ct).ConfigureAwait(false);

        // When Query "SELECT * FROM <table>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Data should contain the expected semi-structured values
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("{\"x\":42}", reader.GetString(0));
        AssertJson("{\"key\":\"value\"}", reader.GetString(1));
        AssertJson("[1,2,3]", reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle NULL semi-structured values from table
    [SnowflakeFact]
    public async Task ShouldHandleNullSemiStructuredValuesFromTableAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with VARIANT column exists containing NULLs and values
        var tableName = await CreateTemporaryTableAsync(connection, "col VARIANT, id INT", ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"INSERT INTO {tableName} SELECT PARSE_JSON(column2), column1 FROM VALUES (1, NULL), (2, '{{\"a\":1}}'), (3, NULL)", ct).ConfigureAwait(false);

        // When Query "SELECT * FROM <table>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT * FROM {tableName} ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain [NULL, {"a":1}, NULL]
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 1");
        Assert.True(reader.IsDBNull(0), "Expected NULL in row 1");
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 2");
        AssertJson("{\"a\":1}", reader.GetString(0));
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 3");
        Assert.True(reader.IsDBNull(0), "Expected NULL in row 3");
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly three rows");
    }

    // Scenario: should handle empty JSON containers
    [SnowflakeFact]
    public async Task ShouldHandleEmptyJsonContainersAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Each column should return a valid empty container
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("{}", reader.GetString(0));
        AssertJson("[]", reader.GetString(1));
        AssertJson("{}", reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle empty JSON array literal
    [SnowflakeFact]
    public async Task ShouldHandleEmptyJsonArrayLiteralAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON('[]')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('[]')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should be an empty JSON array
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("[]", reader.GetString(0));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should round-trip empty JSON containers through a table
    [SnowflakeFact]
    public async Task ShouldRoundTripEmptyJsonContainersThroughATableAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
        var tableName = await CreateTemporaryTableAsync(connection, "var_col VARIANT, obj_col OBJECT, arr_col ARRAY", ct).ConfigureAwait(false);
        await ExecuteNonQueryAsync(connection, $"INSERT INTO {tableName} SELECT PARSE_JSON('{{}}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()", ct).ConfigureAwait(false);

        // When Query "SELECT * FROM <table>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then All columns should return valid empty containers
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertJson("{}", reader.GetString(0));
        AssertJson("{}", reader.GetString(1));
        AssertJson("[]", reader.GetString(2));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle JSON with unicode content
    [SnowflakeFact]
    public async Task ShouldHandleJsonWithUnicodeContentAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query returning JSON with unicode characters is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{\"greeting\":\"こんにちは\",\"emoji\":\"⛄\"}')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should preserve the unicode characters
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var parsed = ParseObject(reader.GetString(0));
        Assert.Equal("こんにちは", (string?)parsed["greeting"]);
        Assert.Equal("⛄", (string?)parsed["emoji"]);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle JSON with unicode in keys
    [SnowflakeFact]
    public async Task ShouldHandleJsonWithUnicodeInKeysAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query returning JSON with unicode characters in keys is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON('{\"名前\":\"テスト\",\"données\":\"valeur\"}')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should preserve unicode keys and their associated values
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var parsed = ParseObject(reader.GetString(0));
        Assert.Equal("テスト", (string?)parsed["名前"]);
        Assert.Equal("valeur", (string?)parsed["données"]);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should download semi-structured data in multiple chunks
    [SnowflakeFact]
    public async Task ShouldDownloadSemiStructuredDataInMultipleChunksAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => {LargeResultSetSize})) v ORDER BY 1";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then All 20000 rows should be fetched and each should contain a value with "id" key
        var ids = new HashSet<long>();
        while (await reader.ReadAsync(ct).ConfigureAwait(false))
        {
            var parsed = ParseObject(reader.GetString(0));
            Assert.True(parsed.ContainsKey("id"), $"Row {ids.Count} has no \"id\" key");
            ids.Add((long)parsed["id"]!);
        }
        Assert.Equal(LargeResultSetSize, ids.Count);
        Assert.Equal(0L, ids.Min());
        Assert.Equal(LargeResultSetSize - 1L, ids.Max());
    }

    // Scenario: should select variant using parameter binding
    [SnowflakeFact]
    public async Task ShouldSelectVariantUsingParameterBindingAsync()
    {
        Skip.FutureMilestone("Parameter binding is not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON(?)";
        AddParameter(cmd, "1", DbType.String, "{\"bound\":true}");
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain a value with "bound" key
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var parsed = ParseObject(reader.GetString(0));
        Assert.True((bool)parsed["bound"]!);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select NULL variant using parameter binding
    [SnowflakeFact]
    public async Task ShouldSelectNullVariantUsingParameterBindingAsync()
    {
        Skip.FutureMilestone("Parameter binding is not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT PARSE_JSON(?)";
        AddParameter(cmd, "1", DbType.String, DBNull.Value);
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should be NULL
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL");
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should insert variant using parameter binding
    [SnowflakeFact]
    public async Task ShouldInsertVariantUsingParameterBindingAsync()
    {
        Skip.FutureMilestone("Parameter binding is not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        string[] jsonValues = ["{\"x\":1}", "[1,2,3]", "{\"nested\":{\"a\":true}}"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with VARIANT column exists
        var tableName = await CreateTemporaryTableAsync(connection, "col VARIANT, id INT", ct).ConfigureAwait(false);

        // When JSON values are inserted using parameter binding via PARSE_JSON(?)
        for (var i = 0; i < jsonValues.Length; i++)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} SELECT PARSE_JSON(?), ?";
            AddParameter(insertCmd, "1", DbType.String, jsonValues[i]);
            AddParameter(insertCmd, "2", DbType.Int32, i);
            await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }

        // Then SELECT should return the inserted JSON values
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT col FROM {tableName} ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        foreach (var expected in jsonValues)
        {
            Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), $"Expected a row for {expected}");
            AssertJson(expected, reader.GetString(0));
        }
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly three rows");
    }

    private static async Task ExecuteNonQueryAsync(ISnowflakeDbConnection connection, string sql, CancellationToken cancellationToken)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task<string> CreateTemporaryTableAsync(ISnowflakeDbConnection connection, string columns, CancellationToken cancellationToken)
    {
        var tableName = $"UD_SEMI_{Guid.NewGuid():N}";
        await ExecuteNonQueryAsync(connection, $"CREATE TEMPORARY TABLE {tableName} ({columns})", cancellationToken).ConfigureAwait(false);
        return tableName;
    }

    private static void AddParameter(DbCommand cmd, string name, DbType type, object value)
    {
        var param = cmd.CreateParameter();
        param.ParameterName = name;
        param.DbType = type;
        param.Value = value;
        cmd.Parameters.Add(param);
    }

    private static void AssertJson(string expected, string actual)
    {
        Assert.True(
            JsonNode.DeepEquals(JsonNode.Parse(expected), JsonNode.Parse(actual)),
            $"Expected JSON {expected}, got {actual}");
    }

    private static JsonObject ParseObject(string json) =>
        Assert.IsType<JsonObject>(JsonNode.Parse(json));
}
