package net.snowflake.jdbc.e2e.types;

import static java.sql.ResultSetMetaData.columnNoNulls;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Timestamp;
import java.sql.Types;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;

/**
 * End-to-end coverage for {@code TIMESTAMP_LTZ}, mirroring the {@code @jdbc_e2e} scenarios in
 * {@code tests/definitions/shared/types/timestamp_ltz.feature}.
 *
 * <p>LTZ is an absolute instant. {@link Timestamp#toInstant()} is therefore timezone-independent,
 * and every literal in the feature carries an explicit {@code +00:00} offset, so assertions compare
 * against the corresponding UTC {@link Instant}. Binding a {@link Timestamp} to a {@code
 * TIMESTAMP_LTZ} round-trips the instant unchanged, so bind tests need no special session timezone.
 */
@Isolated("pins JVM default timezone for stable LTZ metadata")
public class TimestampLtzTests extends SnowflakeIntegrationTestBase
    implements WithScalarResultSetMetadataAssertions, WithPinnedTemporalMetadataTimeZone {
  private static final int LARGE_RESULT_SET_SIZE = 50_000;
  private static final Instant SEQUENCE_START = Instant.parse("2024-01-01T00:00:00Z");
  private static final ColumnExpectation TIMESTAMP_LTZ_COLUMN =
      new ColumnExpectation(
          null,
          Types.TIMESTAMP,
          "TIMESTAMPLTZ",
          Timestamp.class.getName(),
          29,
          9,
          29,
          false,
          false,
          columnNoNulls,
          SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ);

  @Test
  public void shouldCastTimestampLtzValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          // Then All values should be returned as appropriate type
          assertLtz(resultSet, 1, Instant.parse("2024-01-15T10:30:00Z"));
          // And Values should have timezone info
          ResultSetMetaData meta = resultSet.getMetaData();
          SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
          assertScalarResultSetMetadata(
              meta,
              sfMeta,
              Arrays.asList(
                  TIMESTAMP_LTZ_COLUMN.withColumnName(
                      "'2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ")));

          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectTimestampLtzValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT <query_values>" is executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ,"
            + " '2024-06-20 14:45:30 +00:00'::TIMESTAMP_LTZ",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("2024-01-15T10:30:00Z"));
          assertLtz(resultSet, 2, Instant.parse("2024-06-20T14:45:30Z"));
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1970-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("1970-01-01T00:00:00Z"));
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00.123456 +00:00'::TIMESTAMP_LTZ",
        resultSet -> {
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("2024-01-15T10:30:00.123456Z"));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectSubSecondTimestampLtzValuesBeforeEpoch() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Sub-second timestamp_ltz values before the epoch are selected
    withQueryResult(
        connection,
        "SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_LTZ",
        resultSet -> {
          // Then Result should contain the expected sub-second values before the epoch
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("1969-12-31T23:59:59.999999999Z"));
          assertFalse(resultSet.next());
        });
    withQueryResult(
        connection,
        "SELECT '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_LTZ(3)",
        resultSet -> {
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.ofEpochSecond(-2, 500_000_000));
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldHandleNullValuesForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is
    //   executed
    withQueryResult(
        connection,
        "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ",
        resultSet -> {
          // Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("2024-01-15T10:30:00Z"));
          assertNull(resultSet.getTimestamp(2));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01
    //   00:00:00 +00:00'::TIMESTAMP_LTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY
    //   ts" is executed
    String sql =
        "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) ORDER BY ts";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00 UTC
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectValuesFromTableForTimestampLtz() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_LTZ column exists with values <insert_values>
    String tableName = createTempTable(connection, "ud_ts_ltz_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('2024-01-15 10:30:00 +00:00'), ('2024-06-20 14:45:30 +00:00'),"
            + " ('1970-01-01 00:00:00 +00:00'), (NULL)");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
        resultSet -> {
          // Then Result should contain timestamps <expected_values>
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("1970-01-01T00:00:00Z"));
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("2024-01-15T10:30:00Z"));
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, Instant.parse("2024-06-20T14:45:30Z"));
          assertTrue(resultSet.next());
          assertNull(resultSet.getTimestamp(1));
          assertTrue(resultSet.wasNull());
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTableForTimestampLtz()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
    String tableName = createTempTable(connection, "ud_ts_ltz_large_", "col TIMESTAMP_LTZ");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,"
            + " '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ)"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
        resultSet -> {
          // Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01
          //   00:00:00 UTC
          assertSequentialTimestamps(resultSet);
        });
  }

  @Test
  public void shouldSelectTimestampLtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp
    // values
    Instant first = Instant.parse("2024-01-15T10:30:00Z");
    Instant second = Instant.parse("2024-06-20T14:45:30Z");
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ",
        ps -> {
          ps.setTimestamp(1, Timestamp.from(first));
          ps.setTimestamp(2, Timestamp.from(second));
        },
        resultSet -> {
          // Then Result should contain the bound timestamps
          assertTrue(resultSet.next());
          assertLtz(resultSet, 1, first);
          assertLtz(resultSet, 2, second);
          assertFalse(resultSet.next());
        });
  }

  @Test
  public void shouldSelectNullTimestampLtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT ?::TIMESTAMP_LTZ",
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
  public void shouldInsertTimestampLtzUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with TIMESTAMP_LTZ column exists
    String tableName = createTempTable(connection, "ud_ts_ltz_bind_", "col TIMESTAMP_LTZ");

    // When Timestamp values are bulk-inserted using multirow binding
    List<Instant> expected =
        Arrays.asList(
            Instant.parse("1970-01-01T00:00:00Z"),
            Instant.parse("2024-01-15T10:30:00Z"),
            Instant.parse("2024-06-20T22:45:30Z"));
    try (PreparedStatement ps =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setTimestamp(1, Timestamp.from(Instant.parse("2024-06-20T22:45:30Z")));
      ps.addBatch();
      ps.setTimestamp(1, Timestamp.from(Instant.parse("1970-01-01T00:00:00Z")));
      ps.addBatch();
      ps.setTimestamp(1, Timestamp.from(Instant.parse("2024-01-15T10:30:00Z")));
      ps.addBatch();
      ps.executeBatch();
    }

    // And Query "SELECT * FROM <table> ORDER BY col" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY col",
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

  /** Asserts a {@code TIMESTAMP_LTZ} column holds the given absolute instant. */
  private static void assertLtz(ResultSet rs, int col, Instant expected) throws Exception {
    Timestamp ts = rs.getTimestamp(col);
    assertFalse(rs.wasNull(), "column " + col + " should not be NULL");
    assertEquals(expected, ts.toInstant(), "LTZ instant mismatch at column " + col);
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
}
