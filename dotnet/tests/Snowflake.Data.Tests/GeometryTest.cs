using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class GeometryTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public GeometryTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario Outline: should select <shape> geometry literal
    [SnowflakeTheory]
    [InlineData("Point", "TO_GEOMETRY('POINT(1820.12 890.56)')", "[1820.12,890.56]")]
    [InlineData("LineString", "TO_GEOMETRY('LINESTRING(0 0, 1 1, 2 2)')", "[[0,0],[1,1],[2,2]]")]
    [InlineData("Polygon", "TO_GEOMETRY('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))')", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]")]
    public void ShouldSelectShapeGeometryLiteral(string shape, string queryValue, string expectedCoordinatesJson)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // When Query "SELECT <query_value>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT {queryValue}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain a GeoJSON <shape> value
        Assert.True(reader.Read(), "Expected one row");
        AssertGeoJson(reader.GetString(0), shape, expectedCoordinatesJson);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should cast geometry to <expected_type> for <format> output format
    [SnowflakeTheory]
    [InlineData("GeoJSON", "str")]
    [InlineData("WKT", "str")]
    [InlineData("WKB", "bytearray")]
    [InlineData("EWKB", "bytearray")]
    [InlineData("EWKT", "str")]
    public void ShouldCastGeometryToExpectedTypeForFormatOutputFormat(string format, string expectedType)
    {
        if (expectedType == "bytearray")
            Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = $"ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = '{format}'";
        altCmd.ExecuteNonQuery();

        // When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')";
        using var reader = cmd.ExecuteReader();

        // Then Result should be returned as <expected_type> type
        Assert.True(reader.Read(), "Expected one row");
        Assert.False(reader.IsDBNull(0), "Expected non-null result");
        Assert.Equal(expectedType == "str" ? typeof(string) : typeof(byte[]), reader.GetFieldType(0));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select geometry values from table
    [SnowflakeFact]
    public void ShouldSelectGeometryValuesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // And Table with GEOMETRY column exists with WKT values
        var tableName = $"UD_GEOM_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOMETRY('POINT(1820.12 890.56)')
            UNION ALL SELECT 2, TO_GEOMETRY('LINESTRING(0 0, 1 1, 2 2)')
            UNION ALL SELECT 3, TO_GEOMETRY('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain the expected GeoJSON values
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(3, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Polygon", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]");

        Assert.False(reader.Read(), "Expected exactly three rows");
    }

    // Scenario: should handle NULL geometry values from table
    [SnowflakeFact]
    public void ShouldHandleNullGeometryValuesFromTable()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // And Table with GEOMETRY column exists containing NULLs and values
        var tableName = $"UD_GEOM_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $@"INSERT INTO {tableName}
            SELECT 1, TO_GEOMETRY('POINT(1820.12 890.56)')
            UNION ALL SELECT 2, NULL";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table> ORDER BY id" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [GeoJSON Point, NULL]
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(1, reader.GetInt32(0));
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(2, reader.GetInt32(0));
        Assert.True(reader.IsDBNull(1), "Expected NULL at row 2");

        Assert.False(reader.Read(), "Expected exactly two rows");
    }

    // Scenario: should download geometry data in multiple chunks
    [SnowflakeFact]
    public void ShouldDownloadGeometryDataInMultipleChunks()
    {
        const int rowCount = 20000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // When Query generating 20000 geometry points is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $@"SELECT id, TO_GEOMETRY('POINT(' || id || ' ' || id || ')') AS geom
            FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id
            FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))) ORDER BY id";
        using var reader = cmd.ExecuteReader();

        // Then All 20000 rows should be fetched with valid GeoJSON Point values
        var count = 0;
        while (reader.Read())
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
    [SnowflakeTheory]
    [InlineData("WKT string", "POINT(1820.12 890.56)", false)]
    [InlineData("NULL", null, true)]
    public void ShouldSelectGeometryUsingParameterBindingWithInputTypeValue(string inputType, string? boundValue, bool expectNull)
    {
        _ = inputType;
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_GEOMETRY(?)";
        var param = cmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.String;
        param.Value = boundValue ?? (object)DBNull.Value;
        cmd.Parameters.Add(param);
        using var reader = cmd.ExecuteReader();

        // Then Result should <expected_result>
        Assert.True(reader.Read(), "Expected one row");
        if (expectNull)
        {
            Assert.True(reader.IsDBNull(0), "Expected NULL result");
        }
        else
        {
            AssertGeoJson(reader.GetString(0), "Point", "[1820.12,890.56]");
        }
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert geometry using parameter binding
    [SnowflakeFact]
    public void ShouldInsertGeometryUsingParameterBinding()
    {
        Skip.FutureMilestone();

        string[] wktValues = ["POINT(1820.12 890.56)", "LINESTRING(0 0, 1 1, 2 2)", "POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))"];

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var altCmd = connection.CreateCommand();
        altCmd.CommandText = "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = 'GeoJSON'";
        altCmd.ExecuteNonQuery();

        // And Table with GEOMETRY column exists
        var tableName = $"UD_GEOM_BND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, geom GEOMETRY)";
        createCmd.ExecuteNonQuery();

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
            insertCmd.ExecuteNonQuery();
        }

        // Then SELECT should return the inserted GeoJSON values
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT id, geom FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        Assert.True(reader.Read());
        AssertGeoJson(reader.GetString(1), "Point", "[1820.12,890.56]");

        Assert.True(reader.Read());
        AssertGeoJson(reader.GetString(1), "LineString", "[[0,0],[1,1],[2,2]]");

        Assert.True(reader.Read());
        AssertGeoJson(reader.GetString(1), "Polygon", "[[[0,0],[4,0],[4,3],[0,3],[0,0]]]");

        Assert.False(reader.Read(), "Expected exactly three rows");
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
