using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
[Trait("Category", "Conversion")]
public class GeometryTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private static readonly byte[] PointWkb = new byte[] { 0x01, 0x01, 0x00, 0x00, 0x00, 0x14, 0xAE, 0x47, 0xE1, 0x7A, 0x70, 0x9C, 0x40, 0x14, 0xAE, 0x47, 0xE1, 0x7A, 0xD4, 0x8B, 0x40 };
    private static readonly byte[] PointEwkb = new byte[] { 0x01, 0x01, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x14, 0xAE, 0x47, 0xE1, 0x7A, 0x70, 0x9C, 0x40, 0x14, 0xAE, 0x47, 0xE1, 0x7A, 0xD4, 0x8B, 0x40 };

    public GeometryTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario Outline: should select <shape> geometry literal
    [SnowflakeTheory(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    [InlineData("Point", "TO_GEOMETRY('POINT(1820.12 890.56)')", "[1820.12,890.56]")]
    [InlineData("LineString", "TO_GEOMETRY('LINESTRING(0 0, 1 1, 2 2)')", "[[0,0],[1,1],[2,2]]")]
    [InlineData("Polygon", "TO_GEOMETRY('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))')", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]")]
    public async Task ShouldSelectShapeGeometryLiteralAsync(string shape, string queryValue, string expectedCoordinatesJson)
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query "SELECT <query_value>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValue}";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain a GeoJSON <shape> value
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        AssertGeoJson(reader.GetString(0), shape, expectedCoordinatesJson);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario Outline: should cast geometry to <expected_type> for <format> output format
    [SnowflakeTheory(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    [InlineData("GeoJSON", "str")]
    [InlineData("WKT", "str")]
    [InlineData("WKB", "bytearray")]
    [InlineData("EWKB", "bytearray")]
    [InlineData("EWKT", "str")]
    public async Task ShouldCastGeometryToExpectedTypeForFormatOutputFormatAsync(string format, string expectedType)
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = $"ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = '{format}'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should be returned as <expected_type> type
        var expectedClrType = expectedType == "bytearray" ? typeof(byte[]) : typeof(string);
        Assert.Equal(expectedClrType, reader.GetFieldType(0));
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var value = reader.GetValue(0);
        Assert.IsType(expectedClrType, value);
        switch (format)
        {
            case "GeoJSON":
                AssertGeoJson((string)value, "Point", "[1820.12,890.56]");
                break;
            case "WKT":
                Assert.Equal("POINT(1820.12 890.56)", (string)value);
                break;
            case "EWKT":
                Assert.Equal("SRID=0;POINT(1820.12 890.56)", (string)value);
                break;
            case "WKB":
                Assert.Equal(PointWkb, (byte[])value);
                break;
            case "EWKB":
                Assert.Equal(PointEwkb, (byte[])value);
                break;
        }
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should select geometry values from table
    [SnowflakeFact(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    public async Task ShouldSelectGeometryValuesFromTableAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Table with GEOMETRY column exists with WKT values
        var tableName = $"UD_GEOM_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOMETRY('POINT(1820.12 890.56)')
            UNION ALL SELECT 2, TO_GEOMETRY('LINESTRING(0 0, 1 1, 2 2)')
            UNION ALL SELECT 3, TO_GEOMETRY('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))')";
        await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain the expected GeoJSON values
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 3");
        Assert.Equal(3, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Polygon", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]");

        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly three rows");
    }

    // Scenario: should handle NULL geometry values from table
    [SnowflakeFact(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    public async Task ShouldHandleNullGeometryValuesFromTableAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Table with GEOMETRY column exists containing NULLs and values
        var tableName = $"UD_GEOM_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOMETRY('POINT(1820.12 890.56)')
            UNION ALL SELECT 2, NULL";
        await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should contain [GeoJSON Point, NULL]
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL at row 2");

        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly two rows");
    }

    // Scenario: should download geometry data in multiple chunks
    [SnowflakeFact(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    public async Task ShouldDownloadGeometryDataInMultipleChunksAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        const int rowCount = 20000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query generating 20000 geometry points is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $@"SELECT id, TO_GEOMETRY('POINT(' || id || ' ' || id || ')') AS geom
            FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id
            FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))) ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then All 20000 rows should be fetched with valid GeoJSON Point values
        var count = 0;
        while (await reader.ReadAsync(ct).ConfigureAwait(false))
        {
            Assert.Equal(count, reader.GetInt64(0));

            var geoJson = reader.GetString(1);
            using var doc = JsonDocument.Parse(geoJson);
            var geo = doc.RootElement;
            Assert.Equal("Point", geo.GetProperty("type").GetString());
            var coordinates = geo.GetProperty("coordinates");
            Assert.Equal(2, coordinates.GetArrayLength());
            Assert.Equal(count, coordinates[0].GetDouble(), 9);
            Assert.Equal(count, coordinates[1].GetDouble(), 9);

            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario Outline: should select geometry using parameter binding with <input_type> value
    [SnowflakeTheory(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    [InlineData("WKT string", "POINT(1820.12 890.56)", false)]
    [InlineData("NULL", null, true)]
    public async Task ShouldSelectGeometryUsingParameterBindingWithInputTypeValueAsync(string inputType, string? boundValue, bool expectNull)
    {
        _ = inputType;
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOMETRY(?)";
        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.String;
        param.Value = boundValue ?? (object)DBNull.Value;
        cmd.Parameters.Add(param);
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then Result should <expected_result>
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        if (expectNull)
        {
            Assert.True(reader.IsDBNull(0), "Expected NULL result");
        }
        else
        {
            AssertGeoJson(reader.GetString(0), "Point", "[1820.12,890.56]");
        }
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should insert geometry using parameter binding
    [SnowflakeFact(Skip = "TODO SNOW-4483010 future milestone DataConversionNotImplemented")]
    public async Task ShouldInsertGeometryUsingParameterBindingAsync()
    {
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        string[] wktValues = ["POINT(1820.12 890.56)", "LINESTRING(0 0, 1 1, 2 2)", "POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        await altCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Table with GEOMETRY column exists
        var tableName = $"UD_GEOM_BND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
        for (var i = 0; i < wktValues.Length; i++)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} SELECT {i + 1}, TO_GEOMETRY(?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.String;
            param.Value = wktValues[i];
            insertCmd.Parameters.Add(param);
            await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }

        // Then SELECT should return the inserted GeoJSON values
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false));
        AssertGeoJson(reader.GetString(1), "Polygon", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]");

        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly three rows");
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
