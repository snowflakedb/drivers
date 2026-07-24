package net.snowflake.jdbc.e2e.types;

import static java.sql.ResultSetMetaData.columnNoNulls;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Timestamp;
import java.sql.Types;
import java.time.Instant;
import java.time.LocalDateTime;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.time.ZonedDateTime;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Calendar;
import java.util.List;
import java.util.TimeZone;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;

/**
 * End-to-end coverage for {@code TIMESTAMP_TZ}, mirroring the {@code @jdbc_e2e} scenarios in {@code
 * tests/definitions/shared/types/timestamp_tz.feature}.
 *
 * <p>TZ carries its own stored offset. The driver returns a {@link SnowflakeTimestampWithTimezone}
 * whose {@link SnowflakeTimestampWithTimezone#toZonedDateTime() toZonedDateTime()} exposes the
 * preserved offset, so assertions check both the absolute instant and the offset/local wall-clock.
 * Binding a TZ value requires the {@code setTimestamp(int, Timestamp, Calendar)} overload with the
 * session's {@code CLIENT_TIMESTAMP_TYPE_MAPPING} set to {@code TIMESTAMP_TZ} (the only way to bind
 * a stored offset).
 *
 * <p>The session timezone is explicitly set to a non-UTC zone ({@value #SESSION_TIMEZONE}) so the
 * tests prove offset preservation is independent of the session timezone.
 */
@Isolated("pins JVM default timezone for stable TZ metadata")
public class TimestampTzTests extends SnowflakeIntegrationTestBase
    implements WithScalarResultSetMetadataAssertions, WithPinnedTemporalMetadataTimeZone {
  private static final int LARGE_RESULT_SET_SIZE = 50_000;
  private static final Instant SEQUENCE_START = Instant.parse("2024-01-01T00:00:00Z");
  private static final ColumnExpectation TIMESTAMP_TZ_COLUMN =
      new ColumnExpectation(
          null,
          Types.TIMESTAMP_WITH_TIMEZONE,
          "TIMESTAMPTZ",
          Timestamp.class.getName(),
          29,
          9,
          29,
          false,
          false,
          columnNoNulls,
          SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ);

  /** A deliberately non-UTC session timezone; see the class-level note. */
  private static final String SESSION_TIMEZONE = "America/New_York";

  @BeforeAll
  protected void setSessionTimezone() throws Exception {
    applySessionTimezone(getDefaultConnection());
  }

  @Test
  public void shouldCastTimestampTzValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ",
        resultSet -> {
          assertTrue(resultSet.next());
          // Then All values should be returned as appropriate type
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00", "+05:00"));
          // And Values should have timezone info
          assertHasTimezoneInfo(resultSet, 1);

          ResultSetMetaData meta = resultSet.getMetaData();
          SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
          assertScalarResultSetMetadata(
              meta,
              sfMeta,
              Arrays.asList(
                  TIMESTAMP_TZ_COLUMN.withColumnName(
                      "'2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ")));

          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectTimestampTzValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ,"
            + " '2024-06-20 14:45:30 -08:00'::TIMESTAMP_TZ",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00", "+05:00"));
          assertTz(resultSet, 2, offset("2024-06-20T14:45:30", "-08:00"));
          // And Values should have timezone info
          assertHasTimezoneInfo(resultSet, 1);
          assertHasTimezoneInfo(resultSet, 2);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1970-01-01 00:00:00 +00:00'::TIMESTAMP_TZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("1970-01-01T00:00:00", "+00:00"));
          assertHasTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00.123456 +05:00'::TIMESTAMP_TZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00.123456", "+05:00"));
          assertHasTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldPreserveTimezoneOffsetForTimestampTz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00
    //   -08:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00
    //   +04:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ" is executed
    String sql =
        "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ,"
            + " '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ,"
            + " '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ,"
            + " '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ,"
            + " '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should preserve offsets [+05:30, -08:00, +00:00, +04:30, -02:30]
          assertTrue(resultSet.next());
          String[] offsets = {"+05:30", "-08:00", "+00:00", "+04:30", "-02:30"};
          for (int col = 1; col <= offsets.length; col++) {
            assertTz(resultSet, col, offset("2024-01-15T10:30:00", offsets[col - 1]));
          }
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectEdgeDateTimestampTzValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    withQueryResult(
        connection,
        "SELECT '9999-12-31 23:59:59 +00:00'::TIMESTAMP_TZ",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("9999-12-31T23:59:59", "+00:00"));
          // And Values should have timezone info
          assertHasTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1900-01-01 00:00:00 +00:00'::TIMESTAMP_TZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("1900-01-01T00:00:00", "+00:00"));
          assertHasTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1960-06-15 12:00:00 +05:00'::TIMESTAMP_TZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("1960-06-15T12:00:00", "+05:00"));
          assertHasTimezoneInfo(resultSet, 1);
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldHandleNullValuesForTimestampTz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is
    // executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ",
        resultSet -> {
          // Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00", "+05:00"));
          assertNull(resultSet.getTimestamp(2));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksForTimestampTz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01
    //   00:00:00 +00:00'::TIMESTAMP_TZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY
    //   ts" is executed
    String sql =
        "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY ts";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00 +00:00
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectValuesFromTableForTimestampTz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_TZ column exists with values <insert_values>
    String tableName = createTempTable(connection, "ud_ts_tz_", "col TIMESTAMP_TZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('2024-01-15 10:30:00 +05:00'), ('2024-06-20 14:45:30 -08:00'),"
            + " ('1970-01-01 00:00:00 +00:00'), ('2024-01-15 10:30:00.123456 +05:00'), (NULL)");

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("1970-01-01T00:00:00", "+00:00"));
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00", "+05:00"));
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-01-15T10:30:00.123456", "+05:00"));
          assertTrue(resultSet.next());
          assertTz(resultSet, 1, offset("2024-06-20T14:45:30", "-08:00"));
          // And Values should have timezone info
          assertHasTimezoneInfo(resultSet, 1);
          assertTrue(resultSet.next());
          assertNull(resultSet.getTimestamp(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampTz()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
    String tableName = createTempTable(connection, "ud_ts_tz_large_", "col TIMESTAMP_TZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ)"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00 +00:00
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectTimestampTzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openTzMappingConnection()) {

      // When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp
      // values
      withPreparedQueryResult(
          connection,
          "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ",
          ps -> {
            ps.setTimestamp(
                1, Timestamp.from(Instant.parse("2024-01-15T10:30:00Z")), calendarFor("+05:00"));
            ps.setTimestamp(
                2, Timestamp.from(Instant.parse("2024-06-20T14:45:30Z")), calendarFor("-08:00"));
          },
          resultSet -> {
            // Then Result should contain the bound timestamps
            assertTrue(resultSet.next());
            assertTz(
                resultSet,
                1,
                OffsetDateTime.parse("2024-01-15T10:30:00Z")
                    .withOffsetSameInstant(ZoneOffset.of("+05:00")));
            assertTz(
                resultSet,
                2,
                OffsetDateTime.parse("2024-06-20T14:45:30Z")
                    .withOffsetSameInstant(ZoneOffset.of("-08:00")));
            // And Values should have timezone info
            assertHasTimezoneInfo(resultSet, 1);
            assertHasTimezoneInfo(resultSet, 2);
            assertFalse(resultSet.next());
          });
    }
  }

  @Test
  public void shouldSelectNullTimestampTzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openTzMappingConnection()) {

      // When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
      withPreparedQueryResult(
          connection,
          "SELECT ?::TIMESTAMP_TZ",
          ps -> ps.setNull(1, Types.TIMESTAMP),
          resultSet -> {
            // Then Result should contain [NULL]
            assertTrue(resultSet.next());
            assertNull(resultSet.getTimestamp(1));
            assertTrue(resultSet.wasNull());
            assertFalse(resultSet.next());
          });
    }
  }

  @Test
  public void shouldInsertTimestampTzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openTzMappingConnection()) {

      // And Table with TIMESTAMP_TZ column exists
      String tableName = createTempTable(connection, "ud_ts_tz_bind_", "col TIMESTAMP_TZ");

      // When Timestamp values are bulk-inserted using multirow binding
      List<Instant> expected =
          Arrays.asList(
              Instant.parse("1970-01-01T00:00:00Z"),
              Instant.parse("2024-01-15T10:30:00Z"),
              Instant.parse("2024-06-20T14:45:30Z"));
      try (PreparedStatement ps =
          connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
        ps.setTimestamp(
            1, Timestamp.from(Instant.parse("2024-06-20T14:45:30Z")), calendarFor("-08:00"));
        ps.addBatch();
        ps.setTimestamp(
            1, Timestamp.from(Instant.parse("1970-01-01T00:00:00Z")), calendarFor("+00:00"));
        ps.addBatch();
        ps.setTimestamp(
            1, Timestamp.from(Instant.parse("2024-01-15T10:30:00Z")), calendarFor("+05:00"));
        ps.addBatch();
        ps.executeBatch();
      }

      // And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
      withQueryResult(
          connection,
          "SELECT * FROM " + tableName + " ORDER BY col NULLS LAST",
          resultSet -> {
            // Then SELECT should return the same values in any order
            List<Instant> actual = new ArrayList<>();
            while (resultSet.next()) {
              actual.add(resultSet.getTimestamp(1).toInstant());
              assertFalse(resultSet.wasNull());
            }
            assertEquals(expected, actual);
          });
    }
  }

  /**
   * Asserts a {@code TIMESTAMP_TZ} column holds the given offset date-time: same absolute instant,
   * same preserved offset, and same local wall-clock.
   */
  private static void assertTz(ResultSet rs, int col, OffsetDateTime expected) throws Exception {
    Timestamp ts = rs.getTimestamp(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    assertEquals(expected.toInstant(), ts.toInstant(), "TZ instant mismatch at column " + col);

    Object obj = rs.getObject(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    SnowflakeTimestampWithTimezone stz =
        assertInstanceOf(
            SnowflakeTimestampWithTimezone.class,
            obj,
            "TZ getObject should be SnowflakeTimestampWithTimezone");
    ZonedDateTime zdt = stz.toZonedDateTime();
    assertEquals(expected.getOffset(), zdt.getOffset(), "TZ offset mismatch at column " + col);
    assertEquals(
        expected.toLocalDateTime(),
        zdt.toLocalDateTime(),
        "TZ local wall-clock mismatch at column " + col);
  }

  /** Asserts a column carries timezone info (a {@link SnowflakeTimestampWithTimezone}). */
  private static void assertHasTimezoneInfo(ResultSet rs, int col) throws Exception {
    Object obj = rs.getObject(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    assertInstanceOf(
        SnowflakeTimestampWithTimezone.class, obj, "TZ value should carry timezone information");
  }

  /** Asserts the current result set holds {@link #LARGE_RESULT_SET_SIZE} 1-second-spaced rows. */
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

  private static OffsetDateTime offset(String localDateTime, String zoneOffset) {
    return LocalDateTime.parse(localDateTime).atOffset(ZoneOffset.of(zoneOffset));
  }

  private static Calendar calendarFor(String zoneOffset) {
    return Calendar.getInstance(TimeZone.getTimeZone(ZoneOffset.of(zoneOffset)));
  }

  /** Sets the session timezone to the deliberately non-UTC {@link #SESSION_TIMEZONE}. */
  private void applySessionTimezone(Connection connection) throws Exception {
    execute(connection, "ALTER SESSION SET TIMEZONE = '" + SESSION_TIMEZONE + "'");
  }

  /** Opens a connection whose {@code CLIENT_TIMESTAMP_TYPE_MAPPING} is {@code TIMESTAMP_TZ}. */
  private Connection openTzMappingConnection() throws Exception {
    Connection connection = openConnection();
    try {
      ensureDatabaseAndSchema(connection);
      applySessionTimezone(connection);
      execute(connection, "ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_TZ'");
    } catch (Exception e) {
      connection.close();
      throw e;
    }
    return connection;
  }
}
