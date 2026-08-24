package net.snowflake.jdbc.e2e.types;

import static java.sql.ResultSetMetaData.columnNullable;
import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Types;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.jdbc.utils.SkipForJSONResultSet;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * End-to-end coverage for Snowflake VECTOR.
 *
 * <p>VECTOR(INT, n) and VECTOR(FLOAT, n) arrive as Arrow {@code FixedSizeList} and are exposed via
 * {@code getString()} / {@code getObject()} as compact JSON-style strings ({@code [1,2,3]} / {@code
 * [-1.2,5.1]}), matching snowflake-jdbc parity. Typed access uses {@code
 * SnowflakeResultSet.getArray(col, Integer/Float.class)}.
 */
class VectorTests extends SnowflakeIntegrationTestBase implements WithQueryUtils {

  private static final int LARGE_RESULT_SET_SIZE = 20_000;
  private static final int MAX_DIMENSION_SIZE = 4096;
  private static final float FLOAT32_SMALLEST_NORMAL = 1.1754944e-38f;
  private static final String INT_VECTOR_COLUMN = "[1, 2, 3]::VECTOR(INT, 3)";
  private static final String FLOAT_VECTOR_COLUMN = "[1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)";
  private static final int VECTOR_DIMENSION = 3;

  // ==========================================================================
  // TYPE CASTING
  // ==========================================================================

  @Test
  void shouldCastVectorValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
    withQueryResult(
        connection,
        "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)",
        resultSet -> {
          // Then All values should be returned as appropriate type
          assertTrue(resultSet.next());
          assertAllVectorGetters(resultSet, 1, "[1,2,3]", Integer.class, new Integer[] {1, 2, 3});
          assertAllVectorGetters(
              resultSet, 2, "[1.5,2.5,3.5]", Float.class, new Float[] {1.5f, 2.5f, 3.5f});

          ResultSetMetaData meta = resultSet.getMetaData();
          SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
          assertVectorResultSetMetadata(
              meta,
              sfMeta,
              Arrays.asList(INT_VECTOR_COLUMN, FLOAT_VECTOR_COLUMN),
              VECTOR_DIMENSION,
              Arrays.asList(Types.INTEGER, Types.FLOAT));

          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // SELECT LITERALS
  // ==========================================================================

  private static Stream<Arguments> vectorLiteralCases() {
    return Stream.of(
        Arguments.of("INT-3d", "INT", "[1, 3, -5]", 3, "[1,3,-5]"),
        Arguments.of("INT-2d", "INT", "[40, 1234567]", 2, "[40,1234567]"),
        Arguments.of(
            "FLOAT-5d", "FLOAT", "[1.8, -3.4, 6.7, 0.0, 2.3]", 5, "[1.8,-3.4,6.7,0.0,2.3]"));
  }

  @ParameterizedTest
  @MethodSource("vectorLiteralCases")
  void shouldSelectSubtypeVectorLiteral(
      String subtype, String vecType, String expectedValue, int dimension, String expectedString)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    String sql = String.format("SELECT %s::VECTOR(%s, %d)", expectedValue, vecType, dimension);
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain <subtype> vector <expected_value>
          assertTrue(resultSet.next());
          assertEquals(expectedString, resultSet.getString(1));
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleNullVectorValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
    // is executed
    withQueryResult(
        connection,
        "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)",
        resultSet -> {
          // Then Result should contain [[1, 2, 3], NULL, NULL]
          assertTrue(resultSet.next());
          assertEquals("[1,2,3]", resultSet.getString(1));
          assertEquals("[1,2,3]", resultSet.getObject(1));
          assertFalse(resultSet.wasNull());

          assertNull(resultSet.getString(2));
          assertNull(resultSet.getObject(2));
          assertTrue(resultSet.wasNull());

          assertNull(resultSet.getString(3));
          assertNull(resultSet.getObject(3));
          assertTrue(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  private static Stream<Arguments> vectorBoundaryCases() {
    return Stream.of(
        Arguments.of(
            "INT", "INT", "[-2147483648, 2147483647, 0]", 3, "[-2147483648,2147483647,0]", true),
        Arguments.of(
            "FLOAT", "FLOAT", "[3.4028235e38, -3.4028235e38, 0.0]", 3, "[3.402823", false));
  }

  @ParameterizedTest
  @MethodSource("vectorBoundaryCases")
  void shouldSelectSubtypeVectorBoundaryValues(
      String subtype,
      String vecType,
      String expectedValue,
      int dimension,
      String expectedString,
      boolean exactMatch)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    String sql = String.format("SELECT %s::VECTOR(%s, %d)", expectedValue, vecType, dimension);
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should preserve <subtype> boundary values
          assertTrue(resultSet.next());
          String str = resultSet.getString(1);
          if (exactMatch) {
            assertEquals(expectedString, str);
          } else {
            assertTrue(str.startsWith(expectedString), "Expected float max, got: " + str);
          }
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  @SkipForJSONResultSet(
      "Server-side JSON serialization flushes FLOAT32 subnormals to zero; Arrow preserves them")
  void shouldPreserveFloatSmallestNormal() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query selects a VECTOR(FLOAT, ...) containing FLOAT32_SMALLEST_NORMAL
    String sql = "SELECT [" + FLOAT32_SMALLEST_NORMAL + "]::VECTOR(FLOAT, 1)";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the smallest-normal value must not underflow to zero
          assertTrue(resultSet.next());
          String str = resultSet.getString(1);
          assertFalse(str.endsWith("0.0]"), "Vector underflowed to zero");
          assertTrue(str.contains("1.175"), "Mantissa not preserved");
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldSelectMaxDimensionVector() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query selecting 4096-element float vector is executed
    String values =
        IntStream.range(0, MAX_DIMENSION_SIZE)
            .mapToObj(String::valueOf)
            .collect(Collectors.joining(", "));
    String sql = String.format("SELECT [%s]::VECTOR(FLOAT, %d)", values, MAX_DIMENSION_SIZE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should be a valid 4096-element float vector
          assertTrue(resultSet.next());
          ResultSetMetaData meta = resultSet.getMetaData();
          assertEquals(
              MAX_DIMENSION_SIZE,
              meta.unwrap(SnowflakeResultSetMetaData.class).getVectorDimension(1),
              "vector dimension");
          String str = resultSet.getString(1);
          assertTrue(
              str.startsWith("[") && str.endsWith("]"), "Expected vector string, got: " + str);
          assertFalse(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // TABLE OPERATIONS
  // ==========================================================================

  @Test
  void shouldSelectVectorValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
    String table =
        createTempTable(
            connection,
            "ud_vec_table_",
            "id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 5)");

    execute(
        connection,
        "INSERT INTO "
            + table
            + " SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.1, 2.2, 3.3, 4.4, 5.5]::VECTOR(FLOAT, 5) "
            + "UNION ALL SELECT 2, [10, 20, 30]::VECTOR(INT, 3), "
            + "[10.5, 20.5, 30.5, 40.5, 50.5]::VECTOR(FLOAT, 5)");

    // When Query "SELECT * FROM <table> ORDER BY id" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + table + " ORDER BY id",
        resultSet -> {
          // Then Result should contain the expected integer and float vector values
          assertTrue(resultSet.next());
          assertEquals("[1,2,3]", resultSet.getString(2));
          assertFalse(resultSet.wasNull());
          String floatVec1 = resultSet.getString(3);
          assertTrue(floatVec1.startsWith("[1."), "Expected [1.1,...], got: " + floatVec1);
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertEquals("[10,20,30]", resultSet.getString(2));
          assertFalse(resultSet.wasNull());
          assertEquals("[10.5,20.5,30.5,40.5,50.5]", resultSet.getString(3));
          assertFalse(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  @Test
  void shouldHandleNullVectorValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with VECTOR columns exist containing NULLs and values
    String table =
        createTempTable(
            connection,
            "ud_vec_null_",
            "id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3)");

    execute(
        connection,
        "INSERT INTO "
            + table
            + " SELECT 1, [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3) "
            + "UNION ALL SELECT 2, NULL::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3) "
            + "UNION ALL SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)");

    // When Query "SELECT * FROM <table> ORDER BY id" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + table + " ORDER BY id",
        resultSet -> {
          // Then Result should contain both vector values and NULLs
          assertTrue(resultSet.next());
          assertEquals("[1,2,3]", resultSet.getString(2));
          assertFalse(resultSet.wasNull());
          assertNull(resultSet.getString(3));
          assertTrue(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertNull(resultSet.getString(2));
          assertTrue(resultSet.wasNull());
          assertEquals("[1.5,2.5,3.5]", resultSet.getString(3));
          assertFalse(resultSet.wasNull());

          assertTrue(resultSet.next());
          assertNull(resultSet.getString(2));
          assertTrue(resultSet.wasNull());
          assertNull(resultSet.getString(3));
          assertTrue(resultSet.wasNull());

          assertFalse(resultSet.next());
        });
  }

  // ==========================================================================
  // MULTIPLE CHUNKS DOWNLOADING
  // ==========================================================================

  @Test
  void shouldDownloadVectorDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query generating 20000 integer vectors is executed
    String sql =
        "SELECT [SEQ4(), SEQ4() + 1, SEQ4() + 2]::VECTOR(INT, 3) AS v "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY SEQ4()";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then All 20000 rows should be fetched with valid 3-element integer vectors
          int rowCount = 0;
          while (resultSet.next()) {
            String str = resultSet.getString(1);
            assertFalse(resultSet.wasNull(), "Row " + rowCount + " should not be null");
            assertTrue(
                str.startsWith("[") && str.endsWith("]"),
                "Row " + rowCount + " should be a vector string, got: " + str);
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected vector row count");
        });
  }

  private static <T> void assertAllVectorGetters(
      ResultSet resultSet,
      int columnIndex,
      String expectedString,
      Class<T> elementType,
      T[] expectedArray)
      throws Exception {
    assertEquals(expectedString, resultSet.getString(columnIndex));
    assertFalse(resultSet.wasNull());
    assertEquals(expectedString, resultSet.getObject(columnIndex));
    assertFalse(resultSet.wasNull());
    assertArrayEquals(
        expectedArray,
        resultSet.unwrap(SnowflakeResultSet.class).getArray(columnIndex, elementType));
    assertFalse(resultSet.wasNull());
  }

  private static void assertVectorResultSetMetadata(
      ResultSetMetaData meta,
      SnowflakeResultSetMetaData sfMeta,
      List<String> expectedColumnNames,
      int expectedDimension,
      List<Integer> expectedElementJdbcTypes)
      throws Exception {
    assertAll(
        "result set metadata",
        () -> assertEquals(expectedColumnNames.size(), meta.getColumnCount(), "column count"),
        () -> assertEquals(expectedColumnNames, sfMeta.getColumnNames(), "column names"),
        () -> assertFalse(sfMeta.getQueryID().isEmpty(), "query id"));

    for (int column = 1; column <= expectedColumnNames.size(); column++) {
      String columnName = expectedColumnNames.get(column - 1);
      assertVectorColumnMetadata(
          meta,
          sfMeta,
          column,
          columnName,
          expectedDimension,
          expectedElementJdbcTypes.get(column - 1));
    }
  }

  private static void assertVectorColumnMetadata(
      ResultSetMetaData meta,
      SnowflakeResultSetMetaData sfMeta,
      int column,
      String columnName,
      int expectedDimension,
      int expectedElementJdbcType)
      throws Exception {
    assertAll(
        "column " + column + " metadata",
        () -> assertEquals(columnName, meta.getColumnName(column), "column name"),
        () -> assertEquals(columnName, meta.getColumnLabel(column), "column label"),
        () ->
            assertEquals(
                SnowflakeType.EXTRA_TYPES_VECTOR, meta.getColumnType(column), "column type"),
        () -> assertEquals("VECTOR", meta.getColumnTypeName(column), "column type name"),
        () ->
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> meta.getColumnClassName(column),
                "column class name"),
        () -> assertEquals(0, meta.getPrecision(column), "precision"),
        () -> assertEquals(0, meta.getScale(column), "scale"),
        () -> assertEquals(25, meta.getColumnDisplaySize(column), "display size"),
        () -> assertFalse(meta.isSigned(column), "signed"),
        () -> assertFalse(meta.isCaseSensitive(column), "case sensitivity"),
        () -> assertTrue(meta.isSearchable(column), "searchable"),
        () -> assertFalse(meta.isCurrency(column), "currency"),
        () -> assertEquals(columnNullable, meta.isNullable(column), "nullable"),
        () -> assertFalse(meta.isAutoIncrement(column), "auto increment"),
        () -> assertTrue(meta.isReadOnly(column), "read only"),
        () -> assertFalse(meta.isWritable(column), "writable"),
        () -> assertFalse(meta.isDefinitelyWritable(column), "definitely writable"),
        () -> assertEquals("", meta.getCatalogName(column), "catalog name"),
        () -> assertEquals("", meta.getSchemaName(column), "schema name"),
        () -> assertEquals("", meta.getTableName(column), "table name"),
        () ->
            assertEquals(
                SnowflakeType.EXTRA_TYPES_VECTOR,
                sfMeta.getInternalColumnType(column),
                "internal column type"),
        () ->
            assertEquals(expectedDimension, sfMeta.getVectorDimension(column), "vector dimension"),
        () ->
            assertEquals(
                expectedDimension,
                sfMeta.getVectorDimension(columnName),
                "vector dimension by name"),
        () -> assertEquals(column - 1, sfMeta.getColumnIndex(columnName), "column index"));

    assertVectorColumnFields(sfMeta, column, expectedElementJdbcType);
  }

  private static void assertVectorColumnFields(
      SnowflakeResultSetMetaData sfMeta, int column, int expectedElementJdbcType) throws Exception {
    if (isNewDriver()) {
      // TODO(SNOW-3740745): Port structured-type field metadata to proto-based constructor
      return;
    }

    List<FieldMetadata> fields = sfMeta.getColumnFields(column);
    assertEquals(1, fields.size(), "column fields count");
    assertEquals(expectedElementJdbcType, fields.get(0).getType(), "vector element JDBC type");
  }
}
