package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;
import java.util.stream.Stream;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

public class IntTests extends SnowflakeIntegrationTestBase {
  static Stream<String> intTypeSynonyms() {
    return Stream.of("INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT");
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldCastIntegerValuesToAppropriateTypeForIntAndSynonyms(String typeName)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
    String sql =
        String.format("SELECT 0::%1$s, 1000000::%1$s, 9223372036854775807::%1$s", typeName);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + typeName);

      // Then All values should be returned as appropriate type
      // And No precision loss should occur
      assertAllIntegerGettersInRange(resultSet, 1, 0L, "Column 1 mismatch for " + typeName);
      assertAllIntegerGettersInRange(resultSet, 2, 1_000_000L, "Column 2 mismatch for " + typeName);
      assertAllIntegerGettersInRange(
          resultSet, 3, Long.MAX_VALUE, "Column 3 mismatch for " + typeName);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + typeName);
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldSelectIntegerLiteralsForIntAndSynonyms(String typeName) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT 0::<type>, 1::<type>, -1::<type>, 42::<type>" is executed
    String sql = String.format("SELECT 0::%1$s, 1::%1$s, -1::%1$s, 42::%1$s", typeName);

    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then Result should contain integers [0, 1, -1, 42]
      assertTrue(resultSet.next(), "Expected one row for type: " + typeName);
      assertAllIntegerGettersInRange(resultSet, 1, 0L, "Column 1 mismatch for " + typeName);
      assertAllIntegerGettersInRange(resultSet, 2, 1L, "Column 2 mismatch for " + typeName);
      assertAllIntegerGettersInRange(resultSet, 3, -1L, "Column 3 mismatch for " + typeName);
      assertAllIntegerGettersInRange(resultSet, 4, 42L, "Column 4 mismatch for " + typeName);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + typeName);
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldHandleIntegerBoundaryValuesForIntAndSynonyms(String typeName) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT -128::<type>, 127::<type>, 255::<type>" is executed
    // Then Result should contain integers [-128, 127, 255]
    assertSingleRow(
        connection,
        String.format("SELECT -128::%1$s, 127::%1$s, 255::%1$s", typeName),
        Arrays.asList(-128L, 127L, 255L),
        typeName);

    // When Query "SELECT -32768::<type>, 32767::<type>, 65535::<type>" is executed
    // Then Result should contain integers [-32768, 32767, 65535]
    assertSingleRow(
        connection,
        String.format("SELECT -32768::%1$s, 32767::%1$s, 65535::%1$s", typeName),
        Arrays.asList((long) Short.MIN_VALUE, (long) Short.MAX_VALUE, 65535L),
        typeName);

    // When Query "SELECT -2147483648::<type>, 2147483647::<type>, 4294967295::<type>" is executed
    // Then Result should contain integers [-2147483648, 2147483647, 4294967295]
    assertSingleRow(
        connection,
        String.format("SELECT -2147483648::%1$s, 2147483647::%1$s, 4294967295::%1$s", typeName),
        Arrays.asList((long) Integer.MIN_VALUE, (long) Integer.MAX_VALUE, 4294967295L),
        typeName);

    // When Query "SELECT -9223372036854775808::<type>, 9223372036854775807::<type>" is executed
    // Then Result should contain integers [-9223372036854775808, 9223372036854775807]
    assertSingleRow(
        connection,
        String.format("SELECT -9223372036854775808::%1$s, 9223372036854775807::%1$s", typeName),
        Arrays.asList(Long.MIN_VALUE, Long.MAX_VALUE),
        typeName);
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldHandleLargeIntegerValuesForIntAndSynonyms(String typeName) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT -99999999999999999999999999999999999999::<type>,
    // 99999999999999999999999999999999999999::<type>" is executed
    String sql =
        String.format(
            "SELECT -99999999999999999999999999999999999999::%1$s, 99999999999999999999999999999999999999::%1$s",
            typeName);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then Result should contain integers [-99999999999999999999999999999999999999,
      // 99999999999999999999999999999999999999]
      assertTrue(resultSet.next(), "Expected one row for type: " + typeName);
      assertEquals(
          new BigDecimal("-99999999999999999999999999999999999999"),
          resultSet.getBigDecimal(1),
          "Column 1 mismatch for " + typeName);
      assertEquals(
          new BigDecimal("99999999999999999999999999999999999999"),
          resultSet.getBigDecimal(2),
          "Column 2 mismatch for " + typeName);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + typeName);
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldHandleNULLValuesForIntAndSynonyms(String typeName) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
    String sql = String.format("SELECT NULL::%1$s, 42::%1$s, NULL::%1$s", typeName);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then Result should contain [NULL, 42, NULL]
      assertTrue(resultSet.next(), "Expected one row for type: " + typeName);
      assertNull(resultSet.getObject(1), "Column 1 should be NULL for " + typeName);
      assertEquals(
          new BigDecimal("42"), resultSet.getBigDecimal(2), "Column 2 mismatch for " + typeName);
      assertNull(resultSet.getObject(3), "Column 3 should be NULL for " + typeName);
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + typeName);
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldDownloadLargeResultSetWithMultipleChunksForIntAndSynonyms(String typeName)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY
    // id" is executed
    String sql =
        String.format(
            "SELECT seq8()::%1$s as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id",
            typeName);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
      int expected = 0;
      while (resultSet.next()) {
        assertAllIntegerGettersInRange(
            resultSet, 1, expected, "Value mismatch for " + typeName + ", row " + expected);
        expected++;
      }
      assertEquals(50000, expected, "Unexpected row count for " + typeName);
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldSelectIntegersFromTableForIntAndSynonyms(String typeName) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // And Table with <type> column exists with values [0, 1, -1, 100]
    String tableName = createTempTable(connection, "col " + typeName);
    execute(connection, "INSERT INTO " + tableName + " VALUES (0), (1), (-1), (100)");

    // When Query "SELECT * FROM int_table ORDER BY col" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      // Then Result should contain integers [-1, 0, 1, 100]
      List<Long> expectedValues = Arrays.asList(-1L, 0L, 1L, 100L);
      for (int i = 0; i < expectedValues.size(); i++) {
        assertTrue(resultSet.next(), "Missing row " + i + " for " + typeName);
        assertAllIntegerGettersInRange(
            resultSet, 1, expectedValues.get(i), "Value mismatch for " + typeName + ", row " + i);
      }
      assertFalse(resultSet.next(), "Expected exactly four rows for " + typeName);
    }
  }

  @Test
  public void shouldSelectCornerCaseValuesFromTableForIntAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // And Table with columns (tinyint_col TINYINT, byteint_col BYTEINT, smallint_col SMALLINT,
    // int_col INT, integer_col INTEGER, bigint_col BIGINT, int38_col INT) exists
    String tableName =
        createTempTable(
            connection,
            "tinyint_col TINYINT, byteint_col BYTEINT, smallint_col SMALLINT, int_col INT, "
                + "integer_col INTEGER, bigint_col BIGINT, int38_col INT");

    // And Row with positive values (127, 255, 32767, 2147483647, 2147483647, 9223372036854775807,
    // 99999999999999999999999999999999999999) is inserted
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (127, 255, 32767, 2147483647, 2147483647, 9223372036854775807, 99999999999999999999999999999999999999)");

    // And Row with negative values (-128, -1, -32768, -2147483648, -2147483648,
    // -9223372036854775808, -99999999999999999999999999999999999999) is inserted
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (-128, -1, -32768, -2147483648, -2147483648, -9223372036854775808, -99999999999999999999999999999999999999)");

    // And Row with zeroes and nulls (0, NULL, 0, NULL, 0, NULL, 0) is inserted
    execute(connection, "INSERT INTO " + tableName + " VALUES (0, NULL, 0, NULL, 0, NULL, 0)");

    // When Query "SELECT * FROM corner_case_table" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY tinyint_col DESC")) {
      // Then Result should contain 3 rows with expected corner case values for all int type
      // synonyms
      assertTrue(resultSet.next(), "Expected first row");
      assertAllIntegerGettersInRange(resultSet, 1, 127L, "First row, tinyint_col mismatch");
      assertAllIntegerGettersInRange(resultSet, 2, 255L, "First row, byteint_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 3, Short.MAX_VALUE, "First row, smallint_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 4, Integer.MAX_VALUE, "First row, int_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 5, Integer.MAX_VALUE, "First row, integer_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 6, Long.MAX_VALUE, "First row, bigint_col mismatch");
      assertEquals(
          new BigDecimal("99999999999999999999999999999999999999"), resultSet.getBigDecimal(7));

      assertTrue(resultSet.next(), "Expected second row");
      assertAllIntegerGettersInRange(resultSet, 1, 0L, "Second row, tinyint_col mismatch");
      assertNull(resultSet.getObject(2));
      assertAllIntegerGettersInRange(resultSet, 3, 0L, "Second row, smallint_col mismatch");
      assertNull(resultSet.getObject(4));
      assertAllIntegerGettersInRange(resultSet, 5, 0L, "Second row, integer_col mismatch");
      assertNull(resultSet.getObject(6));
      assertAllIntegerGettersInRange(resultSet, 7, 0L, "Second row, int38_col mismatch");

      assertTrue(resultSet.next(), "Expected third row");
      assertAllIntegerGettersInRange(resultSet, 1, -128L, "Third row, tinyint_col mismatch");
      assertAllIntegerGettersInRange(resultSet, 2, -1L, "Third row, byteint_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 3, Short.MIN_VALUE, "Third row, smallint_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 4, Integer.MIN_VALUE, "Third row, int_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 5, Integer.MIN_VALUE, "Third row, integer_col mismatch");
      assertAllIntegerGettersInRange(
          resultSet, 6, Long.MIN_VALUE, "Third row, bigint_col mismatch");
      assertEquals(
          new BigDecimal("-99999999999999999999999999999999999999"), resultSet.getBigDecimal(7));

      assertFalse(resultSet.next(), "Expected exactly three rows");
    }
  }

  @ParameterizedTest
  @MethodSource("intTypeSynonyms")
  public void shouldSelectLargeResultSetFromTableForIntAndSynonyms(String typeName)
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    // And Table with <type> column exists with 50000 sequential values
    String tableName = createTempTable(connection, "col " + typeName);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::"
            + typeName
            + " FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
      int expected = 0;
      while (resultSet.next()) {
        assertAllIntegerGettersInRange(
            resultSet, 1, expected, "Value mismatch for " + typeName + ", row " + expected);
        expected++;
      }
      assertEquals(50000, expected, "Unexpected row count for " + typeName);
    }
  }

  private static void assertSingleRow(
      Connection connection, String sql, List<Long> expected, String typeName) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + typeName);
      for (int i = 0; i < expected.size(); i++) {
        assertAllIntegerGettersInRange(
            resultSet, i + 1, expected.get(i), "Column mismatch for " + typeName);
      }
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + typeName);
    }
  }

  private static String createTempTable(Connection connection, String columns) throws Exception {
    String tableName = "ud_int_" + UUID.randomUUID().toString().replace("-", "");
    execute(connection, "CREATE TEMPORARY TABLE " + tableName + " (" + columns + ")");
    return tableName;
  }

  private static void execute(Connection connection, String sql) throws Exception {
    try (Statement statement = connection.createStatement()) {
      statement.execute(sql);
    }
  }

  private static void assertAllIntegerGettersInRange(
      ResultSet resultSet, int columnIndex, long expected, String message) throws Exception {
    assertEquals(
        BigDecimal.valueOf(expected),
        resultSet.getBigDecimal(columnIndex),
        message + " (getBigDecimal)");
    assertEquals(expected, resultSet.getLong(columnIndex), message + " (getLong)");
    if (expected >= Integer.MIN_VALUE && expected <= Integer.MAX_VALUE) {
      assertEquals((int) expected, resultSet.getInt(columnIndex), message + " (getInt)");
    }
    if (expected >= Short.MIN_VALUE && expected <= Short.MAX_VALUE) {
      assertEquals((short) expected, resultSet.getShort(columnIndex), message + " (getShort)");
    }
  }
}
