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
import java.sql.Types;
import java.time.Duration;
import java.time.Period;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.jdbc.utils.DisabledOnGCP;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;

/**
 * End-to-end coverage for Snowflake INTERVAL columns, mirroring every JDBC-tagged scenario in
 * {@code tests/definitions/shared/types/interval.feature}. INTERVAL YEAR TO MONTH surfaces as
 * {@link java.time.Period} and INTERVAL DAY TO SECOND as {@link java.time.Duration}.
 *
 * <p>Values are always read through the typed {@code getObject(col, Period.class)} / {@code
 * getObject(col, Duration.class)} accessors: this yields a {@code Period}/{@code Duration} for both
 * the int64 (SB2/SB4/SB8) and Decimal128 (SB16) physical layouts, whereas plain {@code getObject}
 * would return a {@code BigDecimal} of raw nanos for the SB16 case.
 *
 * <p>Requires {@code ENABLE_INTERVAL_TYPE} to be active on the account.
 */
public class IntervalTests extends SnowflakeIntegrationTestBase
    implements WithScalarResultSetMetadataAssertions {
  private static final int LARGE_RESULT_SET_SIZE = 50_000;
  private static final ColumnExpectation INTERVAL_YEAR_MONTH_COLUMN =
      new ColumnExpectation(
          null,
          SnowflakeType.EXTRA_TYPES_YEAR_MONTH_INTERVAL,
          "INTERVAL_YEAR_MONTH",
          null,
          0,
          0,
          25,
          false,
          false,
          columnNoNulls,
          null);
  private static final ColumnExpectation INTERVAL_DAY_TIME_COLUMN =
      new ColumnExpectation(
          null,
          SnowflakeType.EXTRA_TYPES_DAY_TIME_INTERVAL,
          "INTERVAL_DAY_TIME",
          null,
          0,
          3,
          25,
          false,
          false,
          columnNoNulls,
          null);

  // Reused corner-case durations.
  private static final Duration D_99999 =
      Duration.ofDays(99999).plusHours(23).plusMinutes(59).plusSeconds(59).plusNanos(999_999_000);
  private static final Duration D_12_3_4_5_678 =
      Duration.ofDays(12).plusHours(3).plusMinutes(4).plusSeconds(5).plusMillis(678);
  private static final Duration D_1_2_3_4_567 =
      Duration.ofDays(1).plusHours(2).plusMinutes(3).plusSeconds(4).plusMillis(567);

  // ==========================================================================
  // TYPE CASTING
  // ==========================================================================

  @Test
  public void shouldCastIntervalValuesToAppropriateTypeForYearToMonthAndDayToSecond()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH, '0
    // 0:0:1.2'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND" is
    // executed
    String sql =
        "SELECT '1-2'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH, "
            + "'0 0:0:1.2'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then all INTERVAL values should be returned as appropriate type for the driver
          assertRow(
              resultSet,
              Period.of(1, 2, 0),
              Period.of(999999999, 11, 0),
              Duration.ofSeconds(1, 200_000_000),
              D_99999);

          ResultSetMetaData meta = resultSet.getMetaData();
          SnowflakeResultSetMetaData sfMeta = meta.unwrap(SnowflakeResultSetMetaData.class);
          assertScalarResultSetMetadata(
              meta,
              sfMeta,
              Arrays.asList(
                  INTERVAL_YEAR_MONTH_COLUMN.withColumnName("'1-2'::INTERVAL YEAR TO MONTH"),
                  INTERVAL_YEAR_MONTH_COLUMN.withColumnName(
                      "'999999999-11'::INTERVAL YEAR TO MONTH"),
                  INTERVAL_DAY_TIME_COLUMN.withColumnName("'0 0:0:1.2'::INTERVAL DAY TO SECOND"),
                  INTERVAL_DAY_TIME_COLUMN.withColumnName(
                      "'99999 23:59:59.999999'::INTERVAL DAY TO SECOND")));
        });
  }

  // ==========================================================================
  // SELECT LITERALS
  // ==========================================================================

  @Test
  public void shouldSelectIntervalYearToMonthLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query selecting INTERVAL YEAR TO MONTH literals is executed
    String sql =
        "SELECT '0-0'::INTERVAL YEAR TO MONTH, '1-2'::INTERVAL YEAR TO MONTH, "
            + "'-1-3'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH, "
            + "'-999999999-11'::INTERVAL YEAR TO MONTH";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the result should contain expected INTERVAL YEAR TO MONTH literal values in order
          assertRow(
              resultSet,
              Period.of(0, 0, 0),
              Period.of(1, 2, 0),
              Period.of(-1, -3, 0),
              Period.of(999999999, 11, 0),
              Period.of(-999999999, -11, 0));
        });
  }

  @Test
  public void shouldSelectIntervalDayToSecondLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query selecting INTERVAL DAY TO SECOND literals is executed
    String sql =
        "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND, '12 3:4:5.678'::INTERVAL DAY TO SECOND, "
            + "'-1 2:3:4.567'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND, "
            + "'-99999 23:59:59.999999'::INTERVAL DAY TO SECOND";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO SECOND literal values in order
          assertRow(
              resultSet,
              Duration.ZERO,
              D_12_3_4_5_678,
              D_1_2_3_4_567.negated(),
              D_99999,
              D_99999.negated());
        });
  }

  @Test
  public void shouldSelectIntervalDayToHourMaxLiteral() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '999999999 23'::INTERVAL DAY TO HOUR" is executed
    withQueryResult(
        connection,
        "SELECT '999999999 23'::INTERVAL DAY TO HOUR",
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO HOUR max value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23));
        });
  }

  @Test
  public void shouldSelectIntervalDayToHourMinLiteral() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '-999999999 23'::INTERVAL DAY TO HOUR" is executed
    withQueryResult(
        connection,
        "SELECT '-999999999 23'::INTERVAL DAY TO HOUR",
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO HOUR min value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).negated());
        });
  }

  @Test
  public void shouldSelectIntervalDayToMinuteMaxLiteral() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
    withQueryResult(
        connection,
        "SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE",
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO MINUTE max value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).plusMinutes(59));
        });
  }

  @Test
  public void shouldSelectIntervalDayToMinuteMinLiteral() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
    withQueryResult(
        connection,
        "SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE",
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO MINUTE min value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).plusMinutes(59).negated());
        });
  }

  @Test
  public void shouldSelectNullIntervalLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, NULL::INTERVAL
    // YEAR, NULL::INTERVAL SECOND" is executed
    String sql =
        "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, "
            + "NULL::INTERVAL YEAR, NULL::INTERVAL SECOND";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then the result should contain:
          assertRow(resultSet, null, null, null, null);
        });
  }

  // ==========================================================================
  // SELECT FROM TABLE
  // ==========================================================================

  @Test
  public void shouldSelectIntervalYearToMonthValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL YEAR TO MONTH column is created
    String tableName = createTempTable(connection, "ud_interval_ym_", "C1 INTERVAL YEAR TO MONTH");

    // And The table is populated with YEAR TO MONTH values including corner cases
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('-999999999-11'), ('-1-3'), ('0-0'), ('1-2'), ('999999999-11'), (NULL)");

    // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the inserted INTERVAL YEAR TO MONTH values in order
          assertOrdered(
              resultSet,
              Arrays.asList(
                  Period.of(-999999999, -11, 0),
                  Period.of(-1, -3, 0),
                  Period.of(0, 0, 0),
                  Period.of(1, 2, 0),
                  Period.of(999999999, 11, 0),
                  null));
        });
  }

  @Test
  public void shouldSelectIntervalDayToSecondValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL DAY TO SECOND column is created
    String tableName = createTempTable(connection, "ud_interval_dt_", "C1 INTERVAL DAY TO SECOND");

    // And The table is populated with DAY TO SECOND values including corner cases
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('0 0:0:0.0'), ('12 3:4:5.678'), ('-1 2:3:4.567'), "
            + "('99999 23:59:59.999999'), ('-99999 23:59:59.999999'), (NULL)");

    // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the inserted INTERVAL DAY TO SECOND values in order
          assertOrdered(
              resultSet,
              Arrays.asList(
                  D_99999.negated(),
                  D_1_2_3_4_567.negated(),
                  Duration.ZERO,
                  D_12_3_4_5_678,
                  D_99999,
                  null));
        });
  }

  @Test
  public void shouldSelectIntervalYear2ToMonthValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL YEAR(2) TO MONTH column is created
    String tableName =
        createTempTable(connection, "ud_interval_ym2_", "C1 INTERVAL YEAR(2) TO MONTH");

    // And The table is populated with values ['0-0', '1-2', '-1-3', '99-11', '-99-11', NULL]
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('0-0'), ('1-2'), ('-1-3'), ('99-11'), ('-99-11'), (NULL)");

    // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the inserted INTERVAL YEAR(2) TO MONTH values in order
          assertOrdered(
              resultSet,
              Arrays.asList(
                  Period.of(-99, -11, 0),
                  Period.of(-1, -3, 0),
                  Period.of(0, 0, 0),
                  Period.of(1, 2, 0),
                  Period.of(99, 11, 0),
                  null));
        });
  }

  @Test
  public void shouldSelectIntervalYear7ToMonthValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL YEAR(7) TO MONTH column is created
    String tableName =
        createTempTable(connection, "ud_interval_ym7_", "C1 INTERVAL YEAR(7) TO MONTH");

    // And The table is populated with values ['0-0', '1-2', '-1-3', '9999999-11', '-9999999-11',
    // NULL]
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('0-0'), ('1-2'), ('-1-3'), ('9999999-11'), ('-9999999-11'), (NULL)");

    // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the inserted INTERVAL YEAR(7) TO MONTH values in order
          assertOrdered(
              resultSet,
              Arrays.asList(
                  Period.of(-9999999, -11, 0),
                  Period.of(-1, -3, 0),
                  Period.of(0, 0, 0),
                  Period.of(1, 2, 0),
                  Period.of(9999999, 11, 0),
                  null));
        });
  }

  @Test
  public void shouldSelectIntervalDay3ToSecondValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL DAY(3) TO SECOND column is created
    String tableName =
        createTempTable(connection, "ud_interval_dt3_", "C1 INTERVAL DAY(3) TO SECOND");

    // And The table is populated with values ['0 0:0:0.0', '1 2:3:4.567', '-1 2:3:4.567', '999
    // 23:59:59.999999', '-999 23:59:59.999999', NULL]
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES ('0 0:0:0.0'), ('1 2:3:4.567'), ('-1 2:3:4.567'), "
            + "('999 23:59:59.999999'), ('-999 23:59:59.999999'), (NULL)");

    // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the inserted INTERVAL DAY(3) TO SECOND values in order
          Duration big =
              Duration.ofDays(999)
                  .plusHours(23)
                  .plusMinutes(59)
                  .plusSeconds(59)
                  .plusNanos(999_999_000);
          assertOrdered(
              resultSet,
              Arrays.asList(
                  big.negated(), D_1_2_3_4_567.negated(), Duration.ZERO, D_1_2_3_4_567, big, null));
        });
  }

  // ==========================================================================
  // MULTIPLE CHUNKS DOWNLOADING
  // ==========================================================================

  @Test
  public void shouldDownloadIntervalYearToMonthDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym FROM TABLE(GENERATOR(ROWCOUNT
    // => 50000)) v ORDER BY ym" is executed
    String sql =
        "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym "
            + "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then there are 50000 rows returned
          int rowCount = 0;
          // And all returned INTERVAL YEAR TO MONTH values should form a sequential series of
          // months starting at 0
          while (resultSet.next()) {
            assertEquals(
                Period.of(rowCount / 12, rowCount % 12, 0),
                resultSet.getObject(1, Period.class),
                "YEAR TO MONTH row " + rowCount);
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Unexpected row count for YEAR TO MONTH");
        });
  }

  @Test
  public void shouldDownloadIntervalDayToSecondDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt FROM
    // TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt" is executed
    String sql =
        "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt "
            + "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt";
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          // Then there are 50000 rows returned
          int rowCount = 0;
          // And all returned INTERVAL DAY TO SECOND values should form a sequential series of
          // seconds starting at 0
          while (resultSet.next()) {
            assertEquals(
                Duration.ofSeconds(rowCount),
                resultSet.getObject(1, Duration.class),
                "DAY TO SECOND row " + rowCount);
            rowCount++;
          }
          assertEquals(LARGE_RESULT_SET_SIZE, rowCount, "Unexpected row count for DAY TO SECOND");
        });
  }

  // ==========================================================================
  // BINDING
  // ==========================================================================

  // TODO(SNOW-3953892): the GCP test account's GS rejects the bound INSERT-into-interval path
  // ("Year-Month Interval '0' is invalid"); AWS and Azure accept it. Re-enable on GCP once that
  // account catches up. The SELECT-binding tests below are unaffected, so only the two
  // insert-and-select-back cases are gated.
  @Test
  @DisabledOnGCP
  public void shouldInsertAndSelectBackIntervalYearToMonthValuesUsingParameterBinding()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL YEAR TO MONTH column is created
    String tableName = createTempTable(connection, "ud_interval_ym_", "C1 INTERVAL YEAR TO MONTH");

    // When INTERVAL YEAR TO MONTH values ['0-0', '1-2', '-1-3', '999999999-11', '-999999999-11',
    // NULL] are inserted using parameter binding
    insertBoundStrings(
        connection,
        "INSERT INTO " + tableName + " VALUES (?::INTERVAL YEAR TO MONTH)",
        "0-0",
        "1-2",
        "-1-3",
        "999999999-11",
        "-999999999-11",
        null);

    // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the bound INTERVAL YEAR TO MONTH values
          // ['-999999999-11', '-1-3', '0-0', '1-2', '999999999-11', NULL]
          assertOrdered(
              resultSet,
              Arrays.asList(
                  Period.of(-999999999, -11, 0),
                  Period.of(-1, -3, 0),
                  Period.of(0, 0, 0),
                  Period.of(1, 2, 0),
                  Period.of(999999999, 11, 0),
                  null));
        });
  }

  // TODO(SNOW-3953892): disabled on GCP for the same reason as the YEAR TO MONTH case above.
  @Test
  @DisabledOnGCP
  public void shouldInsertAndSelectBackIntervalDayToSecondValuesUsingParameterBinding()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with INTERVAL DAY TO SECOND column is created
    String tableName = createTempTable(connection, "ud_interval_dt_", "C1 INTERVAL DAY TO SECOND");

    // When INTERVAL DAY TO SECOND values ['0 0:0:0.0', '12 3:4:5.678', '-1 2:3:4.567', '99999
    // 23:59:59.999999', '-99999 23:59:59.999999', NULL] are inserted using parameter binding
    insertBoundStrings(
        connection,
        "INSERT INTO " + tableName + " VALUES (?::INTERVAL DAY TO SECOND)",
        "0 0:0:0.0",
        "12 3:4:5.678",
        "-1 2:3:4.567",
        "99999 23:59:59.999999",
        "-99999 23:59:59.999999",
        null);

    // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY C1 NULLS LAST",
        resultSet -> {
          // Then the result should contain the bound INTERVAL DAY TO SECOND values ['-99999
          // 23:59:59.999999', '-1 2:3:4.567', '0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999',
          // NULL]
          assertOrdered(
              resultSet,
              Arrays.asList(
                  D_99999.negated(),
                  D_1_2_3_4_567.negated(),
                  Duration.ZERO,
                  D_12_3_4_5_678,
                  D_99999,
                  null));
        });
  }

  @Test
  public void shouldSelectIntervalYearToMonthValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO
    // MONTH" is executed with bound string values ['0-0', '1-2', '999999999-11']
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH",
        bindStrings("0-0", "1-2", "999999999-11"),
        resultSet -> {
          // Then the result should contain:
          assertRow(resultSet, Period.of(0, 0, 0), Period.of(1, 2, 0), Period.of(999999999, 11, 0));
        });
  }

  @Test
  public void shouldSelectIntervalDayToSecondValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO
    // SECOND" is executed with bound string values ['0 0:0:0.0', '12 3:4:5.678', '99999
    // 23:59:59.999999']
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND",
        bindStrings("0 0:0:0.0", "12 3:4:5.678", "99999 23:59:59.999999"),
        resultSet -> {
          // Then the result should contain:
          assertRow(resultSet, Duration.ZERO, D_12_3_4_5_678, D_99999);
        });
  }

  @Test
  public void shouldSelectNullIntervalValuesUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND" is executed with
    // bound NULL values
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND",
        bindStrings(null, null),
        resultSet -> {
          // Then the result should contain:
          assertRow(resultSet, null, null);
        });
  }

  @Test
  public void shouldSelectIntervalDayToHourMaxValueUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '999999999
    // 23'
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL DAY TO HOUR",
        bindStrings("999999999 23"),
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO HOUR max bound value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23));
        });
  }

  @Test
  public void shouldSelectIntervalDayToHourMinValueUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '-999999999
    // 23'
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL DAY TO HOUR",
        bindStrings("-999999999 23"),
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO HOUR min bound value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).negated());
        });
  }

  @Test
  public void shouldSelectIntervalDayToMinuteMaxValueUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '999999999
    // 23:59'
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL DAY TO MINUTE",
        bindStrings("999999999 23:59"),
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO MINUTE max bound value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).plusMinutes(59));
        });
  }

  @Test
  public void shouldSelectIntervalDayToMinuteMinValueUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '-999999999
    // 23:59'
    withPreparedQueryResult(
        connection,
        "SELECT ?::INTERVAL DAY TO MINUTE",
        bindStrings("-999999999 23:59"),
        resultSet -> {
          // Then the result should contain expected INTERVAL DAY TO MINUTE min bound value
          assertRow(resultSet, Duration.ofDays(999999999).plusHours(23).plusMinutes(59).negated());
        });
  }

  // ---- Shared helpers ------------------------------------------------------

  /** Assert a single result row column-by-column via typed accessors, then that no row follows. */
  private static void assertRow(ResultSet resultSet, Object... expected) throws Exception {
    assertTrue(resultSet.next(), "Expected one INTERVAL row");
    for (int i = 0; i < expected.length; i++) {
      assertTypedColumn(resultSet, i + 1, expected[i], "column " + (i + 1));
    }
    assertFalse(resultSet.next(), "Expected exactly one INTERVAL row");
  }

  /** Assert an ordered single-column result set, then that no extra row follows. */
  private static void assertOrdered(ResultSet resultSet, List<Object> expected) throws Exception {
    for (int i = 0; i < expected.size(); i++) {
      assertTrue(resultSet.next(), "Missing INTERVAL row " + i);
      assertTypedColumn(resultSet, 1, expected.get(i), "row " + i);
    }
    assertFalse(resultSet.next(), "Unexpected extra INTERVAL rows");
  }

  /**
   * Assert one column against an expected value, choosing the accessor by expected type: {@code
   * null} reads as SQL NULL; {@link Period} / {@link Duration} via the typed {@code getObject}
   * (works for SB2/4/8 and SB16).
   */
  private static void assertTypedColumn(ResultSet rs, int col, Object expected, String msg)
      throws Exception {
    if (expected == null) {
      assertNull(rs.getObject(col), msg + " (expected SQL NULL)");
      assertTrue(rs.wasNull(), msg + " (expected wasNull)");
      return;
    }
    if (expected instanceof Period) {
      assertEquals(expected, rs.getObject(col, Period.class), msg);
    } else if (expected instanceof Duration) {
      assertEquals(expected, rs.getObject(col, Duration.class), msg);
    } else {
      throw new AssertionError("unsupported expected type: " + expected.getClass());
    }
    assertFalse(rs.wasNull(), msg + " (unexpected wasNull)");
  }

  /** A prepared-statement setup that binds each value as a VARCHAR string (or SQL NULL). */
  private static WithQueryUtils.PreparedStatementSetup bindStrings(String... values) {
    return ps -> {
      for (int i = 0; i < values.length; i++) {
        if (values[i] == null) {
          ps.setNull(i + 1, Types.VARCHAR);
        } else {
          ps.setString(i + 1, values[i]);
        }
      }
    };
  }

  /**
   * Insert each value as its own row using a single-parameter {@code INSERT} that casts the bound
   * VARCHAR string to the target interval type; {@code null} binds as SQL NULL.
   */
  private void insertBoundStrings(Connection connection, String insertSql, String... values)
      throws Exception {
    try (PreparedStatement ps = connection.prepareStatement(insertSql)) {
      for (String value : values) {
        if (value == null) {
          ps.setNull(1, Types.VARCHAR);
        } else {
          ps.setString(1, value);
        }
        ps.executeUpdate();
      }
    }
  }
}
