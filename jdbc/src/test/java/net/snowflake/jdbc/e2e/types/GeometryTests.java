package net.snowflake.jdbc.e2e.types;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSetMetaData;
import java.sql.Types;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Stream;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.jdbc.utils.SkipForJSONResultSet;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * End-to-end coverage for Snowflake GEOMETRY.
 *
 * <p>GEOMETRY values default to GeoJSON strings. Output format is controlled by {@code
 * GEOMETRY_OUTPUT_FORMAT}: GeoJSON, WKT, and EWKT arrive as {@code String}; WKB and EWKB as {@code
 * byte[]}.
 */
class GeometryTests extends SnowflakeIntegrationTestBase implements WithGeoAssertions {

  private static final int LARGE_RESULT_SET_SIZE = 20_000;

  private static final String POINT_WKT = "POINT(1820.12 890.56)";
  private static final String LINESTRING_WKT = "LINESTRING(0 0, 1 1, 2 2)";
  private static final String POLYGON_WKT = "POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))";

  private static Stream<Arguments> geometryLiteralCases() {
    return Stream.of(
        Arguments.of(
            "Point", "TO_GEOMETRY('" + POINT_WKT + "')", pointCoordinates(1820.12, 890.56)),
        Arguments.of(
            "LineString", "TO_GEOMETRY('" + LINESTRING_WKT + "')", lineStringCoordinates()),
        Arguments.of("Polygon", "TO_GEOMETRY('" + POLYGON_WKT + "')", polygonCoordinates()));
  }

  private static Stream<Arguments> geometryOutputFormatCases() {
    return Stream.of(
        Arguments.of("GeoJSON", String.class),
        Arguments.of("WKT", String.class),
        Arguments.of("WKB", byte[].class),
        Arguments.of("EWKT", String.class),
        Arguments.of("EWKB", byte[].class));
  }

  private static Stream<Arguments> geometryBindingSelectCases() {
    return Stream.of(Arguments.of(POINT_WKT), Arguments.of((String) null));
  }

  // ==========================================================================
  // SELECT LITERALS
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geometryLiteralCases")
  void shouldSelectShapeGeometryLiteral(
      String shape, String queryValue, JSONArray expectedCoordinates) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_value>" is executed
    withQueryResult(
        connection,
        "SELECT " + queryValue,
        resultSet -> {
          // Then Result should contain a GeoJSON <shape> value
          assertTrue(resultSet.next());
          assertGeoJson(resultSet.getString(1), shape, expectedCoordinates);
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // TYPE CASTING PER OUTPUT FORMAT
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geometryOutputFormatCases")
  void shouldCastGeometryToExpectedTypeForFormatOutputFormat(
      String outputFormat, Class<?> expectedType) throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openConnection()) {
      // And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
      execute(connection, "ALTER SESSION SET GEOMETRY_OUTPUT_FORMAT = '" + outputFormat + "'");

      // When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
      withQueryResult(
          connection,
          "SELECT TO_GEOMETRY('" + POINT_WKT + "')",
          resultSet -> {
            // Then Result should be returned as <expected_type> type
            assertTrue(resultSet.next());
            Object value = resultSet.getObject(1);
            assertInstanceOf(expectedType, value);
            assertFalse(resultSet.wasNull());
            if (expectedType == String.class) {
              assertFalse(((String) value).isEmpty());
            } else {
              assertTrue(((byte[]) value).length > 0);
            }

            ResultSetMetaData meta = resultSet.getMetaData();
            SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
            int expectedJdbcType;
            boolean expectedCaseSensitive;
            if (expectedType == byte[].class) {
              // BD#53: legacy reports BINARY for WKB/EWKB metadata; UD reports VARCHAR.
              expectedJdbcType = isNewDriver() ? Types.VARCHAR : Types.BINARY;
              expectedCaseSensitive = isNewDriver();
            } else {
              expectedJdbcType = Types.VARCHAR;
              expectedCaseSensitive = true;
            }
            assertAll(
                "geometry metadata for " + outputFormat,
                () -> assertEquals(1, meta.getColumnCount(), "column count"),
                () -> assertFalse(sfMeta.getQueryID().isEmpty(), "query id"),
                () -> assertEquals(expectedJdbcType, meta.getColumnType(1), "column type"),
                () -> assertEquals("GEOMETRY", meta.getColumnTypeName(1), "column type name"),
                () -> assertFalse(meta.isSigned(1), "signed"),
                () ->
                    assertEquals(expectedCaseSensitive, meta.isCaseSensitive(1), "case sensitive"));

            assertFalse(resultSet.next());
          });
    }
  }

  // ==========================================================================
  // TABLE OPERATIONS
  // ==========================================================================

  @Test
  void shouldSelectGeometryValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOMETRY column exists with WKT values
    String tableName = createTempTable(connection, "ud_geom_table_", "id INT, geo GEOMETRY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT 1, TO_GEOMETRY('"
            + POINT_WKT
            + "') UNION ALL SELECT 2, TO_GEOMETRY('"
            + LINESTRING_WKT
            + "') UNION ALL SELECT 3, TO_GEOMETRY('"
            + POLYGON_WKT
            + "')");

    // When Query "SELECT * FROM <table> ORDER BY id" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY id",
        resultSet -> {
          // Then Result should contain the expected GeoJSON values
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(1820.12, 890.56));
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertEquals(2, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertGeoJson(resultSet.getString(2), "LineString", lineStringCoordinates());
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertEquals(3, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertGeoJson(resultSet.getString(2), "Polygon", polygonCoordinates());
          assertFalse(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleNullGeometryValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOMETRY column exists containing NULLs and values
    String tableName = createTempTable(connection, "ud_geom_null_", "id INT, geo GEOMETRY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT 1, TO_GEOMETRY('"
            + POINT_WKT
            + "') UNION ALL SELECT 2, NULL");

    // When Query "SELECT * FROM <table> ORDER BY id" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY id",
        resultSet -> {
          // Then Result should contain [GeoJSON Point, NULL]
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(1820.12, 890.56));
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertEquals(2, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertNull(resultSet.getString(2));
          assertTrue(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // MULTIPLE CHUNKS DOWNLOADING
  // ==========================================================================

  @Test
  @SkipForJSONResultSet("Large geometry result sets require Arrow chunk download")
  void shouldDownloadGeometryDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query generating 20000 geometry points is executed
    String sql =
        "SELECT id, TO_GEOMETRY('POINT(' || id || ' ' || id || ')') AS geo "
            + "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))) ORDER BY id";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then All 20000 rows should be fetched with valid GeoJSON Point values
          int rowCount = 0;
          while (resultSet.next()) {
            assertEquals(rowCount, resultSet.getLong(1), "ID mismatch at row " + rowCount);
            assertFalse(resultSet.wasNull(), "ID should not be NULL at row " + rowCount);

            String geoJson = resultSet.getString(2);
            assertFalse(resultSet.wasNull(), "GEO column should not be NULL at row " + rowCount);
            assertFalse(geoJson.isEmpty(), "GEO column should not be empty at row " + rowCount);

            JSONObject geo = new JSONObject(geoJson);
            assertEquals("Point", geo.getString("type"));
            JSONArray coordinates = geo.getJSONArray("coordinates");
            assertEquals(2, coordinates.length());
            assertEquals((double) rowCount, coordinates.getDouble(0), 1e-9);
            assertEquals((double) rowCount, coordinates.getDouble(1), 1e-9);

            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected geometry row count");
        });
  }

  // ==========================================================================
  // PARAMETER BINDING
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geometryBindingSelectCases")
  void shouldSelectGeometryUsingParameterBindingWithInputTypeValue(String bindValue)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
    withPreparedQueryResult(
        connection,
        "SELECT TO_GEOMETRY(?)",
        ps -> {
          if (bindValue == null) {
            ps.setNull(1, Types.VARCHAR);
          } else {
            ps.setString(1, bindValue);
          }
        },
        resultSet -> {
          // Then Result should <expected_result>
          assertTrue(resultSet.next());
          if (bindValue == null) {
            assertNull(resultSet.getString(1));
            assertTrue(resultSet.wasNull());
          } else {
            assertGeoJson(resultSet.getString(1), "Point", pointCoordinates(1820.12, 890.56));
            assertFalse(resultSet.wasNull());
          }
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldInsertGeometryUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOMETRY column exists
    String tableName = createTempTable(connection, "ud_geom_bind_", "id INT, geo GEOMETRY");

    // When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
    List<String> wktValues = Arrays.asList(POINT_WKT, LINESTRING_WKT, POLYGON_WKT);
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " SELECT ?, TO_GEOMETRY(?)")) {
      for (int i = 0; i < wktValues.size(); i++) {
        preparedStatement.setInt(1, i + 1);
        preparedStatement.setString(2, wktValues.get(i));
        preparedStatement.execute();
      }
    }

    // Then SELECT should return the inserted GeoJSON values
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY id",
        resultSet -> {
          assertTrue(resultSet.next());
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(1820.12, 890.56));
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertGeoJson(resultSet.getString(2), "LineString", lineStringCoordinates());
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertGeoJson(resultSet.getString(2), "Polygon", polygonCoordinates());
          assertFalse(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  private static JSONArray pointCoordinates(double x, double y) {
    return new JSONArray().put(x).put(y);
  }

  private static JSONArray lineStringCoordinates() {
    return new JSONArray()
        .put(new JSONArray().put(0).put(0))
        .put(new JSONArray().put(1).put(1))
        .put(new JSONArray().put(2).put(2));
  }

  private static JSONArray polygonCoordinates() {
    return new JSONArray()
        .put(
            new JSONArray()
                .put(new JSONArray().put(0).put(0))
                .put(new JSONArray().put(4).put(0))
                .put(new JSONArray().put(4).put(3))
                .put(new JSONArray().put(0).put(3))
                .put(new JSONArray().put(0).put(0)));
  }
}
