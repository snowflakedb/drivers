package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Timestamp;
import java.sql.Types;
import java.time.Instant;
import java.time.LocalDateTime;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * End-to-end coverage for {@code TIMESTAMP_NTZ}, mirroring the {@code @jdbc_e2e} scenarios in
 * {@code tests/definitions/shared/types/timestamp_ntz.feature}.
 *
 * <p>NTZ is a wall-clock value with no zone. The driver returns a plain {@link Timestamp} whose
 * instant is the stored wall-clock anchored at UTC, independent of the JVM/session timezone, so
 * assertions compare {@link Timestamp#toInstant()} against the wall-clock interpreted at {@link
 * ZoneOffset#UTC}. Bind/insert tests pin the session timezone to UTC because binding a {@link
 * Timestamp} sends an instant that Snowflake re-anchors to the session timezone when casting to
 * {@code TIMESTAMP_NTZ}.
 */
public class TimestampNtzTests extends SnowflakeIntegrationTestBase {
  private static final int LARGE_RESULT_SET_SIZE = 50_000;
  private static final Instant SEQUENCE_START = Instant.parse("2024-01-01T00:00:00Z");

  @Test
  public void shouldCastTimestampNtzValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          // Then All values should be returned as appropriate type
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
          // And Values should not have timezone info
          assertNoTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectTimestampNtzValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, '2024-06-20 14:45:30'::TIMESTAMP_NTZ",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
          assertNtz(resultSet, 2, LocalDateTime.of(2024, 6, 20, 14, 45, 30));
          // And Values should not have timezone info
          assertNoTimezoneInfo(resultSet, 1);
          assertNoTimezoneInfo(resultSet, 2);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1970-01-01 00:00:00'::TIMESTAMP_NTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(1970, 1, 1, 0, 0, 0));
          assertNoTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00.123456'::TIMESTAMP_NTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0, 123_456_000));
          assertNoTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldHandleNullValuesForTimestampNtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ",
        resultSet -> {
          // Then Result should contain [2024-01-15 10:30:00, NULL]
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
          assertNull(resultSet.getTimestamp(2));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksForTimestampNtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01
    //   00:00:00'::TIMESTAMP_NTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is
    //   executed
    String sql =
        "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY ts";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectValuesFromTableForTimestampNtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_NTZ column exists with values <insert_values>
    String tableName = createTempTable(connection, "ud_ts_ntz_", "col TIMESTAMP_NTZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('2024-01-15 10:30:00'), ('2024-06-20 14:45:30'),"
            + " ('1970-01-01 00:00:00'), ('2024-01-15 10:30:00.123456'), (NULL)");

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(1970, 1, 1, 0, 0, 0));
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0, 123_456_000));
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, LocalDateTime.of(2024, 6, 20, 14, 45, 30));
          // And Values should not have timezone info
          assertNoTimezoneInfo(resultSet, 1);
          assertTrue(resultSet.next());
          assertNull(resultSet.getTimestamp(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampNtz()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
    String tableName = createTempTable(connection, "ud_ts_ntz_large_", "col TIMESTAMP_NTZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00'::TIMESTAMP_NTZ)"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectTimestampNtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openUtcConnection()) {

      // When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp
      //   values
      withPreparedQueryResult(
          connection,
          "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ",
          ps -> {
            ps.setTimestamp(1, Timestamp.from(Instant.parse("2024-01-15T10:30:00Z")));
            ps.setTimestamp(2, Timestamp.from(Instant.parse("2024-06-20T14:45:30Z")));
          },
          resultSet -> {
            // Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
            assertTrue(resultSet.next());
            assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
            assertNtz(resultSet, 2, LocalDateTime.of(2024, 6, 20, 14, 45, 30));
            // And Values should not have timezone info
            assertNoTimezoneInfo(resultSet, 1);
            assertNoTimezoneInfo(resultSet, 2);
            assertFalse(resultSet.next());
          });
    }
  }

  @Test
  public void shouldReturnNullWhenSelectingTimestampNtzUsingParameterBindingWithNullValue()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIMESTAMP_NTZ",
        ps -> ps.setNull(1, Types.TIMESTAMP),
        resultSet -> {
          // Then Result should contain [NULL]
          assertTrue(resultSet.next());
          assertNull(resultSet.getTimestamp(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldStoreUtcEquivalentWhenBindingTimezoneAwareDatetimeToTimestampNtz()
      throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openUtcConnection()) {

      // When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound aware datetime <input>
      withPreparedQueryResult(
          connection,
          "SELECT ?::TIMESTAMP_NTZ",
          ps ->
              ps.setTimestamp(
                  1, Timestamp.from(OffsetDateTime.parse("2024-01-15T12:30:00+02:00").toInstant())),
          resultSet -> {
            // Then Result should contain [<expected>]
            assertTrue(resultSet.next());
            assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
            // And Values should not have timezone info
            assertNoTimezoneInfo(resultSet, 1);
            assertFalse(resultSet.next());
          });

      // Remaining examples: +00:00 keeps the wall-clock, -05:00 shifts it forward to UTC.
      assertAwareBindStoresUtc(
          connection, "2024-01-15T10:30:00+00:00", LocalDateTime.of(2024, 1, 15, 10, 30, 0));
      assertAwareBindStoresUtc(
          connection, "2024-01-15T10:30:00-05:00", LocalDateTime.of(2024, 1, 15, 15, 30, 0));
    }
  }

  @Test
  public void shouldInsertTimestampNtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openUtcConnection()) {

      // And Table with TIMESTAMP_NTZ column exists
      String tableName = createTempTable(connection, "ud_ts_ntz_bind_", "col TIMESTAMP_NTZ");

      // When Timestamp values are bulk-inserted using multirow binding
      try (PreparedStatement ps =
          connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
        ps.setTimestamp(1, Timestamp.from(Instant.parse("2024-06-20T14:45:30Z")));
        ps.addBatch();
        ps.setTimestamp(1, Timestamp.from(Instant.parse("2024-01-15T10:30:00Z")));
        ps.addBatch();
        ps.setTimestamp(1, Timestamp.from(Instant.parse("1970-01-01T00:00:00Z")));
        ps.addBatch();
        ps.executeBatch();
      }

      // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
      withQueryResult(
          connection,
          "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
          resultSet -> {
            // Then SELECT should return the inserted values in ascending order
            assertTrue(resultSet.next());
            assertNtz(resultSet, 1, LocalDateTime.of(1970, 1, 1, 0, 0, 0));
            assertTrue(resultSet.next());
            assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
            assertTrue(resultSet.next());
            assertNtz(resultSet, 1, LocalDateTime.of(2024, 6, 20, 14, 45, 30));
            assertFalse(resultSet.next());
          });
    }
  }

  @Test
  public void shouldReturnNaiveDatetimeForTypeNameAliasWhenSessionMappingIsTimestampNtz()
      throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openConnection()) {
      ensureDatabaseAndSchema(connection);
      // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
      execute(connection, "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");

      for (String alias : new String[] {"TIMESTAMP", "DATETIME"}) {
        // When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
        withQueryResult(
            connection,
            "SELECT '2024-01-15 10:30:00'::" + alias,
            resultSet -> {
              assertTrue(resultSet.next());
              // Then All values should be returned as appropriate type
              assertTrue(
                  resultSet.getMetaData().getColumnTypeName(1).toUpperCase().contains("NTZ"),
                  alias + " alias should map to TIMESTAMP_NTZ");
              assertNtz(resultSet, 1, LocalDateTime.of(2024, 1, 15, 10, 30, 0));
              // And Values should not have timezone info
              assertNoTimezoneInfo(resultSet, 1);
              assertFalse(resultSet.next());
            });
      }
    }
  }

  @Test
  public void shouldReturnAwareDatetimeForTimestampAliasWhenSessionMappingIsTimestampLtz()
      throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openConnection()) {
      ensureDatabaseAndSchema(connection);
      // And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
      execute(connection, "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_LTZ'");

      // When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
      withQueryResult(
          connection,
          "SELECT '2024-01-15 10:30:00'::TIMESTAMP",
          resultSet -> {
            assertTrue(resultSet.next());
            // Then All values should be returned as appropriate type
            assertInstanceOf(Timestamp.class, resultSet.getObject(1));
            assertFalse(resultSet.wasNull());
            // And Values should have timezone info
            assertTrue(
                resultSet.getMetaData().getColumnTypeName(1).toUpperCase().contains("LTZ"),
                "TIMESTAMP alias should map to TIMESTAMP_LTZ");
            assertFalse(resultSet.next());
          });
    }
  }

  /**
   * Binds {@code awareInput} (an offset date-time) as a timestamp and asserts the value read back
   * from a {@code ::TIMESTAMP_NTZ} cast equals the expected UTC wall-clock (session timezone is
   * UTC).
   */
  private void assertAwareBindStoresUtc(
      Connection connection, String awareInput, LocalDateTime expectedUtcWallClock)
      throws Exception {
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIMESTAMP_NTZ",
        ps -> ps.setTimestamp(1, Timestamp.from(OffsetDateTime.parse(awareInput).toInstant())),
        resultSet -> {
          assertTrue(resultSet.next());
          assertNtz(resultSet, 1, expectedUtcWallClock);
          assertNoTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
  }

  /**
   * Asserts the current result set holds {@link #LARGE_RESULT_SET_SIZE} 1-second-spaced NTZ rows.
   */
  private static void assertSequentialTimestamps(ResultSet resultSet) throws Exception {
    int rowCount = 0;
    while (resultSet.next()) {
      Instant expected = SEQUENCE_START.plusSeconds(rowCount);
      assertEquals(
          expected, resultSet.getTimestamp(1).toInstant(), "timestamp mismatch at row " + rowCount);
      assertFalse(resultSet.wasNull());
      rowCount++;
    }
    assertEquals(LARGE_RESULT_SET_SIZE, rowCount);
  }

  /**
   * Asserts a {@code TIMESTAMP_NTZ} column holds the given wall-clock interpreted at UTC (the
   * instant is timezone-independent because NTZ stores a bare wall-clock).
   */
  private static void assertNtz(ResultSet rs, int col, LocalDateTime expectedWallClock)
      throws Exception {
    Timestamp ts = rs.getTimestamp(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    assertEquals(
        expectedWallClock.toInstant(ZoneOffset.UTC),
        ts.toInstant(),
        "NTZ instant mismatch at column " + col);
  }

  /** Asserts a column is a plain {@link Timestamp} carrying no timezone information. */
  private static void assertNoTimezoneInfo(ResultSet rs, int col) throws Exception {
    Object obj = rs.getObject(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    assertInstanceOf(Timestamp.class, obj, "NTZ getObject should be java.sql.Timestamp");
    assertFalse(
        obj instanceof SnowflakeTimestampWithTimezone, "NTZ must not carry timezone information");
  }

  /**
   * Opens a fresh connection pinned to the UTC session timezone for deterministic bind round-trips.
   */
  private Connection openUtcConnection() throws Exception {
    Connection connection = openConnection();
    ensureDatabaseAndSchema(connection);
    execute(connection, "ALTER SESSION SET TIMEZONE = 'UTC'");
    return connection;
  }
}
