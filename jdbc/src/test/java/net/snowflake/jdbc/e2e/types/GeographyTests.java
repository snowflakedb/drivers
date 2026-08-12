package net.snowflake.jdbc.e2e.types;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static net.snowflake.jdbc.utils.JsonTestUtils.arrayNode;
import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
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
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * End-to-end coverage for Snowflake GEOGRAPHY.
 *
 * <p>GEOGRAPHY values default to GeoJSON strings. Output format is controlled by {@code
 * GEOGRAPHY_OUTPUT_FORMAT}: GeoJSON, WKT, and EWKT arrive as {@code String}; WKB and EWKB as {@code
 * byte[]}.
 */
class GeographyTests extends SnowflakeIntegrationTestBase implements WithGeoAssertions {

  private static final int LARGE_RESULT_SET_SIZE = 20_000;

  private static final String POINT_WKT = "POINT(-122.35 37.55)";
  private static final String LINESTRING_WKT = "LINESTRING(0 0, 1 1, 2 2)";
  private static final String POLYGON_WKT = "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))";

  private static Stream<Arguments> geographyLiteralCases() {
    return Stream.of(
        Arguments.of(
            "Point", "TO_GEOGRAPHY('" + POINT_WKT + "')", pointCoordinates(-122.35, 37.55)),
        Arguments.of(
            "LineString", "TO_GEOGRAPHY('" + LINESTRING_WKT + "')", lineStringCoordinates()),
        Arguments.of("Polygon", "TO_GEOGRAPHY('" + POLYGON_WKT + "')", polygonCoordinates()));
  }

  private static Stream<Arguments> geographyOutputFormatCases() {
    return Stream.of(
        Arguments.of("GeoJSON", String.class),
        Arguments.of("WKT", String.class),
        Arguments.of("WKB", byte[].class),
        Arguments.of("EWKT", String.class),
        Arguments.of("EWKB", byte[].class));
  }

  private static Stream<Arguments> geographyBindingSelectCases() {
    return Stream.of(Arguments.of(POINT_WKT), Arguments.of((String) null));
  }

  // ==========================================================================
  // SELECT LITERALS
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geographyLiteralCases")
  void shouldSelectShapeGeographyLiteral(
      String shape, String queryValue, JsonNode expectedCoordinates) throws Exception {
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

  @Test
  void shouldSelectGeographyFromGeoJsonInput() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is
    // executed
    String geoJson = "{\"type\":\"Point\",\"coordinates\":[-122.35,37.55]}";
    withQueryResult(
        connection,
        "SELECT TO_GEOGRAPHY('" + geoJson + "')",
        resultSet -> {
          // Then Result should contain a GeoJSON Point value
          assertTrue(resultSet.next());
          assertGeoJson(resultSet.getString(1), "Point", pointCoordinates(-122.35, 37.55));
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // TYPE CASTING PER OUTPUT FORMAT
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geographyOutputFormatCases")
  void shouldCastGeographyToExpectedTypeForFormatOutputFormat(
      String outputFormat, Class<?> expectedType) throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openConnection()) {
      // And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
      execute(connection, "ALTER SESSION SET GEOGRAPHY_OUTPUT_FORMAT = '" + outputFormat + "'");

      // When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
      withQueryResult(
          connection,
          "SELECT TO_GEOGRAPHY('" + POINT_WKT + "')",
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
                "geography metadata for " + outputFormat,
                () -> assertEquals(1, meta.getColumnCount(), "column count"),
                () -> assertFalse(sfMeta.getQueryID().isEmpty(), "query id"),
                () -> assertEquals(expectedJdbcType, meta.getColumnType(1), "column type"),
                () -> assertEquals("GEOGRAPHY", meta.getColumnTypeName(1), "column type name"),
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
  void shouldSelectGeographyValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOGRAPHY column exists with WKT values
    String tableName = createTempTable(connection, "ud_geo_table_", "id INT, geo GEOGRAPHY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT 1, TO_GEOGRAPHY('"
            + POINT_WKT
            + "') UNION ALL SELECT 2, TO_GEOGRAPHY('"
            + LINESTRING_WKT
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
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(-122.35, 37.55));
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertEquals(2, resultSet.getInt(1));
          assertFalse(resultSet.wasNull());
          assertGeoJson(resultSet.getString(2), "LineString", lineStringCoordinates());
          assertFalse(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleNullGeographyValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOGRAPHY column exists containing NULLs and values
    String tableName = createTempTable(connection, "ud_geo_null_", "id INT, geo GEOGRAPHY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT 1, TO_GEOGRAPHY('"
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
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(-122.35, 37.55));
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
  @SkipForJSONResultSet("Large geography result sets require Arrow chunk download")
  void shouldDownloadGeographyDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query generating 20000 geography points is executed
    String sql =
        "SELECT id, TO_GEOGRAPHY('POINT(' || (MOD(id, 360) - 180) || ' ' || (MOD(id, 180) - 90)"
            + " || ')') AS geo "
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

            JsonNode geo = parseJson(geoJson);
            assertEquals("Point", geo.get("type").asText());
            JsonNode coordinates = geo.get("coordinates");
            assertEquals(2, coordinates.size());
            double expectedLon = (rowCount % 360) - 180.0;
            double expectedLat = (rowCount % 180) - 90.0;
            assertEquals(expectedLon, coordinates.get(0).asDouble(), 1e-9);
            assertEquals(expectedLat, coordinates.get(1).asDouble(), 1e-9);

            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected geography row count");
        });
  }

  // ==========================================================================
  // PARAMETER BINDING
  // ==========================================================================

  @ParameterizedTest
  @MethodSource("geographyBindingSelectCases")
  void shouldSelectGeographyUsingParameterBindingWithInputTypeValue(String bindValue)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
    withPreparedQueryResult(
        connection,
        "SELECT TO_GEOGRAPHY(?)",
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
            assertGeoJson(resultSet.getString(1), "Point", pointCoordinates(-122.35, 37.55));
            assertFalse(resultSet.wasNull());
          }
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldInsertGeographyUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with GEOGRAPHY column exists
    String tableName = createTempTable(connection, "ud_geo_bind_", "id INT, geo GEOGRAPHY");

    // When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
    List<String> wktValues = Arrays.asList(POINT_WKT, LINESTRING_WKT, POLYGON_WKT);
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " SELECT ?, TO_GEOGRAPHY(?)")) {
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
          assertGeoJson(resultSet.getString(2), "Point", pointCoordinates(-122.35, 37.55));
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

  private static ArrayNode pointCoordinates(double lon, double lat) {
    return arrayNode().add(lon).add(lat);
  }

  private static ArrayNode lineStringCoordinates() {
    return arrayNode()
        .add(arrayNode().add(0).add(0))
        .add(arrayNode().add(1).add(1))
        .add(arrayNode().add(2).add(2));
  }

  private static ArrayNode polygonCoordinates() {
    return arrayNode()
        .add(
            arrayNode()
                .add(arrayNode().add(0).add(0))
                .add(arrayNode().add(10).add(0))
                .add(arrayNode().add(10).add(10))
                .add(arrayNode().add(0).add(10))
                .add(arrayNode().add(0).add(0)));
  }
}
