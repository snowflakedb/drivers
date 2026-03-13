package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Timestamp;
import java.sql.Types;
import java.time.Instant;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

public class TimestampLtzTests extends SnowflakeIntegrationTestBase {

  private static final int LARGE_RESULT_SET_SIZE = 50_000;

  private static final String TS_2024_JAN_STR = "2024-01-15 10:30:00 +00:00";
  private static final String TS_2024_JUN_STR = "2024-06-20 14:45:30 +00:00";
  private static final String TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00";
  private static final String TS_MICRO_STR = "2024-01-15 10:30:00.123456 +00:00";
  private static final String TS_SEQ_BASE_STR = "2024-01-01 00:00:00 +00:00";

  private static final long TS_2024_JAN_MS = Instant.parse("2024-01-15T10:30:00Z").toEpochMilli();
  private static final long TS_2024_JUN_MS = Instant.parse("2024-06-20T14:45:30Z").toEpochMilli();
  private static final long TS_EPOCH_MS = 0L;
  private static final long TS_MICRO_MS =
      Instant.parse("2024-01-15T10:30:00.123456Z").toEpochMilli();
  private static final int TS_MICRO_NANOS = 123_456_000; // microseconds -> nanoseconds
  private static final long TS_SEQ_BASE_MS = Instant.parse("2024-01-01T00:00:00Z").toEpochMilli();

  // ==========================================================================
  // 1. Type casting
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldCastTimestampLtzValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
    // Then All values should be returned as appropriate type
    // And Values should have timezone info
    Connection connection = getDefaultConnection();
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT '" + TS_2024_JAN_STR + "'::TIMESTAMP_LTZ")) {
      assertTrue(rs.next(), "Expected one row");
      Object obj = rs.getObject(1);
      // positive: getObject returns Timestamp, wasNull is false
      assertInstanceOf(Timestamp.class, obj, "getObject should return Timestamp for TIMESTAMP_LTZ");
      assertFalse(rs.wasNull(), "Non-null column should not set wasNull");
      // negative: result must not be raw Long or null (which would indicate missing conversion)
      assertNotNull(obj, "Non-null column must not return null from getObject");
      assertFalse(obj instanceof Long, "TIMESTAMP_LTZ must not come back as a raw Long");
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }

  // ==========================================================================
  // 2. Literal SELECT (basic, epoch, microseconds)
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldSelectTimestampLtzValues() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT <query_values>" is executed
    // Then Result should contain timestamps <expected_values>
    Connection connection = getDefaultConnection();

    // basic: two distinct timestamps — positive: values match, negative: Jan != Jun and is earlier
    String basicSql =
        "SELECT '"
            + TS_2024_JAN_STR
            + "'::TIMESTAMP_LTZ, '"
            + TS_2024_JUN_STR
            + "'::TIMESTAMP_LTZ";
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery(basicSql)) {
      assertTrue(rs.next(), "Expected one row for basic");
      assertTimestampColumn(rs, 1, TS_2024_JAN_MS, "basic Jan");
      assertTimestampColumn(rs, 2, TS_2024_JUN_MS, "basic Jun");
      assertNotEquals(
          rs.getTimestamp(1).getTime(),
          rs.getTimestamp(2).getTime(),
          "Jan and Jun timestamps must differ");
      assertTrue(
          rs.getTimestamp(1).getTime() < rs.getTimestamp(2).getTime(),
          "Jan must be earlier than Jun");
      assertFalse(rs.next(), "Expected exactly one row for basic");
    }

    // epoch: zero-epoch timestamp — positive: millis are exactly 0
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT '" + TS_EPOCH_STR + "'::TIMESTAMP_LTZ")) {
      assertTrue(rs.next(), "Expected one row for epoch");
      assertTimestampColumn(rs, 1, TS_EPOCH_MS, "epoch");
      assertFalse(rs.next(), "Expected exactly one row for epoch");
    }

    // microseconds: nanos preserved — positive: millis and nanos match
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT '" + TS_MICRO_STR + "'::TIMESTAMP_LTZ")) {
      assertTrue(rs.next(), "Expected one row for microseconds");
      Timestamp micro = rs.getTimestamp(1);
      assertNotNull(micro, "Microsecond timestamp must not be null");
      assertEquals(TS_MICRO_MS, micro.getTime(), "Microsecond millis mismatch");
      assertEquals(TS_MICRO_NANOS, micro.getNanos(), "Microsecond nanos mismatch");
      // negative: nanos must not be 0 (that would lose microsecond precision)
      assertNotEquals(0, micro.getNanos(), "Microsecond precision must not be truncated to 0 nanos");
      assertFalse(rs.next(), "Expected exactly one row for microseconds");
    }
  }

  // ==========================================================================
  // 3. NULL handling
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldHandleNullValuesForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
    // Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
    Connection connection = getDefaultConnection();
    try (Statement stmt = connection.createStatement();
        ResultSet rs =
            stmt.executeQuery(
                "SELECT '" + TS_2024_JAN_STR + "'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ")) {
      assertTrue(rs.next(), "Expected one row");
      // positive: non-null column returns correct Timestamp and wasNull is false
      assertTimestampColumn(rs, 1, TS_2024_JAN_MS, "non-null column");
      // negative: non-null column must not be misidentified as null
      assertFalse(rs.wasNull(), "Non-null column must not set wasNull()");
      // positive: NULL column returns null and sets wasNull
      assertNull(rs.getObject(2), "NULL column must return null from getObject");
      assertTrue(rs.wasNull(), "NULL column must set wasNull() after getObject");
      assertNull(rs.getTimestamp(2), "NULL column must return null from getTimestamp");
      assertTrue(rs.wasNull(), "NULL column must set wasNull() after getTimestamp");
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }

  // ==========================================================================
  // 4. Large result set — GENERATOR (no table)
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldDownloadLargeResultSetWithMultipleChunksForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
    //   '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts
    //   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
    // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '"
            + TS_SEQ_BASE_STR
            + "'::TIMESTAMP_LTZ) as ts "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY 1";
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery(sql)) {
      int rowCount = 0;
      while (rs.next()) {
        Timestamp ts = rs.getTimestamp(1);
        // positive: each timestamp is not null and matches expected sequential value
        assertNotNull(ts, "Timestamp at row " + rowCount + " must not be null");
        assertEquals(
            TS_SEQ_BASE_MS + (long) rowCount * 1000,
            ts.getTime(),
            "Sequential timestamp mismatch at row " + rowCount);
        rowCount++;
      }
      // positive: all 50000 rows were returned; negative: not fewer (chunk download error)
      assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected exactly 50000 rows");
    }
  }

  // ==========================================================================
  // 5. Table SELECT (basic, epoch, null)
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldSelectValuesFromTableForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    // And Table with TIMESTAMP_LTZ column exists with values <insert_values>
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain timestamps <expected_values>
    Connection connection = getDefaultConnection();

    // basic: positive values match, negative: two distinct values must differ and be ordered
    String basicTable = createTempTable(connection, "ud_tsltz_basic_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO "
            + basicTable
            + " VALUES ('"
            + TS_2024_JAN_STR
            + "'), ('"
            + TS_2024_JUN_STR
            + "')");
    assertTimestampRows(
        connection,
        basicTable,
        new long[] {TS_2024_JAN_MS, TS_2024_JUN_MS},
        new boolean[] {false, false});
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT * FROM " + basicTable + " ORDER BY col")) {
      assertTrue(rs.next());
      long first = rs.getTimestamp(1).getTime();
      assertTrue(rs.next());
      long second = rs.getTimestamp(1).getTime();
      assertNotEquals(first, second, "Two different timestamps must not come back equal");
      assertTrue(first < second, "Earlier timestamp must sort before later");
    }

    // epoch: positive: zero-epoch value round-trips correctly
    String epochTable = createTempTable(connection, "ud_tsltz_epoch_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO "
            + epochTable
            + " VALUES ('"
            + TS_EPOCH_STR
            + "'), ('"
            + TS_2024_JAN_STR
            + "')");
    assertTimestampRows(
        connection,
        epochTable,
        new long[] {TS_EPOCH_MS, TS_2024_JAN_MS},
        new boolean[] {false, false});

    // null: positive: NULL round-trips as null with wasNull; negative: non-null not misidentified
    String nullTable = createTempTable(connection, "ud_tsltz_null_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO " + nullTable + " VALUES (NULL), ('" + TS_2024_JAN_STR + "')");
    try (Statement stmt = connection.createStatement();
        ResultSet rs =
            stmt.executeQuery("SELECT * FROM " + nullTable + " ORDER BY col NULLS FIRST")) {
      assertTrue(rs.next(), "Expected first row (NULL)");
      assertNull(rs.getTimestamp(1), "NULL row must return null");
      assertTrue(rs.wasNull(), "NULL row must set wasNull");
      assertTrue(rs.next(), "Expected second row (non-null)");
      assertTimestampColumn(rs, 1, TS_2024_JAN_MS, "non-null row after NULL");
      // negative: non-null row must not set wasNull
      assertFalse(rs.wasNull(), "Non-null row after NULL must not set wasNull");
      assertFalse(rs.next(), "Expected exactly two rows");
    }
  }

  // ==========================================================================
  // 6. Large result set — TABLE
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: getTimestamp")
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampLtz()
      throws Exception {
    // Given Snowflake client is logged in
    // And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_tsltz_large_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '"
            + TS_SEQ_BASE_STR
            + "'::TIMESTAMP_LTZ)"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      int rowCount = 0;
      while (rs.next()) {
        Timestamp ts = rs.getTimestamp(1);
        // positive: each timestamp is not null and sequential
        assertNotNull(ts, "Row " + rowCount + " must not be null");
        assertEquals(
            TS_SEQ_BASE_MS + (long) rowCount * 1000,
            ts.getTime(),
            "Sequential timestamp mismatch at row " + rowCount);
        rowCount++;
      }
      // positive: all rows returned; negative: not fewer (chunk download error detection)
      assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Expected exactly 50000 rows from table");
    }
  }

  // ==========================================================================
  // 7. Parameter binding — SELECT
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: setTimestamp / getTimestamp")
  public void shouldSelectTimestampLtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
    // Then Result should contain the bound timestamps
    // When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
    // Then Result should contain [NULL]
    Connection connection = getDefaultConnection();

    // positive: bound non-null timestamps return Timestamp objects; negative: wasNull is false
    try (PreparedStatement ps =
        connection.prepareStatement("SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ")) {
      ps.setTimestamp(1, new Timestamp(TS_2024_JAN_MS));
      ps.setTimestamp(2, new Timestamp(TS_2024_JUN_MS));
      try (ResultSet rs = ps.executeQuery()) {
        assertTrue(rs.next(), "Expected one row");
        Object obj1 = rs.getObject(1);
        assertInstanceOf(Timestamp.class, obj1, "Col 1 must be Timestamp");
        assertNotNull(obj1, "Non-null binding must not return null");
        assertFalse(rs.wasNull(), "Col 1 non-null binding must not set wasNull");
        assertInstanceOf(Timestamp.class, rs.getObject(2), "Col 2 must be Timestamp");
        assertFalse(rs.wasNull(), "Col 2 non-null binding must not set wasNull");
        assertFalse(rs.next(), "Expected exactly one row");
      }
    }

    // positive: bound NULL returns null and sets wasNull
    try (PreparedStatement ps = connection.prepareStatement("SELECT ?::TIMESTAMP_LTZ")) {
      ps.setNull(1, Types.TIMESTAMP);
      try (ResultSet rs = ps.executeQuery()) {
        assertTrue(rs.next(), "Expected one row");
        assertNull(rs.getObject(1), "NULL binding must return null");
        assertTrue(rs.wasNull(), "NULL binding must set wasNull");
      }
    }
  }

  // ==========================================================================
  // 8. Parameter binding — INSERT
  // ==========================================================================

  @Test
  @Disabled("Pending implementation in Universal Driver: setTimestamp / getTimestamp")
  public void shouldInsertTimestampLtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // And Table with TIMESTAMP_LTZ column exists
    // When Timestamp values are bulk-inserted using multirow binding
    // And Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then SELECT should return the same values in any order
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_tsltz_bind_", "col TIMESTAMP_LTZ");

    try (PreparedStatement ps =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setTimestamp(1, new Timestamp(TS_2024_JAN_MS));
      ps.execute();
      ps.setTimestamp(1, new Timestamp(TS_2024_JUN_MS));
      ps.execute();
      ps.setNull(1, Types.TIMESTAMP);
      ps.execute();
    }

    int nonNullCount = 0;
    int nullCount = 0;
    try (Statement stmt = connection.createStatement();
        ResultSet rs =
            stmt.executeQuery("SELECT * FROM " + tableName + " ORDER BY col NULLS FIRST")) {
      while (rs.next()) {
        rs.getTimestamp(1);
        if (rs.wasNull()) {
          nullCount++;
        } else {
          nonNullCount++;
        }
      }
    }
    // positive: 2 non-null + 1 null returned; negative: not more (no phantom duplicate rows)
    assertEquals(2, nonNullCount, "Expected 2 non-null rows after binding insert");
    assertEquals(1, nullCount, "Expected 1 null row after binding insert");
    assertEquals(3, nonNullCount + nullCount, "Expected exactly 3 total rows after 3 inserts");
  }

  // ==========================================================================
  // Helpers
  // ==========================================================================

  private static void assertTimestampColumn(
      ResultSet rs, int col, long expectedMs, String label) throws Exception {
    Object obj = rs.getObject(col);
    assertInstanceOf(Timestamp.class, obj, label + ": getObject should return Timestamp");
    assertFalse(rs.wasNull(), label + ": non-null column should not set wasNull");
    assertEquals(
        expectedMs, rs.getTimestamp(col).getTime(), label + ": getTimestamp millis mismatch");
    assertFalse(rs.wasNull(), label + ": getTimestamp should not set wasNull");
  }

  private static void assertTimestampRows(
      Connection connection, String tableName, long[] expectedMs, boolean[] expectedNull)
      throws Exception {
    try (Statement stmt = connection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      for (int i = 0; i < expectedMs.length; i++) {
        assertTrue(rs.next(), "Expected row " + i);
        if (expectedNull[i]) {
          assertNull(rs.getTimestamp(1), "Row " + i + " should be null");
          assertTrue(rs.wasNull(), "Row " + i + " should set wasNull");
        } else {
          assertTimestampColumn(rs, 1, expectedMs[i], "Row " + i);
        }
      }
      assertFalse(rs.next(), "Unexpected extra rows in " + tableName);
    }
  }
}
