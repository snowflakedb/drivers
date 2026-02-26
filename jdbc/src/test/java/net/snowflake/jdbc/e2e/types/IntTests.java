package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class IntTests extends SnowflakeIntegrationTestBase {
  private static final String INT_TYPE = "INT";
  private static final String SMALL_INT = "-99999999999999999999999999999999999999";
  private static final String LARGE_INT = "99999999999999999999999999999999999999";
  private static final int LARGE_RESULT_SET_SIZE = 50_000;

  @Test
  public void shouldCastIntegerValuesToAppropriateTypeForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
    // Then All values should be returned as appropriate type
    // And No precision loss should occur
    Connection connection = getDefaultConnection();
    String sql =
        String.format("SELECT 0::%1$s, 1000000::%1$s, 9223372036854775807::%1$s", INT_TYPE);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + INT_TYPE);
      assertAllIntegerGettersInRange(resultSet, 1, 0L, "Column 1 mismatch for " + INT_TYPE);
      assertAllIntegerGettersInRange(resultSet, 2, 1_000_000L, "Column 2 mismatch for " + INT_TYPE);
      assertAllIntegerGettersInRange(
          resultSet, 3, Long.MAX_VALUE, "Column 3 mismatch for " + INT_TYPE);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + INT_TYPE);
    }
  }

  @Test
  public void shouldSelectIntegerValuesForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT <query_values>" is executed
    // Then Result should contain integers <expected_values>
    Connection connection = getDefaultConnection();
    assertSingleRow(connection, String.format("SELECT 0::%1$s", INT_TYPE), Arrays.asList(0L));
    assertSingleRow(
        connection,
        String.format("SELECT -128::%1$s, 127::%1$s, 255::%1$s", INT_TYPE),
        Arrays.asList(-128L, 127L, 255L));
    assertSingleRow(
        connection,
        String.format("SELECT -32768::%1$s, 32767::%1$s, 65535::%1$s", INT_TYPE),
        Arrays.asList((long) Short.MIN_VALUE, (long) Short.MAX_VALUE, 65535L));
    assertSingleRow(
        connection,
        String.format("SELECT -2147483648::%1$s, 2147483647::%1$s, 4294967295::%1$s", INT_TYPE),
        Arrays.asList((long) Integer.MIN_VALUE, (long) Integer.MAX_VALUE, 4294967295L));
    assertSingleRow(
        connection,
        String.format("SELECT -9223372036854775808::%1$s, 9223372036854775807::%1$s", INT_TYPE),
        Arrays.asList(Long.MIN_VALUE, Long.MAX_VALUE));
  }

  @Test
  public void shouldHandleLargeIntegerValuesForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT -99999999999999999999999999999999999999::<type>,
    // 99999999999999999999999999999999999999::<type>" is executed
    // Then Result should contain integers [-99999999999999999999999999999999999999,
    // 99999999999999999999999999999999999999]
    Connection connection = getDefaultConnection();
    String sql = String.format("SELECT %1$s::%3$s, %2$s::%3$s", SMALL_INT, LARGE_INT, INT_TYPE);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + INT_TYPE);
      assertInt38NumberGetters(resultSet, 1, SMALL_INT, "Column 1 mismatch for " + INT_TYPE);
      assertInt38NumberGetters(resultSet, 2, LARGE_INT, "Column 2 mismatch for " + INT_TYPE);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + INT_TYPE);
    }
  }

  @Test
  public void shouldHandleNULLValuesForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
    // Then Result should contain [NULL, 42, NULL]
    Connection connection = getDefaultConnection();
    String sql = String.format("SELECT NULL::%1$s, 42::%1$s, NULL::%1$s", INT_TYPE);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + INT_TYPE);
      assertNull(resultSet.getObject(1), "Column 1 should be NULL for " + INT_TYPE);
      assertTrue(resultSet.wasNull(), "Column 1 should set wasNull() for " + INT_TYPE);
      assertEquals(
          0L, resultSet.getLong(1), "Column 1 getLong should return 0 for NULL " + INT_TYPE);
      assertTrue(resultSet.wasNull(), "Column 1 getLong should set wasNull() for " + INT_TYPE);
      assertAllIntegerGettersInRange(resultSet, 2, 42L, "Column 2 mismatch for " + INT_TYPE);
      assertNull(resultSet.getObject(3), "Column 3 should be NULL for " + INT_TYPE);
      assertTrue(resultSet.wasNull(), "Column 3 should set wasNull() for " + INT_TYPE);
      assertEquals(
          0L, resultSet.getLong(3), "Column 3 getLong should return 0 for NULL " + INT_TYPE);
      assertTrue(resultSet.wasNull(), "Column 3 getLong should set wasNull() for " + INT_TYPE);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + INT_TYPE);
    }
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY
    // id" is executed
    // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT seq8()::%1$s as id FROM TABLE(GENERATOR(ROWCOUNT => %2$d)) v ORDER BY id",
            INT_TYPE, LARGE_RESULT_SET_SIZE);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      int expected = 0;
      while (resultSet.next()) {
        assertAllIntegerGettersInRange(
            resultSet, 1, expected, "Value mismatch for " + INT_TYPE + ", row " + expected);
        expected++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + INT_TYPE);
    }
  }

  @Test
  public void shouldSelectValuesFromTableForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with <type> column exists with values <insert_values>
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain integers <expected_values>
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_int_", "col " + INT_TYPE);

    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (0), (1), (127), (255), (32767), (65535), (2147483647), (4294967295), (9223372036854775807)");
    assertSingleColumnRows(
        connection,
        tableName,
        Arrays.asList(
            0L,
            1L,
            127L,
            255L,
            (long) Short.MAX_VALUE,
            65535L,
            (long) Integer.MAX_VALUE,
            4294967295L,
            Long.MAX_VALUE));

    execute(connection, "TRUNCATE TABLE " + tableName);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (-1), (-128), (-32768), (-2147483648), (-9223372036854775808)");
    assertSingleColumnRows(
        connection,
        tableName,
        Arrays.asList(
            Long.MIN_VALUE, (long) Integer.MIN_VALUE, (long) Short.MIN_VALUE, -128L, -1L));

    execute(connection, "TRUNCATE TABLE " + tableName);
    execute(connection, "INSERT INTO " + tableName + " VALUES (0), (NULL), (42)");
    assertSingleColumnRows(connection, tableName, Arrays.asList(0L, 42L, null));
  }

  @Test
  public void shouldSelectLargeIntegerValuesFromTableForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with <type> column exists with values [-99999999999999999999999999999999999999,
    // 99999999999999999999999999999999999999]
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain integers [-99999999999999999999999999999999999999,
    // 99999999999999999999999999999999999999]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_int_", "col " + INT_TYPE);
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (" + SMALL_INT + "), (" + LARGE_INT + ")");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      assertTrue(resultSet.next(), "Expected first row for type: " + INT_TYPE);
      assertEquals(new BigDecimal(SMALL_INT), resultSet.getBigDecimal(1));
      assertFalse(resultSet.wasNull(), "First row should not be NULL for " + INT_TYPE);
      assertTrue(resultSet.next(), "Expected second row for type: " + INT_TYPE);
      assertEquals(new BigDecimal(LARGE_INT), resultSet.getBigDecimal(1));
      assertFalse(resultSet.wasNull(), "Second row should not be NULL for " + INT_TYPE);
      assertFalse(resultSet.next(), "Expected exactly two rows for type: " + INT_TYPE);
    }
  }

  @Test
  public void shouldSelectLargeResultSetFromTableForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with <type> column exists with 50000 sequential values
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_int_", "col " + INT_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::"
            + INT_TYPE
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      int expected = 0;
      while (resultSet.next()) {
        assertAllIntegerGettersInRange(
            resultSet, 1, expected, "Value mismatch for " + INT_TYPE + ", row " + expected);
        expected++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + INT_TYPE);
    }
  }

  private static void assertSingleRow(Connection connection, String sql, List<Long> expected)
      throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + INT_TYPE);
      for (int i = 0; i < expected.size(); i++) {
        assertAllIntegerGettersInRange(
            resultSet, i + 1, expected.get(i), "Column mismatch for " + INT_TYPE);
      }
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + INT_TYPE);
    }
  }

  private static void assertSingleColumnRows(
      Connection connection, String tableName, List<Long> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      for (int i = 0; i < expectedValues.size(); i++) {
        assertTrue(resultSet.next(), "Missing row " + i + " for " + INT_TYPE);
        Long expected = expectedValues.get(i);
        if (expected == null) {
          assertNull(resultSet.getObject(1), "Expected NULL at row " + i + " for " + INT_TYPE);
          assertTrue(resultSet.wasNull(), "Expected wasNull() after getObject NULL at row " + i);
          assertEquals(0L, resultSet.getLong(1), "Expected getLong=0 for NULL at row " + i);
          assertTrue(resultSet.wasNull(), "Expected wasNull() after getLong NULL at row " + i);
        } else {
          assertAllIntegerGettersInRange(
              resultSet, 1, expected, "Value mismatch for " + INT_TYPE + ", row " + i);
        }
      }
      assertFalse(resultSet.next(), "Unexpected extra rows for " + INT_TYPE);
    }
  }

  private static void assertAllIntegerGettersInRange(
      ResultSet resultSet, int columnIndex, long expected, String message) throws Exception {
    assertEquals(expected, resultSet.getLong(columnIndex), message + " (getLong)");
    assertFalse(resultSet.wasNull(), message + " (getLong should not be NULL)");

    if (expected >= Integer.MIN_VALUE && expected <= Integer.MAX_VALUE) {
      assertEquals((int) expected, resultSet.getInt(columnIndex), message + " (getInt)");
      assertFalse(resultSet.wasNull(), message + " (getInt should not be NULL)");
    } else {
      assertThrows(
          SQLException.class,
          () -> resultSet.getInt(columnIndex),
          message + " (getInt should fail on overflow)");
      assertFalse(resultSet.wasNull(), message + " (getInt overflow should not set NULL)");
    }

    if (expected >= Short.MIN_VALUE && expected <= Short.MAX_VALUE) {
      assertEquals((short) expected, resultSet.getShort(columnIndex), message + " (getShort)");
      assertFalse(resultSet.wasNull(), message + " (getShort should not be NULL)");
    } else {
      assertThrows(
          SQLException.class,
          () -> resultSet.getShort(columnIndex),
          message + " (getShort should fail on overflow)");
      assertFalse(resultSet.wasNull(), message + " (getShort overflow should not set NULL)");
    }
  }

  private static void assertInt38NumberGetters(
      ResultSet resultSet, int columnIndex, String expected, String message) throws Exception {
    BigDecimal expectedValue = new BigDecimal(expected);
    assertEquals(expectedValue, resultSet.getBigDecimal(columnIndex), message + " (getBigDecimal)");
    assertEquals(expectedValue, resultSet.getObject(columnIndex), message + " (getObject)");
    assertEquals(expected, resultSet.getString(columnIndex), message + " (getString)");

    assertThrows(
        SQLException.class,
        () -> resultSet.getLong(columnIndex),
        message + " (getLong should fail on overflow)");
    assertThrows(
        SQLException.class,
        () -> resultSet.getInt(columnIndex),
        message + " (getInt should fail on overflow)");
    assertThrows(
        SQLException.class,
        () -> resultSet.getShort(columnIndex),
        message + " (getShort should fail on overflow)");
  }
}
