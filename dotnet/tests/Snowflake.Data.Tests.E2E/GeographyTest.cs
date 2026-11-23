using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
[Trait("Category", "Conversion")]
public class GeographyTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private static readonly byte[] PointWkb = new byte[] { 0x01, 0x01, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x96, 0x5E, 0xC0, 0x66, 0x66, 0x66, 0x66, 0x66, 0xC6, 0x42, 0x40 };
    private static readonly byte[] PointEwkb = new byte[] { 0x01, 0x01, 0x00, 0x00, 0x20, 0xE6, 0x10, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x96, 0x5E, 0xC0, 0x66, 0x66, 0x66, 0x66, 0x66, 0xC6, 0x42, 0x40 };

    public GeographyTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario Outline: should select <shape> geography literal
    [SnowflakeTheory]
    [InlineData("Point", "TO_GEOGRAPHY('POINT(-122.35 37.55)')", "[-122.35,37.55]")]
    [InlineData("LineString", "TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')", "[[0,0],[1,1],[2,2]]")]
    [InlineData("Polygon", "TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')", "[[[0,0],[10,0],[10,10],[0,10],[0,0]]]")]
    public async Task ShouldSelectShapeGeographyLiteralAsync(string shape, string queryValue, string expectedCoordinatesJson)
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT <query_value>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValue}";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain a GeoJSON <shape> value
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        AssertGeoJson(reader.GetString(0), shape, expectedCoordinatesJson);
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select geography from GeoJSON input
    [SnowflakeFact]
    public async Task ShouldSelectGeographyFromGeoJsonInputAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOGRAPHY('{\"type\":\"Point\",\"coordinates\":[-122.35,37.55]}')";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain a GeoJSON Point value
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        AssertGeoJson(reader.GetString(0), "Point", "[-122.35,37.55]");
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should cast geography to <expected_type> for <format> output format
    [SnowflakeTheory]
    [InlineData("GeoJSON", "str")]
    [InlineData("WKT", "str")]
    [InlineData("WKB", "bytearray")]
    [InlineData("EWKT", "str")]
    [InlineData("EWKB", "bytearray")]
    public async Task ShouldCastGeographyToExpectedTypeForFormatOutputFormatAsync(string format, string expectedType)
    {
        if (format is "GeoJSON" or "WKB" or "EWKB")
        {
            Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");
        }

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = $"ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = '{format}'";
        await altCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should be returned as <expected_type> type
        var expectedClrType = expectedType == "bytearray" ? typeof(byte[]) : typeof(string);
        Assert.Equal(expectedClrType, reader.GetFieldType(0));
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        var value = reader.GetValue(0);
        Assert.IsType(expectedClrType, value);
        switch (format)
        {
            case "GeoJSON":
                AssertGeoJson((string)value, "Point", "[-122.35,37.55]");
                break;
            case "WKT":
                Assert.Equal("POINT(-122.35 37.55)", (string)value);
                break;
            case "EWKT":
                Assert.Equal("SRID=4326;POINT(-122.35 37.55)", (string)value);
                break;
            case "WKB":
                Assert.Equal(PointWkb, (byte[])value);
                break;
            case "EWKB":
                Assert.Equal(PointEwkb, (byte[])value);
                break;
            default:
                Assert.Fail($"Unhandled format {format}");
                break;
        }
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select geography values from table
    [SnowflakeFact]
    public async Task ShouldSelectGeographyValuesFromTableAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // And Table with GEOGRAPHY column exists with WKT values
        var tableName = $"UD_GEO_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geo GEOGRAPHY)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)')
            UNION ALL SELECT 2, TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geo FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain the expected GeoJSON values
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[-122.35,37.55]");

        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario: should handle NULL geography values from table
    [SnowflakeFact]
    public async Task ShouldHandleNullGeographyValuesFromTableAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // And Table with GEOGRAPHY column exists containing NULLs and values
        var tableName = $"UD_GEO_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geo GEOGRAPHY)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOGRAPHY('POINT(-122.35 37.55)')
            UNION ALL SELECT 2, NULL";
        await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geo FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should contain [GeoJSON Point, NULL]
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[-122.35,37.55]");

        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL at row 2");

        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario: should download geography data in multiple chunks
    [SnowflakeFact]
    public async Task ShouldDownloadGeographyDataInMultipleChunksAsync()
    {
        const int rowCount = 20000;

        Skip.FutureMilestone("TODO SNOW-4483010 future milestone DataConversionNotImplemented");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query generating 20000 geography points is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $@"SELECT id, TO_GEOGRAPHY('POINT(' || (MOD(id, 360) - 180) || ' ' || (MOD(id, 180) - 90) || ')') AS geo
            FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id
            FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))) ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then All 20000 rows should be fetched with valid GeoJSON Point values
        var count = 0;
        while (await reader.ReadAsync().ConfigureAwait(false))
        {
            Assert.Equal(count, reader.GetInt64(0));

            var geoJson = reader.GetString(1);
            using var doc = JsonDocument.Parse(geoJson);
            var geo = doc.RootElement;
            Assert.Equal("Point", geo.GetProperty("type").GetString());
            var coordinates = geo.GetProperty("coordinates");
            Assert.Equal(2, coordinates.GetArrayLength());
            var expectedLon = (count % 360) - 180.0;
            var expectedLat = (count % 180) - 90.0;
            Assert.Equal(expectedLon, coordinates[0].GetDouble(), 9);
            Assert.Equal(expectedLat, coordinates[1].GetDouble(), 9);

            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select geography using parameter binding with <input_type> value
    [SnowflakeTheory]
    [InlineData("WKT string", "POINT(-122.35 37.55)", false)]
    [InlineData("NULL", null, true)]
    public async Task ShouldSelectGeographyUsingParameterBindingWithInputTypeValueAsync(string inputType, string? boundValue, bool expectNull)
    {
        _ = inputType;
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOGRAPHY(?)";
        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.String;
        param.Value = boundValue ?? (object)DBNull.Value;
        cmd.Parameters.Add(param);
        using var reader = await cmd.ExecuteReaderAsync().ConfigureAwait(false);

        // Then Result should <expected_result>
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected one row");
        if (expectNull)
        {
            Assert.True(reader.IsDBNull(0), "Expected NULL result");
        }
        else
        {
            AssertGeoJson(reader.GetString(0), "Point", "[-122.35,37.55]");
        }
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should insert geography using parameter binding
    [SnowflakeFact]
    public async Task ShouldInsertGeographyUsingParameterBindingAsync()
    {
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");

        string[] wktValues = ["POINT(-122.35 37.55)", "LINESTRING(0 0, 1 1, 2 2)", "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        using var fmtCmd = connection.CreateCommand();
        fmtCmd.CommandText = "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = 'GeoJSON'";
        await fmtCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // And Table with GEOGRAPHY column exists
        var tableName = $"UD_GEO_BND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geo GEOGRAPHY)";
        await createCmd.ExecuteNonQueryAsync().ConfigureAwait(false);

        // When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
        for (var i = 0; i < wktValues.Length; i++)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} SELECT {i + 1}, TO_GEOGRAPHY(?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.String;
            param.Value = wktValues[i];
            insertCmd.Parameters.Add(param);
            await insertCmd.ExecuteNonQueryAsync().ConfigureAwait(false);
        }

        // Then SELECT should return the inserted GeoJSON values
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geo FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync().ConfigureAwait(false);

        Assert.True(await reader.ReadAsync().ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "Point", "[-122.35,37.55]");

        Assert.True(await reader.ReadAsync().ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.True(await reader.ReadAsync().ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "Polygon", "[[[0,0],[10,0],[10,10],[0,10],[0,0]]]");

        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly three rows");
    }

    private static void AssertGeoJson(string value, string expectedType, string expectedCoordinatesJson)
    {
        using var doc = JsonDocument.Parse(value);
        var geo = doc.RootElement;
        Assert.Equal(expectedType, geo.GetProperty("type").GetString());

        using var expectedDoc = JsonDocument.Parse(expectedCoordinatesJson);
        AssertCoordinatesEqual(expectedDoc.RootElement, geo.GetProperty("coordinates"));
    }

    private static void AssertCoordinatesEqual(JsonElement expected, JsonElement actual)
    {
        Assert.Equal(expected.GetArrayLength(), actual.GetArrayLength());
        for (var i = 0; i < expected.GetArrayLength(); i++)
        {
            var expectedItem = expected[i];
            var actualItem = actual[i];
            if (expectedItem.ValueKind == JsonValueKind.Array)
            {
                Assert.Equal(JsonValueKind.Array, actualItem.ValueKind);
                AssertCoordinatesEqual(expectedItem, actualItem);
            }
            else
            {
                Assert.Equal(JsonValueKind.Number, actualItem.ValueKind);
                Assert.Equal(expectedItem.GetDouble(), actualItem.GetDouble(), 9);
            }
        }
    }
}
