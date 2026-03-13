package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Types;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class FloatTests extends SnowflakeIntegrationTestBase {
  private static final String FLOAT_TYPE = "FLOAT";
  private static final int LARGE_RESULT_SET_SIZE = 50_000;

  @Test
  public void shouldCastFloatValuesToAppropriateTypeForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>,
    // 'inf'::<type>" is executed
    String sql =
        String.format(
            "SELECT 0.0::%1$s, 123.456::%1$s, 1.23e10::%1$s, 'NaN'::%1$s, 'inf'::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then All values should be returned as appropriate type
          assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);

          // And Regular values should have approximately 15 decimal digits precision
          assertAllFloatGetters(resultSet, 1, 0.0, "Column 1 mismatch for " + FLOAT_TYPE);
          assertAllFloatGetters(resultSet, 2, 123.456, "Column 2 mismatch for " + FLOAT_TYPE);
          assertAllFloatGetters(resultSet, 3, 1.23e10, "Column 3 mismatch for " + FLOAT_TYPE);

          // And NaN and inf values should be identified correctly
          assertAllFloatGetters(resultSet, 4, Double.NaN, "Column 4 mismatch for " + FLOAT_TYPE);
          assertAllFloatGetters(
              resultSet, 5, Double.POSITIVE_INFINITY, "Column 5 mismatch for " + FLOAT_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldSelectFloatLiteralsForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>"
    // is executed
    String sql =
        String.format(
            "SELECT 0.0::%1$s, 1.0::%1$s, -1.0::%1$s, 123.456::%1$s, -123.456::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
          assertSingleRow(resultSet, Arrays.asList(0.0, 1.0, -1.0, 123.456, -123.456));
        });
  }

  @Test
  public void shouldHandleSpecialFloatValuesFromLiteralsForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
    String sql = String.format("SELECT 'NaN'::%1$s, 'inf'::%1$s, '-inf'::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NaN, positive_infinity, negative_infinity]
          assertSingleRow(
              resultSet,
              Arrays.asList(Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY));
        });
  }

  @Test
  public void shouldHandleFloatBoundaryValuesFromLiteralsForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 1.7976931348623157e308::<type>, -1.7976931348623157e308::<type>" is
    // executed
    String maxBoundarySql =
        String.format(
            "SELECT 1.7976931348623157e308::%1$s, -1.7976931348623157e308::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        maxBoundarySql,
        resultSet -> {

          // Then Result should contain floats [1.7976931348623157e308, -1.7976931348623157e308]
          assertSingleRow(resultSet, Arrays.asList(Double.MAX_VALUE, -Double.MAX_VALUE));
        });

    // When Query "SELECT 2.2250738585072014e-308::<type>, 5e-324::<type>" is executed
    String minBoundarySql =
        String.format("SELECT 2.2250738585072014e-308::%1$s, 5e-324::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        minBoundarySql,
        resultSet -> {

          // Then Result should contain floats [2.2250738585072014e-308, approximately 5e-324]
          assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);
          assertFiniteDouble(
              resultSet.getDouble(1), Double.MIN_NORMAL, "Column 1 mismatch for " + FLOAT_TYPE);
          double actualSubnormal = resultSet.getDouble(2);
          assertTrue(actualSubnormal > 0.0, "Column 2 should be positive for " + FLOAT_TYPE);
          assertTrue(
              actualSubnormal <= Double.MIN_VALUE,
              "Column 2 should be subnormal for " + FLOAT_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
        });

    // When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
    String precisionSql =
        String.format("SELECT 123456789012345.0::%1$s, 1234567890123456.0::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        precisionSql,
        resultSet -> {

          // Then Result should verify precision around 15 decimal digits
          assertSingleRow(resultSet, Arrays.asList(123456789012345.0, 1234567890123456.0));
        });
  }

  @Test
  public void shouldHandleNULLValuesFromLiteralsForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
    String sql = String.format("SELECT NULL::%1$s, 42.5::%1$s, NULL::%1$s", FLOAT_TYPE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NULL, 42.5, NULL]
          assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);
          assertNull(resultSet.getObject(1), "Column 1 should be NULL for " + FLOAT_TYPE);
          assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() for " + FLOAT_TYPE);
          assertEquals(
              0.0,
              resultSet.getDouble(1),
              "Column 1 getDouble should return 0.0 for NULL " + FLOAT_TYPE);
          assertTrue(
              resultSet.wasNull(), "Column 1 getDouble should set wasNull() for " + FLOAT_TYPE);

          assertAllFloatGetters(resultSet, 2, 42.5, "Column 2 mismatch for " + FLOAT_TYPE);

          assertNull(resultSet.getObject(3), "Column 3 should be NULL for " + FLOAT_TYPE);
          assertTrue(resultSet.wasNull(), "Column 3 should set wasNull() for " + FLOAT_TYPE);
          assertEquals(
              0.0,
              resultSet.getDouble(3),
              "Column 3 getDouble should return 0.0 for NULL " + FLOAT_TYPE);
          assertTrue(
              resultSet.wasNull(), "Column 3 getDouble should set wasNull() for " + FLOAT_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGeneratorForFloatAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is
    // executed
    String sql =
        String.format(
            "SELECT seq8()::%1$s as id FROM TABLE(GENERATOR(ROWCOUNT => %2$d)) v ORDER BY id",
            FLOAT_TYPE, LARGE_RESULT_SET_SIZE);
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain 50000 rows with all values returned as appropriate float
          // type
          int expected = 0;
          while (resultSet.next()) {
            assertFiniteDouble(
                resultSet.getDouble(1),
                expected,
                "Value mismatch for " + FLOAT_TYPE + ", row " + expected);
            expected++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldSelectFloatsFromTableForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
    String tableName = createTempTable(connection, "ud_float_", "col " + FLOAT_TYPE);
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (0.0), (123.456), (-789.012), (1.23e5), (-9.87e-3)");

    // When Query "SELECT * FROM float_table" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
          assertSingleColumnRows(
              resultSet, Arrays.asList(0.0, 123.456, -789.012, 123000.0, -0.00987));
        });
  }

  @Test
  public void shouldHandleSpecialFloatValuesFromTableForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
    String tableName = createTempTable(connection, "ud_float_", "col " + FLOAT_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('NaN'::"
            + FLOAT_TYPE
            + "), ('inf'::"
            + FLOAT_TYPE
            + "), ('-inf'::"
            + FLOAT_TYPE
            + "), (42.0), (-42.0)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
          Map<String, Integer> counts = new HashMap<>();
          while (resultSet.next()) {
            double value = resultSet.getDouble(1);
            String key;
            if (Double.isNaN(value)) {
              key = "NaN";
            } else if (value == Double.POSITIVE_INFINITY) {
              key = "POS_INF";
            } else if (value == Double.NEGATIVE_INFINITY) {
              key = "NEG_INF";
            } else if (value == 42.0) {
              key = "POS_42";
            } else if (value == -42.0) {
              key = "NEG_42";
            } else {
              key = "UNEXPECTED";
            }
            counts.put(key, counts.getOrDefault(key, 0) + 1);
          }

          assertEquals(1, counts.getOrDefault("NaN", 0), "Expected one NaN for " + FLOAT_TYPE);
          assertEquals(
              1, counts.getOrDefault("POS_INF", 0), "Expected one +Infinity for " + FLOAT_TYPE);
          assertEquals(
              1, counts.getOrDefault("NEG_INF", 0), "Expected one -Infinity for " + FLOAT_TYPE);
          assertEquals(1, counts.getOrDefault("POS_42", 0), "Expected one 42.0 for " + FLOAT_TYPE);
          assertEquals(1, counts.getOrDefault("NEG_42", 0), "Expected one -42.0 for " + FLOAT_TYPE);
          assertEquals(
              0,
              counts.getOrDefault("UNEXPECTED", 0),
              "Unexpected value returned for " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldHandleFloatBoundaryValuesFromTableForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with <type> column exists with boundary values [1.7976931348623157e308,
    // -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
    String tableName = createTempTable(connection, "ud_float_", "col " + FLOAT_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (1.7976931348623157e308), (-1.7976931348623157e308), (2.2250738585072014e-308), (5e-324), (123456789012345.0)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain maximum, minimum, and precision boundary values preserved
          // within float precision limits
          boolean foundMax = false;
          boolean foundMin = false;
          boolean foundMinNormal = false;
          boolean foundMinSubnormal = false;
          boolean foundPrecisionValue = false;
          int rowCount = 0;
          while (resultSet.next()) {
            rowCount++;
            double value = resultSet.getDouble(1);
            if (approximatelyEquals(value, Double.MAX_VALUE)) {
              foundMax = true;
            } else if (approximatelyEquals(value, -Double.MAX_VALUE)) {
              foundMin = true;
            } else if (approximatelyEquals(value, Double.MIN_NORMAL)) {
              foundMinNormal = true;
            } else if (value > 0.0 && value <= Double.MIN_VALUE) {
              foundMinSubnormal = true;
            } else if (approximatelyEquals(value, 123456789012345.0)) {
              foundPrecisionValue = true;
            }
          }

          assertEquals(5, rowCount, "Unexpected row count for " + FLOAT_TYPE);
          assertTrue(foundMax, "Expected Double.MAX_VALUE for " + FLOAT_TYPE);
          assertTrue(foundMin, "Expected -Double.MAX_VALUE for " + FLOAT_TYPE);
          assertTrue(foundMinNormal, "Expected Double.MIN_NORMAL for " + FLOAT_TYPE);
          assertTrue(foundMinSubnormal, "Expected positive subnormal value for " + FLOAT_TYPE);
          assertTrue(foundPrecisionValue, "Expected 123456789012345.0 for " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldHandleNULLValuesFromTableForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
    String tableName = createTempTable(connection, "ud_float_", "col " + FLOAT_TYPE);
    execute(
        connection, "INSERT INTO " + tableName + " VALUES (NULL), (123.456), (NULL), (-789.012)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NULL, 123.456, NULL, -789.012]
          assertSingleColumnRows(resultSet, Arrays.asList(null, 123.456, null, -789.012));
        });
  }

  @Test
  public void shouldSelectLargeResultSetFromTableForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with <type> column exists with 50000 sequential values
    String tableName = createTempTable(connection, "ud_float_", "col " + FLOAT_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT seq8()::"
            + FLOAT_TYPE
            + " FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT * FROM " + tableName + " ORDER BY col";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain 50000 rows with all values returned as appropriate float
          // type
          int expected = 0;
          while (resultSet.next()) {
            assertFiniteDouble(
                resultSet.getDouble(1),
                expected,
                "Value mismatch for " + FLOAT_TYPE + ", row " + expected);
            expected++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + FLOAT_TYPE);
        });
  }

  @Test
  public void shouldSelectFloatUsingParameterBindingForFloatAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed with bound float values
    // [123.456, -789.012, 42.0]
    String sql = String.format("SELECT ?::%1$s, ?::%1$s, ?::%1$s", FLOAT_TYPE);
    // When
    withPreparedQueryResult(
        connection,
        sql,
        ps -> {
          ps.setDouble(1, 123.456);
          ps.setDouble(2, -789.012);
          ps.setDouble(3, 42.0);
        },
        resultSet -> {
          // Then Result should contain floats [123.456, -789.012, 42.0]
          assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);
          assertAllFloatGetters(resultSet, 1, 123.456, "Column 1 mismatch for " + FLOAT_TYPE);
          assertAllFloatGetters(resultSet, 2, -789.012, "Column 2 mismatch for " + FLOAT_TYPE);
          assertAllFloatGetters(resultSet, 3, 42.0, "Column 3 mismatch for " + FLOAT_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
        });

    // When Query "SELECT ?::<type>" is executed with bound NULL value
    String nullSql = String.format("SELECT ?::%1$s", FLOAT_TYPE);
    withPreparedQueryResult(
        connection,
        nullSql,
        ps -> ps.setNull(1, Types.DOUBLE),
        resultSet -> {
          // Then Result should contain NULL
          assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);
          assertNull(resultSet.getObject(1), "Column 1 should be NULL for " + FLOAT_TYPE);
          assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() for " + FLOAT_TYPE);
          assertEquals(
              0.0,
              resultSet.getDouble(1),
              "Column 1 getDouble should return 0.0 for NULL " + FLOAT_TYPE);
          assertTrue(
              resultSet.wasNull(), "Column 1 getDouble should set wasNull() for " + FLOAT_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
        });
  }

  private static void assertSingleRow(ResultSet resultSet, List<Double> expected) throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: " + FLOAT_TYPE);
    for (int i = 0; i < expected.size(); i++) {
      assertAllFloatGetters(
          resultSet, i + 1, expected.get(i), "Column " + (i + 1) + " mismatch for " + FLOAT_TYPE);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: " + FLOAT_TYPE);
  }

  private static void assertSingleColumnRows(ResultSet resultSet, List<Double> expectedValues)
      throws Exception {
    for (int i = 0; i < expectedValues.size(); i++) {
      assertTrue(resultSet.next(), "Missing row " + i + " for " + FLOAT_TYPE);
      Double expected = expectedValues.get(i);
      if (expected == null) {
        assertNull(resultSet.getObject(1), "Expected NULL at row " + i + " for " + FLOAT_TYPE);
        assertTrue(resultSet.wasNull(), "Expected wasNull() after getObject NULL at row " + i);
        assertEquals(0.0, resultSet.getDouble(1), "Expected getDouble=0.0 for NULL at row " + i);
        assertTrue(resultSet.wasNull(), "Expected wasNull() after getDouble NULL at row " + i);
      } else {
        assertAllFloatGetters(
            resultSet, 1, expected, "Value mismatch for " + FLOAT_TYPE + ", row " + i);
      }
    }
    assertFalse(resultSet.next(), "Unexpected extra rows for " + FLOAT_TYPE);
  }

  private static void assertAllFloatGetters(
      ResultSet resultSet, int columnIndex, double expected, String message) throws Exception {
    Object value = resultSet.getObject(columnIndex);
    assertTrue(value instanceof Double, message + " (getObject should return Double)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");
    assertDoubleValue(((Double) value).doubleValue(), expected, message + " (getObject)");

    double doubleValue = resultSet.getDouble(columnIndex);
    assertFalse(resultSet.wasNull(), message + " (getDouble should not be NULL)");
    assertDoubleValue(doubleValue, expected, message + " (getDouble)");

    float floatValue = resultSet.getFloat(columnIndex);
    assertFalse(resultSet.wasNull(), message + " (getFloat should not be NULL)");
    assertFloatValue(floatValue, expected, message + " (getFloat)");
  }

  private static void assertFiniteDouble(double actual, double expected, String message) {
    assertTrue(Double.isFinite(actual), message + " (actual is not finite)");
    assertEquals(expected, actual, toleranceFor(expected), message);
  }

  private static void assertDoubleValue(double actual, double expected, String message) {
    if (Double.isNaN(expected)) {
      assertTrue(Double.isNaN(actual), message + " (expected NaN)");
      return;
    }
    if (Double.isInfinite(expected)) {
      assertEquals(expected, actual, message + " (expected infinity)");
      return;
    }
    assertFiniteDouble(actual, expected, message);
  }

  private static void assertFloatValue(float actual, double expected, String message) {
    float expectedFloat = (float) expected;
    if (Float.isNaN(expectedFloat)) {
      assertTrue(Float.isNaN(actual), message + " (expected NaN)");
      return;
    }
    if (Float.isInfinite(expectedFloat)) {
      assertEquals(expectedFloat, actual, message + " (expected infinity)");
      return;
    }
    assertEquals(expectedFloat, actual, floatToleranceFor(expectedFloat), message);
  }

  private static boolean approximatelyEquals(double actual, double expected) {
    return Double.isFinite(actual) && Math.abs(actual - expected) <= toleranceFor(expected);
  }

  private static double toleranceFor(double expected) {
    double ulpTolerance = Math.ulp(expected) * 8;
    double relativeTolerance = Math.abs(expected) * 1e-12;
    return Math.max(ulpTolerance, relativeTolerance);
  }

  private static float floatToleranceFor(float expected) {
    float ulpTolerance = Math.ulp(expected) * 8;
    float relativeTolerance = Math.abs(expected) * 1e-6f;
    return Math.max(ulpTolerance, relativeTolerance);
  }
}
