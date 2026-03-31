package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class BinaryTests extends SnowflakeIntegrationTestBase {
  private static final String BINARY_TYPE = "BINARY";
  private static final int LARGE_RESULT_SET_SIZE = 30_000;
  private static final int SEQUENTIAL_BINARY_TEXT_WIDTH = 10;

  @Test
  public void shouldCastBinaryValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=',
    // 'BASE64')::BINARY" is executed
    String sql =
        "SELECT TO_BINARY('48656C6C6F', 'HEX')::"
            + BINARY_TYPE
            + ", TO_BINARY('V29ybGQ=', 'BASE64')::"
            + BINARY_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then All values should be returned as appropriate binary type
          List<byte[]> row = assertBinaryTypeRow(resultSet, 2);

          // And the result should contain binary values:
          assertBinaryRowEquals(
              row, Arrays.asList(bytesFromHex("48656C6C6F"), bytesFromHex("576F726C64")));
        });
  }

  @Test
  public void shouldSelectBinaryLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Queries selecting binary literals are executed:
    String sql =
        "SELECT X'48656C6C6F'::"
            + BINARY_TYPE
            + ", TO_BINARY('48656C6C6F', 'HEX')::"
            + BINARY_TYPE
            + ", TO_BINARY('ASNFZ4mrze8=', 'BASE64')::"
            + BINARY_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the results should contain expected binary values
          assertSingleRow(
              resultSet,
              Arrays.asList(
                  bytesFromHex("48656C6C6F"),
                  bytesFromHex("48656C6C6F"),
                  bytesFromHex("0123456789ABCDEF")));
        });
  }

  @Test
  public void shouldHandleBinaryCornerCaseValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query selecting corner case binary literals is executed
    String sql =
        "SELECT X''::"
            + BINARY_TYPE
            + ", X'00'::"
            + BINARY_TYPE
            + ", X'FF'::"
            + BINARY_TYPE
            + ", X'0000000000'::"
            + BINARY_TYPE
            + ", X'FFFFFFFFFF'::"
            + BINARY_TYPE
            + ", X'48006500'::"
            + BINARY_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the result should contain expected corner case binary values
          assertSingleRow(
              resultSet,
              Arrays.asList(
                  bytesFromHex(""),
                  bytesFromHex("00"),
                  bytesFromHex("FF"),
                  bytesFromHex("0000000000"),
                  bytesFromHex("FFFFFFFFFF"),
                  bytesFromHex("48006500")));
        });
  }

  @Test
  public void shouldHandleNULLBinaryValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
    String sql = "SELECT NULL::" + BINARY_TYPE + ", X'ABCD', NULL::" + BINARY_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain [NULL, 0xABCD, NULL]
          assertSingleRow(resultSet, Arrays.asList(null, bytesFromHex("ABCD"), null));
        });
  }

  @Test
  public void shouldSelectBinaryValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with BINARY column is created
    String tableName = createTempTable(connection, "ud_binary_", "col " + BINARY_TYPE);

    // And The table is populated with binary values [X'48656C6C6F', X'576F726C64',
    // X'0123456789ABCDEF']
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (X'48656C6C6F'), (X'576F726C64'), (X'0123456789ABCDEF')");

    // When Query "SELECT * FROM {table} ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY col",
        resultSet -> {
          // Then the result should contain binary values in order:
          assertRowsInOrder(
              resultSet,
              Arrays.asList(
                  bytesFromHex("0123456789ABCDEF"),
                  bytesFromHex("48656C6C6F"),
                  bytesFromHex("576F726C64")));
        });
  }

  @Test
  public void shouldSelectCornerCaseBinaryValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with BINARY column is created
    String tableName = createTempTable(connection, "ud_binary_", "col " + BINARY_TYPE);

    // And The table is populated with corner case binary values
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (X''), (X'00'), (X'FF'), (X'000000'), (X'48006500')");

    // When Query "SELECT * FROM {table} ORDER BY 1" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY 1",
        resultSet -> {
          // Then the result should contain the inserted corner case binary values
          assertRowsInOrder(
              resultSet,
              Arrays.asList(
                  bytesFromHex(""),
                  bytesFromHex("00"),
                  bytesFromHex("000000"),
                  bytesFromHex("48006500"),
                  bytesFromHex("FF")));
        });
  }

  @Test
  public void shouldSelectNULLBinaryValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with BINARY column is created
    String tableName = createTempTable(connection, "ud_binary_", "col " + BINARY_TYPE);

    // And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
    execute(connection, "INSERT INTO " + tableName + " VALUES (NULL), (X'ABCD'), (NULL)");

    // When Query "SELECT * FROM {table}" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName,
        resultSet -> {
          int rowCount = 0;
          int nullCount = 0;
          int nonNullCount = 0;
          while (resultSet.next()) {
            rowCount++;
            byte[] actual = resultSet.getBytes(1);
            if (actual == null) {
              nullCount++;
              assertTrue(resultSet.wasNull(), "NULL binary value should set wasNull()");
            } else {
              nonNullCount++;
              assertArrayEquals(bytesFromHex("ABCD"), actual, "Unexpected non-NULL binary value");
              assertFalse(resultSet.wasNull(), "Non-NULL binary value should not set wasNull()");
            }
          }

          // Then there are 3 rows returned
          assertEquals(3, rowCount, "Unexpected row count for " + BINARY_TYPE);

          // And 2 rows should contain NULL values
          assertEquals(2, nullCount, "Unexpected NULL row count for " + BINARY_TYPE);

          // And 1 row should contain 0xABCD
          assertEquals(1, nonNullCount, "Unexpected non-NULL row count for " + BINARY_TYPE);
        });
  }

  @Test
  public void shouldSelectBinaryWithSpecifiedLengthFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
    String tableName =
        createTempTable(
            connection, "ud_binary_len_", "bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY");

    // And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')");

    // When Query "SELECT * FROM {table}" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName,
        resultSet -> {
          // Then Result should contain binary values with correct lengths
          assertTrue(resultSet.next(), "Expected one row for type: " + BINARY_TYPE);
          assertBinaryColumn(
              resultSet, 1, bytesFromHex("0102030405"), "Column 1 mismatch for " + BINARY_TYPE);
          assertBinaryColumn(
              resultSet,
              2,
              bytesFromHex("01020304050607080910"),
              "Column 2 mismatch for " + BINARY_TYPE);
          assertBinaryColumn(
              resultSet, 3, bytesFromHex("48656C6C6F"), "Column 3 mismatch for " + BINARY_TYPE);
          assertEquals(5, resultSet.getBytes(1).length, "Unexpected length for bin5");
          assertFalse(resultSet.wasNull(), "bin5 length should not be NULL");
          assertEquals(10, resultSet.getBytes(2).length, "Unexpected length for bin10");
          assertFalse(resultSet.wasNull(), "bin10 length should not be NULL");
          assertEquals(5, resultSet.getBytes(3).length, "Unexpected length for bin_default");
          assertFalse(resultSet.wasNull(), "bin_default length should not be NULL");
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + BINARY_TYPE);
        });
  }

  @Test
  public void shouldSelectBinaryLiteralsUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values
    // [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
    withPreparedQueryResult(
        connection,
        "SELECT ?::" + BINARY_TYPE + ", ?::" + BINARY_TYPE + ", ?::" + BINARY_TYPE,
        ps -> {
          ps.setBytes(1, bytesFromHex("48656C6C6F"));
          ps.setBytes(2, bytesFromHex("576F726C64"));
          ps.setBytes(3, bytesFromHex("0123456789ABCDEF"));
        },
        resultSet -> {
          // Then the result should contain:
          assertSingleRow(
              resultSet,
              Arrays.asList(
                  bytesFromHex("48656C6C6F"),
                  bytesFromHex("576F726C64"),
                  bytesFromHex("0123456789ABCDEF")));
        });
  }

  @Test
  public void shouldInsertBinaryUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with BINARY column exists
    String tableName = createTempTable(connection, "ud_binary_bind_", "id INT, col " + BINARY_TYPE);

    // When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?, ?)")) {
      preparedStatement.setInt(1, 1);
      preparedStatement.setBytes(2, bytesFromHex("48656C6C6F"));
      preparedStatement.execute();

      preparedStatement.setInt(1, 2);
      preparedStatement.setBytes(2, bytesFromHex("576F726C64"));
      preparedStatement.execute();

      preparedStatement.setInt(1, 3);
      preparedStatement.setBytes(2, bytesFromHex("00"));
      preparedStatement.execute();

      preparedStatement.setInt(1, 4);
      preparedStatement.setBytes(2, bytesFromHex("FF"));
      preparedStatement.execute();

      preparedStatement.setInt(1, 5);
      preparedStatement.setBytes(2, bytesFromHex(""));
      preparedStatement.execute();
    }

    // And Query "SELECT * FROM {table}" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY id",
        resultSet -> {
          // Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
          assertRowsInOrder(
              resultSet,
              Arrays.asList(
                  bytesFromHex("48656C6C6F"),
                  bytesFromHex("576F726C64"),
                  bytesFromHex("00"),
                  bytesFromHex("FF"),
                  bytesFromHex("")));
        });
  }

  @Test
  public void shouldBindCornerCaseBinaryValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
    assertSingleBoundBinaryValue(connection, bytesFromHex(""));
    assertSingleBoundBinaryValue(connection, bytesFromHex("00"));
    assertSingleBoundBinaryValue(connection, bytesFromHex("FF"));
    assertSingleBoundBinaryValue(connection, bytesFromHex("48006500"));

    // Then the result should match the bound corner case value
    withPreparedQueryResult(
        connection,
        "SELECT ?::" + BINARY_TYPE,
        ps -> ps.setNull(1, Types.BINARY),
        resultSet -> {
          assertTrue(resultSet.next(), "Expected one row for type: " + BINARY_TYPE);
          assertBinaryColumn(resultSet, 1, null, "NULL value mismatch for " + BINARY_TYPE);
          assertFalse(resultSet.next(), "Expected exactly one row for type: " + BINARY_TYPE);
        });
  }

  @Test
  public void shouldDownloadBinaryDataInMultipleChunksUsingGENERATOR() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS
    // bin_val FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
    String sql =
        "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), "
            + SEQUENTIAL_BINARY_TEXT_WIDTH
            + ", '0'), 'UTF-8') AS bin_val "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY id";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          List<byte[]> actualValues = new ArrayList<>();
          int expected = 0;
          while (resultSet.next()) {
            long id = resultSet.getLong(1);
            assertFalse(resultSet.wasNull(), "Row id should not be NULL");
            assertEquals(expected, id, "Unexpected generated row id");
            byte[] actualValue = resultSet.getBytes(2);
            assertFalse(resultSet.wasNull(), "Generated binary value should not be NULL");
            actualValues.add(actualValue);
            expected++;
          }

          // Then there are 30000 rows returned
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + BINARY_TYPE);

          // And all returned binary values should match the generated values in order
          for (int i = 0; i < actualValues.size(); i++) {
            assertArrayEquals(
                sequentialBinaryValue(i),
                actualValues.get(i),
                "Generated binary value mismatch at row " + i);
          }
        });
  }

  @Test
  public void shouldDownloadBinaryDataInMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with (bin_data BINARY) exists with 30000 sequential binary values
    String tableName = createTempTable(connection, "ud_binary_large_", "bin_data " + BINARY_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT TO_BINARY(LPAD(TO_VARCHAR(seq8()), "
            + SEQUENTIAL_BINARY_TEXT_WIDTH
            + ", '0'), 'UTF-8') "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
    withQueryResult(
        connection,
        "SELECT bin_data FROM " + tableName + " ORDER BY bin_data",
        resultSet -> {
          List<byte[]> actualValues = new ArrayList<>();
          int expected = 0;
          while (resultSet.next()) {
            byte[] actualValue = resultSet.getBytes(1);
            assertFalse(resultSet.wasNull(), "Inserted binary value should not be NULL");
            actualValues.add(actualValue);
            expected++;
          }

          // Then there are 30000 rows returned
          assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + BINARY_TYPE);

          // And all returned binary values should match the inserted values in order
          for (int i = 0; i < actualValues.size(); i++) {
            assertArrayEquals(
                sequentialBinaryValue(i),
                actualValues.get(i),
                "Inserted binary value mismatch at row " + i);
          }
        });
  }

  private static List<byte[]> assertBinaryTypeRow(ResultSet resultSet, int columnCount)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: " + BINARY_TYPE);
    List<byte[]> values = new ArrayList<>();
    for (int i = 1; i <= columnCount; i++) {
      Object objectValue = resultSet.getObject(i);
      assertInstanceOf(
          byte[].class,
          objectValue,
          "Column " + i + " getObject should return byte[] for " + BINARY_TYPE);
      assertFalse(resultSet.wasNull(), "Column " + i + " should not be NULL for " + BINARY_TYPE);
      byte[] bytesValue = resultSet.getBytes(i);
      assertFalse(resultSet.wasNull(), "Column " + i + " getBytes should not be NULL");
      assertArrayEquals(
          (byte[]) objectValue, bytesValue, "Column " + i + " binary getter mismatch");
      values.add(bytesValue);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: " + BINARY_TYPE);
    return values;
  }

  private static void assertSingleRow(ResultSet resultSet, List<byte[]> expectedValues)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: " + BINARY_TYPE);
    for (int i = 0; i < expectedValues.size(); i++) {
      assertBinaryColumn(
          resultSet,
          i + 1,
          expectedValues.get(i),
          "Column " + (i + 1) + " mismatch for " + BINARY_TYPE);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: " + BINARY_TYPE);
  }

  private static void assertRowsInOrder(ResultSet resultSet, List<byte[]> expectedValues)
      throws Exception {
    for (int i = 0; i < expectedValues.size(); i++) {
      assertTrue(resultSet.next(), "Missing row " + i + " for " + BINARY_TYPE);
      assertBinaryColumn(
          resultSet, 1, expectedValues.get(i), "Value mismatch for " + BINARY_TYPE + ", row " + i);
    }
    assertFalse(resultSet.next(), "Unexpected extra rows for " + BINARY_TYPE);
  }

  private static void assertBinaryColumn(
      ResultSet resultSet, int columnIndex, byte[] expected, String message) throws Exception {
    Object objectValue = resultSet.getObject(columnIndex);
    if (expected == null) {
      assertNull(objectValue, message + " (getObject should be NULL)");
      assertTrue(resultSet.wasNull(), message + " (getObject should set wasNull)");
      assertNull(resultSet.getBytes(columnIndex), message + " (getBytes should be NULL)");
      assertTrue(resultSet.wasNull(), message + " (getBytes should set wasNull)");
      return;
    }

    assertInstanceOf(byte[].class, objectValue, message + " (getObject should return byte[])");
    assertArrayEquals(expected, (byte[]) objectValue, message + " (getObject)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");

    assertArrayEquals(expected, resultSet.getBytes(columnIndex), message + " (getBytes)");
    assertFalse(resultSet.wasNull(), message + " (getBytes should not be NULL)");
  }

  private static void assertBinaryRowEquals(
      List<byte[]> actualValues, List<byte[]> expectedValues) {
    assertEquals(expectedValues.size(), actualValues.size(), "Unexpected binary column count");
    for (int i = 0; i < expectedValues.size(); i++) {
      byte[] expected = expectedValues.get(i);
      byte[] actual = actualValues.get(i);
      if (expected == null) {
        assertNull(actual, "Expected NULL binary value at column " + (i + 1));
      } else {
        assertArrayEquals(expected, actual, "Unexpected binary value at column " + (i + 1));
      }
    }
  }

  private static void assertSingleBoundBinaryValue(Connection connection, byte[] expected)
      throws Exception {
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("SELECT ?::" + BINARY_TYPE)) {
      preparedStatement.setBytes(1, expected);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: " + BINARY_TYPE);
        assertBinaryColumn(resultSet, 1, expected, "Bound value mismatch for " + BINARY_TYPE);
        assertFalse(resultSet.next(), "Expected exactly one row for type: " + BINARY_TYPE);
      }
    }
  }

  private static byte[] sequentialBinaryValue(int value) {
    return String.format("%0" + SEQUENTIAL_BINARY_TEXT_WIDTH + "d", value)
        .getBytes(StandardCharsets.UTF_8);
  }

  private static byte[] bytesFromHex(String hex) {
    int length = hex.length();
    byte[] bytes = new byte[length / 2];
    for (int i = 0; i < length; i += 2) {
      int high = Character.digit(hex.charAt(i), 16);
      int low = Character.digit(hex.charAt(i + 1), 16);
      bytes[i / 2] = (byte) ((high << 4) + low);
    }
    return bytes;
  }
}
