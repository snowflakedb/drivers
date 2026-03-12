package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Stream;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

public class DecfloatTests extends SnowflakeIntegrationTestBase {
  private static final int LARGE_RESULT_SET_SIZE = 20_000;

  @Test
  public void shouldCastDecfloatValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT,
    // '12345678901234567890123456789012345678'::DECFLOAT" is executed
    String sql =
        "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, "
            + "'12345678901234567890123456789012345678'::DECFLOAT";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then All values should be returned as appropriate type
          List<BigDecimal> row = assertDecfloatTypes(resultSet, 4);

          // And Values should maintain full 38-digit precision
          List<BigDecimal> expected =
              Arrays.asList(
                  new BigDecimal("0"),
                  new BigDecimal("123.456"),
                  new BigDecimal("1.23E+37"),
                  new BigDecimal("12345678901234567890123456789012345678"));
          assertEquals(expected.size(), row.size(), "Column count mismatch for DECFLOAT");
          for (int i = 0; i < expected.size(); i++) {
            assertEquals(
                0,
                expected.get(i).compareTo(row.get(i)),
                "Column " + (i + 1) + " value mismatch for DECFLOAT");
          }
          assertEquals(38, row.get(3).precision(), "Column 4 should preserve 38 digits");
        });
  }

  @Test
  public void shouldSelectDecfloatLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT,
    // -987.654321::DECFLOAT" is executed
    String sql =
        "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
          assertSingleRowWithNulls(
              resultSet,
              Arrays.asList(
                  new BigDecimal("0"),
                  new BigDecimal("1.5"),
                  new BigDecimal("-1.5"),
                  new BigDecimal("123.456789"),
                  new BigDecimal("-987.654321")));
        });
  }

  @Test
  public void shouldHandleFull38DigitPrecisionValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT,
    // '1.2345678901234567890123456789012345678E+100'::DECFLOAT,
    // '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
    String sql =
        "SELECT '12345678901234567890123456789012345678'::DECFLOAT, "
            + "'1.2345678901234567890123456789012345678E+100'::DECFLOAT, "
            + "'1.2345678901234567890123456789012345678E-100'::DECFLOAT";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should preserve all 38 digits for each value
          assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
          assertDecfloatColumn(
              resultSet,
              1,
              new BigDecimal("12345678901234567890123456789012345678"),
              "Column 1 mismatch for DECFLOAT");
          assertDecfloatColumn(
              resultSet,
              2,
              new BigDecimal("1.2345678901234567890123456789012345678E+100"),
              "Column 2 mismatch for DECFLOAT");
          assertDecfloatColumn(
              resultSet,
              3,
              new BigDecimal("1.2345678901234567890123456789012345678E-100"),
              "Column 3 mismatch for DECFLOAT");

          assertEquals(
              38, resultSet.getBigDecimal(1).precision(), "Column 1 should preserve 38 digits");
          assertFalse(resultSet.wasNull(), "Column 1 should not be NULL for DECFLOAT");
          assertEquals(
              38, resultSet.getBigDecimal(2).precision(), "Column 2 should preserve 38 digits");
          assertFalse(resultSet.wasNull(), "Column 2 should not be NULL for DECFLOAT");
          assertEquals(
              38, resultSet.getBigDecimal(3).precision(), "Column 3 should preserve 38 digits");
          assertFalse(resultSet.wasNull(), "Column 3 should not be NULL for DECFLOAT");
          assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
        });
  }

  private static Stream<Arguments> caseExponentLiteralValues() {
    return Stream.of(
        Arguments.of(
            "max positive and min positive",
            "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT",
            Arrays.asList(new BigDecimal("1E+16384"), new BigDecimal("1E-16383"))),
        Arguments.of(
            "large negative and small positive",
            "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT",
            Arrays.asList(new BigDecimal("-1.234E+8000"), new BigDecimal("9.876E-8000"))));
  }

  @ParameterizedTest
  @MethodSource("caseExponentLiteralValues")
  public void shouldHandleCaseExponentValuesFromLiterals(
      String caseName, String sql, List<BigDecimal> expectedValues) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain [<expected_values>]
          assertSingleRowWithNulls(resultSet, expectedValues);
        });
  }

  @Test
  public void shouldHandleNULLValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
    String sql = "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NULL, 42.5, NULL]
          assertSingleRowWithNulls(resultSet, Arrays.asList(null, new BigDecimal("42.5"), null));
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGenerator() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is
    // executed
    String sql =
        "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY id";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate
          // type
          int expected = 0;
          while (resultSet.next()) {
            assertDecfloatColumn(
                resultSet,
                1,
                BigDecimal.valueOf(expected),
                "Value mismatch for DECFLOAT, row " + expected);
            expected++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for DECFLOAT");
        });
  }

  @Test
  public void shouldSelectDecfloatsFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (0), (123.456), (-789.012), (1.23e20), (-9.87e-15)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(
                  new BigDecimal("-789.012"),
                  new BigDecimal("-9.87E-15"),
                  new BigDecimal("0"),
                  new BigDecimal("123.456"),
                  new BigDecimal("1.23E+20")));
        });
  }

  @Test
  public void shouldHandleFull38DigitPrecisionValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists with values
    // [12345678901234567890123456789012345678, 1.2345678901234567890123456789012345678E+100,
    // 1.2345678901234567890123456789012345678E-100]
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('12345678901234567890123456789012345678'::DECFLOAT), "
            + "('1.2345678901234567890123456789012345678E+100'::DECFLOAT), "
            + "('1.2345678901234567890123456789012345678E-100'::DECFLOAT)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should preserve all 38 digits for each value
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(
                  new BigDecimal("1.2345678901234567890123456789012345678E-100"),
                  new BigDecimal("12345678901234567890123456789012345678"),
                  new BigDecimal("1.2345678901234567890123456789012345678E+100")));
        });
  }

  @Test
  public void shouldHandleExtremeExponentValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000,
    // 9.876E-8000]
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('1E+16384'::DECFLOAT), "
            + "('1E-16383'::DECFLOAT), "
            + "('-1.234E+8000'::DECFLOAT), "
            + "('9.876E-8000'::DECFLOAT)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(
                  new BigDecimal("-1.234E+8000"),
                  new BigDecimal("1E-16383"),
                  new BigDecimal("9.876E-8000"),
                  new BigDecimal("1E+16384")));
        });
  }

  @Test
  public void shouldHandleNULLValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection, "INSERT INTO " + tableName + " VALUES (NULL), (123.456), (NULL), (-789.012)");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col NULLS FIRST";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [NULL, 123.456, NULL, -789.012]
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(null, null, new BigDecimal("-789.012"), new BigDecimal("123.456")));
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists with values from 0 to 19999
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT seq8()::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate
          // type
          int expected = 0;
          while (resultSet.next()) {
            assertDecfloatColumn(
                resultSet,
                1,
                BigDecimal.valueOf(expected),
                "Value mismatch for DECFLOAT, row " + expected);
            expected++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for DECFLOAT");
        });
  }

  @Test
  public void shouldSelectDecfloatUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT
    // values [123.456, -789.012, 42.0]
    withPreparedQueryResult(
        connection,
        "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT",
        ps -> {
          ps.setBigDecimal(1, new BigDecimal("123.456"));
          ps.setBigDecimal(2, new BigDecimal("-789.012"));
          ps.setBigDecimal(3, new BigDecimal("42.0"));
        },
        resultSet -> {
          // Then Result should contain [123.456, -789.012, 42.0]
          assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
          assertDecfloatColumn(
              resultSet, 1, new BigDecimal("123.456"), "Column 1 mismatch for DECFLOAT");
          assertDecfloatColumn(
              resultSet, 2, new BigDecimal("-789.012"), "Column 2 mismatch for DECFLOAT");
          assertDecfloatColumn(
              resultSet, 3, new BigDecimal("42.0"), "Column 3 mismatch for DECFLOAT");
          assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
        });
  }

  @Test
  public void shouldSelectNullDecfloatUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT ?::DECFLOAT",
        ps -> ps.setNull(1, Types.DECIMAL),
        resultSet -> {
          // Then Result should contain [NULL]
          assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
          assertNull(resultSet.getObject(1), "Column 1 should be NULL for DECFLOAT");
          assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() for DECFLOAT");
          assertNull(resultSet.getBigDecimal(1), "Column 1 BigDecimal should be NULL for DECFLOAT");
          assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() after getBigDecimal");
          assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
        });
  }

  private static Stream<Arguments> caseDecfloatBindingValues() {
    return Stream.of(
        Arguments.of("max exponent", new BigDecimal("1E+16384")),
        Arguments.of("large negative exponent", new BigDecimal("-1.234E+8000")));
  }

  @ParameterizedTest
  @MethodSource("caseDecfloatBindingValues")
  public void shouldSelectCaseDecfloatUsingParameterBinding(String caseName, BigDecimal value)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::DECFLOAT" is executed with bound value <value>
    withPreparedQueryResult(
        connection,
        "SELECT ?::DECFLOAT",
        ps -> ps.setBigDecimal(1, value),
        resultSet -> {
          // Then Result should contain [<expected>]
          assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
          assertDecfloatColumn(resultSet, 1, value, "Value mismatch for DECFLOAT");
          assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
        });
  }

  @Test
  public void shouldInsertDecfloatUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");

    // When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      preparedStatement.setBigDecimal(1, new BigDecimal("0"));
      preparedStatement.execute();
      preparedStatement.setBigDecimal(1, new BigDecimal("123.456"));
      preparedStatement.execute();
      preparedStatement.setBigDecimal(1, new BigDecimal("-789.012"));
      preparedStatement.execute();
      preparedStatement.setNull(1, Types.DECIMAL);
      preparedStatement.execute();
    }

    // Then SELECT should return the same exact values
    String sql = "SELECT col FROM " + tableName + " ORDER BY col NULLS FIRST";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(
                  null,
                  new BigDecimal("-789.012"),
                  new BigDecimal("0"),
                  new BigDecimal("123.456")));
        });
  }

  @Test
  public void shouldInsertExtremeDecfloatValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with DECFLOAT column exists
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");

    // When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?::DECFLOAT)")) {
      preparedStatement.setString(1, "1E+16384");
      preparedStatement.execute();
      preparedStatement.setString(1, "1E-16383");
      preparedStatement.execute();
      preparedStatement.setString(1, "-1.234E+8000");
      preparedStatement.execute();
    }

    // And Query "SELECT * FROM <table>" is executed
    String sql = "SELECT col FROM " + tableName + " ORDER BY col";
    // Then SELECT should return the same exact values
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          assertSingleColumnRows(
              resultSet,
              Arrays.asList(
                  new BigDecimal("-1.234E+8000"),
                  new BigDecimal("1E-16383"),
                  new BigDecimal("1E+16384")));
        });
  }

  private static List<BigDecimal> assertDecfloatTypes(ResultSet resultSet, int columnCount)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
    List<BigDecimal> values = new ArrayList<>();
    for (int i = 1; i <= columnCount; i++) {
      Object objectValue = resultSet.getObject(i);
      assertInstanceOf(
          BigDecimal.class,
          objectValue,
          "Column " + i + " getObject should return BigDecimal for DECFLOAT");
      assertFalse(resultSet.wasNull(), "Column " + i + " should not be NULL for DECFLOAT");
      values.add((BigDecimal) objectValue);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
    return values;
  }

  private static void assertSingleRowWithNulls(ResultSet resultSet, List<BigDecimal> expectedValues)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
    for (int i = 0; i < expectedValues.size(); i++) {
      assertNullableDecfloatColumn(
          resultSet,
          i + 1,
          expectedValues.get(i),
          "Value mismatch for DECFLOAT, column " + (i + 1));
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
  }

  private static void assertSingleColumnRows(ResultSet resultSet, List<BigDecimal> expectedValues)
      throws Exception {
    for (int i = 0; i < expectedValues.size(); i++) {
      assertTrue(resultSet.next(), "Missing row " + i + " for DECFLOAT");
      assertNullableDecfloatColumn(
          resultSet, 1, expectedValues.get(i), "Value mismatch for DECFLOAT, row " + i);
    }
    assertFalse(resultSet.next(), "Unexpected extra rows for DECFLOAT");
  }

  private static void assertNullableDecfloatColumn(
      ResultSet resultSet, int columnIndex, BigDecimal expected, String message) throws Exception {
    if (expected == null) {
      assertNull(resultSet.getObject(columnIndex), message + " (expected NULL object)");
      assertTrue(resultSet.wasNull(), message + " (expected wasNull after getObject)");
      assertNull(resultSet.getBigDecimal(columnIndex), message + " (expected NULL BigDecimal)");
      assertTrue(resultSet.wasNull(), message + " (expected wasNull after getBigDecimal)");
      return;
    }
    assertDecfloatColumn(resultSet, columnIndex, expected, message);
  }

  private static void assertDecfloatColumn(
      ResultSet resultSet, int columnIndex, BigDecimal expected, String message) throws Exception {
    BigDecimal actualBigDecimal = resultSet.getBigDecimal(columnIndex);
    assertEquals(
        0, expected.compareTo(actualBigDecimal), message + " (getBigDecimal numeric value)");
    assertFalse(resultSet.wasNull(), message + " (getBigDecimal should not be NULL)");

    Object objectValue = resultSet.getObject(columnIndex);
    assertInstanceOf(
        BigDecimal.class, objectValue, message + " (getObject should return BigDecimal)");
    BigDecimal actualObjectValue = (BigDecimal) objectValue;
    assertEquals(0, expected.compareTo(actualObjectValue), message + " (getObject numeric value)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");
  }
}
