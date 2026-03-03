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
import java.sql.Statement;
import java.sql.Types;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class DecfloatTests extends SnowflakeIntegrationTestBase {
  private static final int LARGE_RESULT_SET_SIZE = 20_000;

  @Test
  public void shouldCastDecfloatValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT,
    // '12345678901234567890123456789012345678'::DECFLOAT" is executed
    // Then All values should be returned as appropriate type
    // And Values should maintain full 38-digit precision
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, "
            + "'12345678901234567890123456789012345678'::DECFLOAT";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");

      assertDecfloatColumn(resultSet, 1, new BigDecimal("0"), "Column 1 mismatch for DECFLOAT");
      assertDecfloatColumn(
          resultSet, 2, new BigDecimal("123.456"), "Column 2 mismatch for DECFLOAT");
      assertDecfloatColumn(
          resultSet, 3, new BigDecimal("1.23E+37"), "Column 3 mismatch for DECFLOAT");

      BigDecimal maxPrecision = new BigDecimal("12345678901234567890123456789012345678");
      assertDecfloatColumn(resultSet, 4, maxPrecision, "Column 4 mismatch for DECFLOAT");
      assertEquals(
          38, resultSet.getBigDecimal(4).precision(), "Column 4 should preserve 38 digits");
      assertFalse(resultSet.wasNull(), "Column 4 should not be NULL for DECFLOAT");
      assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
    }
  }

  @Test
  public void shouldSelectDecfloatLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT,
    // -987.654321::DECFLOAT" is executed
    // Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
    Connection connection = getDefaultConnection();
    assertSingleRowWithNulls(
        connection,
        "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT",
        Arrays.asList(
            new BigDecimal("0"),
            new BigDecimal("1.5"),
            new BigDecimal("-1.5"),
            new BigDecimal("123.456789"),
            new BigDecimal("-987.654321")));
  }

  @Test
  public void shouldHandleFull38DigitPrecisionValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT,
    // '1.2345678901234567890123456789012345678E+100'::DECFLOAT,
    // '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
    // Then Result should preserve all 38 digits for each value
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT '12345678901234567890123456789012345678'::DECFLOAT, "
            + "'1.2345678901234567890123456789012345678E+100'::DECFLOAT, "
            + "'1.2345678901234567890123456789012345678E-100'::DECFLOAT";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
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
    }
  }

  @Test
  public void shouldHandleExtremeExponentValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT" is executed
    // Then Result should contain [1E+16384, 1E-16383]
    // When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
    // Then Result should contain [-1.234E+8000, 9.876E-8000]
    Connection connection = getDefaultConnection();
    assertSingleRowWithNulls(
        connection,
        "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT",
        Arrays.asList(new BigDecimal("1E+16384"), new BigDecimal("1E-16383")));
    assertSingleRowWithNulls(
        connection,
        "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT",
        Arrays.asList(new BigDecimal("-1.234E+8000"), new BigDecimal("9.876E-8000")));
  }

  @Test
  public void shouldHandleNULLValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
    // Then Result should contain [NULL, 42.5, NULL]
    Connection connection = getDefaultConnection();
    String sql = "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT";
    assertSingleRowWithNulls(connection, sql, Arrays.asList(null, new BigDecimal("42.5"), null));
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGenerator() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is
    // executed
    // Then Result should contain consecutive numbers from 0 to 19999
    // And All values should be returned as appropriate type
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
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
    }
  }

  @Test
  public void shouldSelectDecfloatsFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (0), (123.456), (-789.012), (1.23e20), (-9.87e-15)");

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col",
        Arrays.asList(
            new BigDecimal("-789.012"),
            new BigDecimal("-9.87E-15"),
            new BigDecimal("0"),
            new BigDecimal("123.456"),
            new BigDecimal("1.23E+20")));
  }

  @Test
  public void shouldHandleFull38DigitPrecisionValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists with values
    // [12345678901234567890123456789012345678, 1.2345678901234567890123456789012345678E+100,
    // 1.2345678901234567890123456789012345678E-100]
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should preserve all 38 digits for each value
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('12345678901234567890123456789012345678'::DECFLOAT), "
            + "('1.2345678901234567890123456789012345678E+100'::DECFLOAT), "
            + "('1.2345678901234567890123456789012345678E-100'::DECFLOAT)");

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col",
        Arrays.asList(
            new BigDecimal("1.2345678901234567890123456789012345678E-100"),
            new BigDecimal("12345678901234567890123456789012345678"),
            new BigDecimal("1.2345678901234567890123456789012345678E+100")));
  }

  @Test
  public void shouldHandleExtremeExponentValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000,
    // 9.876E-8000]
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('1E+16384'::DECFLOAT), "
            + "('1E-16383'::DECFLOAT), "
            + "('-1.234E+8000'::DECFLOAT), "
            + "('9.876E-8000'::DECFLOAT)");

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col",
        Arrays.asList(
            new BigDecimal("-1.234E+8000"),
            new BigDecimal("1E-16383"),
            new BigDecimal("9.876E-8000"),
            new BigDecimal("1E+16384")));
  }

  @Test
  public void shouldHandleNULLValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain [NULL, 123.456, NULL, -789.012]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection, "INSERT INTO " + tableName + " VALUES (NULL), (123.456), (NULL), (-789.012)");

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col NULLS FIRST",
        Arrays.asList(null, null, new BigDecimal("-789.012"), new BigDecimal("123.456")));
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists with values from 0 to 19999
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain consecutive numbers from 0 to 19999
    // And All values should be returned as appropriate type
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT seq8()::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT col FROM " + tableName + " ORDER BY col")) {
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
    }
  }

  @Test
  public void shouldSelectDecfloatUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT
    // values [123.456, -789.012, 42.0]
    // Then Result should contain [123.456, -789.012, 42.0]
    // When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
    // Then Result should contain [NULL]
    Connection connection = getDefaultConnection();
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT")) {
      preparedStatement.setBigDecimal(1, new BigDecimal("123.456"));
      preparedStatement.setBigDecimal(2, new BigDecimal("-789.012"));
      preparedStatement.setBigDecimal(3, new BigDecimal("42.0"));
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
        assertDecfloatColumn(
            resultSet, 1, new BigDecimal("123.456"), "Column 1 mismatch for DECFLOAT");
        assertDecfloatColumn(
            resultSet, 2, new BigDecimal("-789.012"), "Column 2 mismatch for DECFLOAT");
        assertDecfloatColumn(
            resultSet, 3, new BigDecimal("42.0"), "Column 3 mismatch for DECFLOAT");
        assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
      }
    }

    try (PreparedStatement preparedStatement = connection.prepareStatement("SELECT ?::DECFLOAT")) {
      preparedStatement.setNull(1, Types.DECIMAL);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
        assertNull(resultSet.getObject(1), "Column 1 should be NULL for DECFLOAT");
        assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() for DECFLOAT");
        assertNull(resultSet.getBigDecimal(1), "Column 1 BigDecimal should be NULL for DECFLOAT");
        assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() after getBigDecimal");
        assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
      }
    }
  }

  @Test
  public void shouldSelectExtremeDecfloatValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT ?::DECFLOAT" is executed with bound value 1E+16384
    // Then Result should contain [1E+16384]
    // When Query "SELECT ?::DECFLOAT" is executed with bound value -1.234E+8000
    // Then Result should contain [-1.234E+8000]
    Connection connection = getDefaultConnection();
    try (PreparedStatement preparedStatement = connection.prepareStatement("SELECT ?::DECFLOAT")) {
      preparedStatement.setBigDecimal(1, new BigDecimal("1E+16384"));
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
        assertDecfloatColumn(
            resultSet, 1, new BigDecimal("1E+16384"), "Value mismatch for DECFLOAT");
        assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
      }
      preparedStatement.setBigDecimal(1, new BigDecimal("-1.234E+8000"));
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: DECFLOAT");
        assertDecfloatColumn(
            resultSet, 1, new BigDecimal("-1.234E+8000"), "Value mismatch for DECFLOAT");
        assertFalse(resultSet.next(), "Expected exactly one row for type: DECFLOAT");
      }
    }
  }

  @Test
  public void shouldInsertDecfloatUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists
    // When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
    // Then SELECT should return the same exact values
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
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

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col NULLS FIRST",
        Arrays.asList(
            null, new BigDecimal("-789.012"), new BigDecimal("0"), new BigDecimal("123.456")));
  }

  @Test
  public void shouldInsertExtremeDecfloatValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DECFLOAT column exists
    // When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
    // And Query "SELECT * FROM <table>" is executed
    // Then SELECT should return the same exact values
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_decfloat_", "col DECFLOAT");
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?::DECFLOAT)")) {
      preparedStatement.setString(1, "1E+16384");
      preparedStatement.execute();
      preparedStatement.setString(1, "1E-16383");
      preparedStatement.execute();
      preparedStatement.setString(1, "-1.234E+8000");
      preparedStatement.execute();
    }

    assertSingleColumnRows(
        connection,
        tableName,
        "ORDER BY col",
        Arrays.asList(
            new BigDecimal("-1.234E+8000"),
            new BigDecimal("1E-16383"),
            new BigDecimal("1E+16384")));
  }

  private static void assertSingleRowWithNulls(
      Connection connection, String sql, List<BigDecimal> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
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
  }

  private static void assertSingleColumnRows(
      Connection connection,
      String tableName,
      String orderByClause,
      List<BigDecimal> expectedValues)
      throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT col FROM " + tableName + " " + orderByClause)) {
      for (int i = 0; i < expectedValues.size(); i++) {
        assertTrue(resultSet.next(), "Missing row " + i + " for DECFLOAT");
        assertNullableDecfloatColumn(
            resultSet, 1, expectedValues.get(i), "Value mismatch for DECFLOAT, row " + i);
      }
      assertFalse(resultSet.next(), "Unexpected extra rows for DECFLOAT");
    }
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
