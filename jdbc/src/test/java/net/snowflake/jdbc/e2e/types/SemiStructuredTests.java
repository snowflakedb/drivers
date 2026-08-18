package net.snowflake.jdbc.e2e.types;

import static java.sql.ResultSetMetaData.columnNullable;
import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSetMetaData;
import java.sql.Types;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * End-to-end coverage for Snowflake semi-structured types (VARIANT, OBJECT, ARRAY).
 *
 * <p>Values are returned as JSON strings ({@code String}). Output format varies by result-set
 * encoding, but logical JSON content must match.
 */
class SemiStructuredTests extends SnowflakeIntegrationTestBase
    implements WithScalarResultSetMetadataAssertions {

  private static final int LARGE_RESULT_SET_SIZE = 20_000;

  // Literal/expression metadata matches SnowflakeResultSetMetaDataImplTest scalar cases.
  private static final int SEMI_STRUCTURED_LITERAL_DISPLAY_SIZE = 0;

  private static final ColumnExpectation VARIANT_LITERAL_COLUMN =
      new ColumnExpectation(
          null,
          Types.VARCHAR,
          "VARIANT",
          String.class.getName(),
          0,
          0,
          SEMI_STRUCTURED_LITERAL_DISPLAY_SIZE,
          false,
          true,
          columnNullable,
          null);

  private static final ColumnExpectation ARRAY_LITERAL_COLUMN =
      new ColumnExpectation(
          null,
          Types.VARCHAR,
          "ARRAY",
          String.class.getName(),
          0,
          0,
          SEMI_STRUCTURED_LITERAL_DISPLAY_SIZE,
          false,
          true,
          columnNullable,
          null);

  private static final ColumnExpectation OBJECT_LITERAL_COLUMN =
      new ColumnExpectation(
          null,
          Types.VARCHAR,
          "OBJECT",
          String.class.getName(),
          0,
          0,
          SEMI_STRUCTURED_LITERAL_DISPLAY_SIZE,
          false,
          true,
          columnNullable,
          null);

  // ==========================================================================
  // TYPE CASTING
  // ==========================================================================

  @Test
  void shouldCastSemiStructuredValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3),
    // OBJECT_CONSTRUCT('key','val')" is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')",
        resultSet -> {
          // Then All values should be returned as appropriate type
          assertTrue(resultSet.next());
          assertInstanceOf(String.class, resultSet.getObject(1));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(2));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(3));
          assertFalse(resultSet.wasNull());

          assertJsonEquals(resultSet.getString(1), "{\"a\":1}");
          assertJsonEquals(resultSet.getString(2), "[1,2,3]");
          assertJsonEquals(resultSet.getString(3), "{\"key\":\"val\"}");

          ResultSetMetaData meta = resultSet.getMetaData();
          SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
          assertScalarResultSetMetadata(
              meta,
              sfMeta,
              Arrays.asList(
                  VARIANT_LITERAL_COLUMN.withColumnName(meta.getColumnName(1)),
                  ARRAY_LITERAL_COLUMN.withColumnName(meta.getColumnName(2)),
                  OBJECT_LITERAL_COLUMN.withColumnName(meta.getColumnName(3))));

          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // SELECT LITERALS
  // ==========================================================================

  @Test
  void shouldSelectSemiStructuredLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30),
    // OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), "
            + "OBJECT_CONSTRUCT('a', 1, 'b', 2)",
        resultSet -> {
          // Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
          assertTrue(resultSet.next());
          assertInstanceOf(String.class, resultSet.getObject(1));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(2));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(3));
          assertFalse(resultSet.wasNull());

          assertJsonEquals(resultSet.getString(1), "{\"key\":\"value\"}");
          assertJsonEquals(resultSet.getString(2), "[10,20,30]");
          assertJsonEquals(resultSet.getString(3), "{\"a\":1,\"b\":2}");
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldSelectDeeplyNestedSemiStructuredLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')",
        resultSet -> {
          // Then Result should contain the expected nested value
          assertTrue(resultSet.next());
          assertInstanceOf(String.class, resultSet.getObject(1));
          assertFalse(resultSet.wasNull());
          assertJsonEquals(resultSet.getString(1), "{\"a\":{\"b\":[1,2,{\"c\":true}]}}");
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // NULL HANDLING
  // ==========================================================================

  @Test
  void shouldHandleNullSemiStructuredValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
    withQueryResult(
        connection,
        "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY",
        resultSet -> {
          // Then All columns should return null indicators
          assertTrue(resultSet.next());
          assertNull(resultSet.getString(1));
          assertNull(resultSet.getObject(1));
          assertTrue(resultSet.wasNull());
          assertNull(resultSet.getString(2));
          assertNull(resultSet.getObject(2));
          assertTrue(resultSet.wasNull());
          assertNull(resultSet.getString(3));
          assertNull(resultSet.getObject(3));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // TABLE OPERATIONS
  // ==========================================================================

  @Test
  void shouldSelectSemiStructuredValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
    String tableName =
        createTempTable(
            connection, "ud_semi_struct_", "var_col VARIANT, obj_col OBJECT, arr_col ARRAY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT PARSE_JSON('{\"x\":42}'), "
            + "OBJECT_CONSTRUCT('key', 'value'), "
            + "ARRAY_CONSTRUCT(1, 2, 3)");

    // When Query "SELECT * FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName,
        resultSet -> {
          ResultSetMetaData meta = resultSet.getMetaData();
          assertSemiStructuredTableColumnMetadata(meta, 1, "VARIANT", tableName);
          assertSemiStructuredTableColumnMetadata(meta, 2, "OBJECT", tableName);
          assertSemiStructuredTableColumnMetadata(meta, 3, "ARRAY", tableName);

          // Then Data should contain the expected semi-structured values
          assertTrue(resultSet.next());
          assertInstanceOf(String.class, resultSet.getObject(1));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(2));
          assertFalse(resultSet.wasNull());
          assertInstanceOf(String.class, resultSet.getObject(3));
          assertFalse(resultSet.wasNull());

          assertJsonEquals(resultSet.getString(1), "{\"x\":42}");
          assertJsonEquals(resultSet.getString(2), "{\"key\":\"value\"}");
          assertJsonEquals(resultSet.getString(3), "[1,2,3]");
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleNullSemiStructuredValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VARIANT column exists containing NULLs and values
    String tableName = createTempTable(connection, "ud_semi_null_", "col VARIANT, id INT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT PARSE_JSON(column2), column1 FROM VALUES (1, NULL), (2, '{\"a\":1}'), (3, NULL)");

    // When Query "SELECT * FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY id",
        resultSet -> {
          // Then Result should contain [NULL, {"a":1}, NULL]
          assertTrue(resultSet.next());
          assertNull(resultSet.getString(1));
          assertNull(resultSet.getObject(1));
          assertTrue(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "{\"a\":1}");
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertNull(resultSet.getString(1));
          assertNull(resultSet.getObject(1));
          assertTrue(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // EMPTY JSON CONTAINERS
  // ==========================================================================

  @Test
  void shouldHandleEmptyJsonContainers() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()",
        resultSet -> {
          // Then Each column should return a valid empty container
          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "{}");
          assertFalse(resultSet.wasNull());
          assertJsonEquals(resultSet.getString(2), "[]");
          assertFalse(resultSet.wasNull());
          assertJsonEquals(resultSet.getString(3), "{}");
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleEmptyJsonArrayLiteral() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON('[]')" is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('[]')",
        resultSet -> {
          // Then Result should be an empty JSON array
          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "[]");
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldRoundTripEmptyJsonContainersThroughATable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
    String tableName =
        createTempTable(
            connection, "ud_semi_empty_", "var_col VARIANT, obj_col OBJECT, arr_col ARRAY");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT PARSE_JSON('{}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()");

    // When Query "SELECT * FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName,
        resultSet -> {
          // Then All columns should return valid empty containers
          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "{}");
          assertFalse(resultSet.wasNull());
          assertJsonEquals(resultSet.getString(2), "{}");
          assertFalse(resultSet.wasNull());
          assertJsonEquals(resultSet.getString(3), "[]");
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // JSON WITH UNICODE CONTENT
  // ==========================================================================

  @Test
  void shouldHandleJsonWithUnicodeContent() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query returning JSON with unicode characters is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{\"greeting\":\"\u3053\u3093\u306b\u3061\u306f\",\"emoji\":\"\u26c4\"}')",
        resultSet -> {
          // Then Result should preserve the unicode characters
          assertTrue(resultSet.next());
          JsonNode parsed = parseJson(resultSet.getString(1));
          assertFalse(resultSet.wasNull());
          assertEquals("\u3053\u3093\u306b\u3061\u306f", parsed.get("greeting").asText());
          assertEquals("\u26c4", parsed.get("emoji").asText());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleJsonWithUnicodeInKeys() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query returning JSON with unicode characters in keys is executed
    withQueryResult(
        connection,
        "SELECT PARSE_JSON('{\"\u540d\u524d\":\"\u30c6\u30b9\u30c8\",\"donn\u00e9es\":\"valeur\"}')",
        resultSet -> {
          // Then Result should preserve unicode keys and their associated values
          assertTrue(resultSet.next());
          JsonNode parsed = parseJson(resultSet.getString(1));
          assertFalse(resultSet.wasNull());
          assertEquals("\u30c6\u30b9\u30c8", parsed.get("\u540d\u524d").asText());
          assertEquals("valeur", parsed.get("donn\u00e9es").asText());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // MULTIPLE CHUNKS DOWNLOADING
  // ==========================================================================

  @Test
  void shouldDownloadSemiStructuredDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT =>
    // 20000)) v ORDER BY 1" is executed
    String sql =
        "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY 1";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then All 20000 rows should be fetched and each should contain a value with "id" key
          int rowCount = 0;
          while (resultSet.next()) {
            JsonNode parsed = parseJson(resultSet.getString(1));
            assertFalse(resultSet.wasNull(), "obj column should not be NULL at row " + rowCount);
            assertTrue(parsed.has("id"), "missing id key at row " + rowCount);
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected semi-structured row count");
        });
  }

  // ==========================================================================
  // PARAMETER BINDING
  // ==========================================================================

  @Test
  void shouldSelectVariantUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
    withPreparedQueryResult(
        connection,
        "SELECT PARSE_JSON(?)",
        ps -> ps.setString(1, "{\"bound\":true}"),
        resultSet -> {
          // Then Result should contain a value with "bound" key
          assertTrue(resultSet.next());
          JsonNode parsed = parseJson(resultSet.getString(1));
          assertFalse(resultSet.wasNull());
          assertTrue(parsed.get("bound").asBoolean());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldSelectNullVariantUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT PARSE_JSON(?)",
        ps -> ps.setNull(1, Types.VARCHAR),
        resultSet -> {
          // Then Result should be NULL
          assertTrue(resultSet.next());
          assertNull(resultSet.getString(1));
          assertNull(resultSet.getObject(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldInsertVariantUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VARIANT column exists
    String tableName = createTempTable(connection, "ud_semi_bind_", "col VARIANT, id INT");

    // When JSON values are inserted using parameter binding via PARSE_JSON(?)
    List<String> jsonValues = Arrays.asList("{\"x\":1}", "[1,2,3]", "{\"nested\":{\"a\":true}}");
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " SELECT PARSE_JSON(?), ?")) {
      for (int i = 0; i < jsonValues.size(); i++) {
        preparedStatement.setString(1, jsonValues.get(i));
        preparedStatement.setInt(2, i + 1);
        preparedStatement.execute();
      }
    }

    // Then SELECT should return the inserted JSON values
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY id",
        resultSet -> {
          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "{\"x\":1}");
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "[1,2,3]");
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertJsonEquals(resultSet.getString(1), "{\"nested\":{\"a\":true}}");
          assertFalse(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  /**
   * Table-backed columns carry the same sizing as literals but, unlike them, report the originating
   * catalog/schema/table — so they can't reuse {@code assertScalarResultSetMetadata}, which pins
   * those three to empty.
   */
  private static void assertSemiStructuredTableColumnMetadata(
      ResultSetMetaData meta, int column, String expectedTypeName, String expectedTableName) {
    assertAll(
        "table column " + column + " metadata",
        () -> assertEquals(Types.VARCHAR, meta.getColumnType(column), "column type"),
        () -> assertEquals(expectedTypeName, meta.getColumnTypeName(column), "column type name"),
        () ->
            assertEquals(
                String.class.getName(), meta.getColumnClassName(column), "column class name"),
        () -> assertEquals(0, meta.getPrecision(column), "precision"),
        () -> assertEquals(0, meta.getScale(column), "scale"),
        () -> assertEquals(0, meta.getColumnDisplaySize(column), "display size"),
        () -> assertFalse(meta.isSigned(column), "signed"),
        () -> assertTrue(meta.isCaseSensitive(column), "case sensitive"),
        () -> assertTrue(meta.isSearchable(column), "searchable"),
        () -> assertEquals(columnNullable, meta.isNullable(column), "nullable"),
        () ->
            assertEquals(
                expectedTableName.toUpperCase(Locale.ROOT),
                meta.getTableName(column),
                "table name"));
  }

  /** Compare JSON by logical content so Arrow vs JSON result formats stay interchangeable. */
  private static void assertJsonEquals(String actualJson, String expectedJson) {
    assertEquals(parseJson(expectedJson), parseJson(actualJson));
  }
}
