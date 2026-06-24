package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class TimeTests extends SnowflakeIntegrationTestBase {
  private static final int LARGE_RESULT_SET_SIZE = 100_000;

  @Test
  public void shouldCastTimeValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
    String sql = "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then All values should be returned as appropriate type
          assertTrue(resultSet.next());
          for (int i = 1; i <= 3; i++) {
            Object obj = resultSet.getObject(i);
            assertInstanceOf(Time.class, obj, "Column " + i + " getObject should return Time");
            assertFalse(resultSet.wasNull());
          }
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectTimeValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    String basicQuery = "SELECT '10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME";
    String midnightQuery = "SELECT '00:00:00'::TIME";
    String microsecondsQuery = "SELECT '10:30:00.123456'::TIME";

    // Then Result should contain times <expected_values>
    withQueryResult(
        connection,
        basicQuery,
        resultSet -> {
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));
          assertTimeGetters(resultSet, 2, LocalTime.of(14, 45, 30));
          assertTimeGetters(resultSet, 3, LocalTime.of(23, 59, 59));
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        midnightQuery,
        resultSet -> {
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(0, 0, 0));
          assertFalse(resultSet.next());
        });
    // Java's java.sql.Time has millisecond resolution, so sub-ms is truncated.
    withQueryResult(
        connection,
        microsecondsQuery,
        resultSet -> {
          assertTrue(resultSet.next());
          long expectedMs = LocalTime.of(10, 30, 0).toNanoOfDay() / 1_000_000L + 123L;
          assertEquals(expectedMs, resultSet.getTime(1).getTime());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldHandleTimePrecisionScale() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
    String queryScale0 = "SELECT '10:30:00.123456789'::TIME(0)";
    String queryScale3 = "SELECT '10:30:00.123456789'::TIME(3)";
    String queryScale6 = "SELECT '10:30:00.123456789'::TIME(6)";

    // Then Result should contain [<expected>]
    withQueryResult(
        connection,
        queryScale0,
        resultSet -> {
          assertTrue(resultSet.next());
          assertEquals(
              new Time(LocalTime.of(10, 30, 0).toNanoOfDay() / 1_000_000L), resultSet.getTime(1));
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        queryScale3,
        resultSet -> {
          assertTrue(resultSet.next());
          long expectedMs = LocalTime.of(10, 30, 0).toNanoOfDay() / 1_000_000L + 123L;
          assertEquals(new Time(expectedMs), resultSet.getTime(1));
          assertFalse(resultSet.next());
        });
    // Java Time truncates to ms (123 ms) regardless of scale > 3.
    withQueryResult(
        connection,
        queryScale6,
        resultSet -> {
          assertTrue(resultSet.next());
          long expectedMs = LocalTime.of(10, 30, 0).toNanoOfDay() / 1_000_000L + 123L;
          assertEquals(new Time(expectedMs), resultSet.getTime(1));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldHandleNULLValuesForTime() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
    String sql = "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [10:30:00, NULL, 23:59:59]
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));

          assertNull(resultSet.getTime(2));
          assertTrue(resultSet.wasNull());
          assertNull(resultSet.getObject(2));
          assertTrue(resultSet.wasNull());

          assertTimeGetters(resultSet, 3, LocalTime.of(23, 59, 59));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksForTime() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1,
    // '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
    String sql =
        "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY t";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain 100000 sequentially increasing time values from 00:00:00
          int rowCount = 0;
          while (resultSet.next()) {
            long expectedMs = rowCount;
            assertEquals(
                new Time(expectedMs), resultSet.getTime(1), "Time mismatch at row " + rowCount);
            assertFalse(resultSet.wasNull());
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount);
        });
  }

  @Test
  public void shouldSelectValuesFromTableForTime() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIME column exists with values <insert_values>
    String tableName = createTempTable(connection, "ud_time_select_", "col TIME");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES ('10:30:00'), ('14:45:30'), ('23:59:59')");

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
        resultSet -> {
          // Then Result should contain times <expected_values>
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(14, 45, 30));
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(23, 59, 59));
          assertFalse(resultSet.next());
        });

    // null example
    String nullTable = createTempTable(connection, "ud_time_select_null_", "col TIME");
    execute(connection, "INSERT INTO " + nullTable + " VALUES (NULL), ('10:30:00')");
    withQueryResult(
        connection,
        "SELECT * FROM " + nullTable + " ORDER BY col NULLS LAST",
        resultSet -> {
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));
          assertTrue(resultSet.next());
          assertNull(resultSet.getTime(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTableForTime() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
    String tableName = createTempTable(connection, "ud_time_large_", "col TIME");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1,"
            + " '00:00:00'::TIME) FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
        resultSet -> {

          // Then Result should contain 100000 sequentially increasing time values from 00:00:00
          int rowCount = 0;
          while (resultSet.next()) {
            long expectedMs = rowCount;
            assertEquals(
                new Time(expectedMs), resultSet.getTime(1), "Time mismatch at row " + rowCount);
            assertFalse(resultSet.wasNull());
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount);
        });
  }

  @Test
  public void shouldSelectTimeUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed with bound time values [10:30:00,
    // 14:45:30, 23:59:59]
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIME, ?::TIME, ?::TIME",
        ps -> {
          ps.setTime(1, Time.valueOf("10:30:00"));
          ps.setTime(2, Time.valueOf("14:45:30"));
          ps.setTime(3, Time.valueOf("23:59:59"));
        },
        resultSet -> {
          // Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));
          assertTimeGetters(resultSet, 2, LocalTime.of(14, 45, 30));
          assertTimeGetters(resultSet, 3, LocalTime.of(23, 59, 59));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectNullTimeUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::TIME" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIME",
        ps -> ps.setNull(1, Types.TIME),
        resultSet -> {
          // Then Result should contain [NULL]
          assertTrue(resultSet.next());
          assertNull(resultSet.getTime(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldInsertTimeUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIME column exists
    String tableName = createTempTable(connection, "ud_time_bind_", "col TIME");

    // When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
    try (PreparedStatement ps =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setTime(1, Time.valueOf("00:00:00"));
      ps.execute();
      ps.setTime(1, Time.valueOf("10:30:00"));
      ps.execute();
      ps.setTime(1, Time.valueOf("14:45:30"));
      ps.execute();
      ps.setTime(1, Time.valueOf("23:59:59"));
      ps.execute();
    }

    // And Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
        resultSet -> {

          // Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(0, 0, 0));
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(10, 30, 0));
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(14, 45, 30));
          assertTrue(resultSet.next());
          assertTimeGetters(resultSet, 1, LocalTime.of(23, 59, 59));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldInsertTimeWithFractionalSecondsUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIME column exists
    String tableName = createTempTable(connection, "ud_time_batch_", "col TIME");

    // The Gherkin scenario expresses microsecond literals (10:30:00.123456) for the Python driver.
    // Java's java.sql.Time has millisecond resolution; sub-ms is truncated when constructing Time.
    Time t1 = new Time(LocalTime.of(10, 30, 0).toNanoOfDay() / 1_000_000L + 123L);
    Time t2 = new Time(LocalTime.of(14, 45, 30).toNanoOfDay() / 1_000_000L + 654L);

    // When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
    try (PreparedStatement ps =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setTime(1, t1);
      ps.addBatch();
      ps.setTime(1, t2);
      ps.addBatch();

      int[] counts = ps.executeBatch();
      assertArrayEquals(new int[] {1, 1}, counts);
    }

    // And Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
        resultSet -> {
          // Then Result should contain times [10:30:00.123456, 14:45:30.654321]
          assertTrue(resultSet.next());
          assertEquals(t1, resultSet.getTime(1));
          assertTrue(resultSet.next());
          assertEquals(t2, resultSet.getTime(1));
          assertFalse(resultSet.next());
        });
  }

  private static void assertTimeGetters(ResultSet rs, int col, LocalTime expected)
      throws Exception {
    Time expectedTime = new Time(expected.toNanoOfDay() / 1_000_000L);

    assertEquals(expectedTime, rs.getTime(col), "getTime mismatch");
    assertFalse(rs.wasNull());

    // LocalTime.toString() omits trailing zero components ("10:30:00" → "10:30"); the driver
    // formats via TIME_OUTPUT_FORMAT (default "HH:mm:ss"), so format the expected the same way.
    assertEquals(
        expected.format(DateTimeFormatter.ofPattern("HH:mm:ss")),
        rs.getString(col),
        "getString mismatch");
    assertFalse(rs.wasNull());

    Object obj = rs.getObject(col);
    assertEquals(expectedTime, obj, "getObject mismatch");
    assertFalse(rs.wasNull());

    assertEquals(expectedTime, rs.getObject(col, Time.class), "getObject(Time.class) mismatch");
    assertFalse(rs.wasNull());

    // getTimestamp on a TIME column: default path is new Timestamp(getTime().getTime()).
    assertEquals(
        new Timestamp(expectedTime.getTime()), rs.getTimestamp(col), "getTimestamp mismatch");
    assertFalse(rs.wasNull());
  }
}
