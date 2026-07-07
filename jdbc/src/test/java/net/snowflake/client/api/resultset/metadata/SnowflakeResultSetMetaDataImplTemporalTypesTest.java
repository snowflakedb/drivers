package net.snowflake.client.api.resultset.metadata;

import java.sql.Connection;
import java.sql.Date;
import java.sql.PreparedStatement;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.util.TimeZone;
import java.util.stream.Stream;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.parallel.Isolated;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/** Tests ResultSetMetaData precision and display size for temporal types under various formats. */
@Isolated("sets JVM default timezone - must not run concurrently with other tests")
@SkipNewDriver("not yet implemented - calculating display size & precision")
class SnowflakeResultSetMetaDataImplTemporalTypesTest extends SnowflakeIntegrationTestBase
    implements WithMetaDataAssertions {

  private TimeZone originalTimeZone;

  @BeforeAll
  void setUpDefaultTimezone() {
    // Display size is computed by formatting a sample timestamp with the JVM's default timezone.
    // Java's SimpleDateFormat "XX" pattern (used for TZHTZM) renders UTC offsets as "Z" (1 char)
    // instead of "+0000" (5 chars).
    // Pinning a non-UTC timezone ensures consistent display size calculations.
    originalTimeZone = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Europe/Warsaw"));
  }

  @AfterAll
  void tearDownDefaultTimezone() {
    TimeZone.setDefault(originalTimeZone);
  }

  static Stream<Arguments> defaultFormatCases() {

    return Stream.of(
        Arguments.of("'2026-01-01'::DATE", Types.DATE, "DATE", Date.class, 10, 0, 10, false, false),
        Arguments.of(
            "'10:11:12'::TIME",
            Types.TIME,
            "TIME",
            Time.class,
            8, // account default format → 8 chars
            9,
            8,
            false,
            false),
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_NTZ",
            Types.TIMESTAMP,
            "TIMESTAMPNTZ",
            Timestamp.class,
            23, // account default format → 23 chars
            9,
            23,
            false,
            false),
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_LTZ",
            Types.TIMESTAMP,
            "TIMESTAMPLTZ",
            Timestamp.class,
            29, // account default format → 29 chars
            9,
            29,
            false,
            false),
        Arguments.of(
            "'2026-01-01 10:11:12 +00:00'::TIMESTAMP_TZ",
            Types.TIMESTAMP_WITH_TIMEZONE,
            "TIMESTAMPTZ",
            Timestamp.class,
            29, // account default format → 29 chars
            9,
            29,
            false,
            false));
  }

  @ParameterizedTest(name = "should describe metadata for {0} with default format")
  @MethodSource("defaultFormatCases")
  void shouldDescribeMetadataWithDefaultFormat(
      String sqlExpression,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive)
      throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT " + sqlExpression + " AS col")) {
      assertColumnMetadata(
          stmt.getMetaData(),
          sqlExpression,
          expectedType,
          expectedTypeName,
          expectedClass,
          expectedPrecision,
          expectedScale,
          expectedDisplaySize,
          expectedSigned,
          expectedCaseSensitive);
    }
  }

  static Stream<Arguments> formatCases() {
    return Stream.of(
        // DATE — format is fixed (DATE_OUTPUT_FORMAT doesn't affect display size in the driver),
        // but included for completeness with an explicit format
        Arguments.of(
            "'2026-01-01'::DATE",
            "DATE_OUTPUT_FORMAT",
            "YYYY-MM-DD",
            Types.DATE,
            "DATE",
            Date.class,
            10,
            0,
            10,
            false,
            false),
        // TIME — compact format
        Arguments.of(
            "'10:11:12'::TIME",
            "TIME_OUTPUT_FORMAT",
            "HH24:MI:SS",
            Types.TIME,
            "TIME",
            Time.class,
            8, // "HH:MI:SS" → 8 chars
            9,
            8,
            false,
            false),
        // TIME — format with fractional seconds
        Arguments.of(
            "'10:11:12'::TIME",
            "TIME_OUTPUT_FORMAT",
            "HH24:MI:SS.FF6",
            Types.TIME,
            "TIME",
            Time.class,
            15, // "HH:MI:SS.ffffff" → 15 chars
            9,
            15,
            false,
            false),
        // TIMESTAMP_NTZ — format with milliseconds
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_NTZ",
            "TIMESTAMP_NTZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS.FF3",
            Types.TIMESTAMP,
            "TIMESTAMPNTZ",
            Timestamp.class,
            23, // "YYYY-MM-DD HH:MI:SS.fff" → 23 chars
            9,
            23,
            false,
            false),
        // TIMESTAMP_NTZ — format with nanoseconds
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_NTZ",
            "TIMESTAMP_NTZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS.FF9",
            Types.TIMESTAMP,
            "TIMESTAMPNTZ",
            Timestamp.class,
            29, // "YYYY-MM-DD HH:MI:SS.fffffffff" → 29 chars
            9,
            29,
            false,
            false),
        // TIMESTAMP_LTZ — format with offset and milliseconds
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_LTZ",
            "TIMESTAMP_LTZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
            Types.TIMESTAMP,
            "TIMESTAMPLTZ",
            Timestamp.class,
            29, // "YYYY-MM-DD HH:MI:SS.fff +HHMM" → 29 chars
            9,
            29,
            false,
            false),
        // TIMESTAMP_LTZ — compact format without fractional seconds
        Arguments.of(
            "'2026-01-01 10:11:12'::TIMESTAMP_LTZ",
            "TIMESTAMP_LTZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS TZHTZM",
            Types.TIMESTAMP,
            "TIMESTAMPLTZ",
            Timestamp.class,
            25, // "YYYY-MM-DD HH:MI:SS +HHMM" → 25 chars
            9,
            25,
            false,
            false),
        // TIMESTAMP_TZ — format with offset and milliseconds
        Arguments.of(
            "'2026-01-01 10:11:12 +00:00'::TIMESTAMP_TZ",
            "TIMESTAMP_TZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
            Types.TIMESTAMP_WITH_TIMEZONE,
            "TIMESTAMPTZ",
            Timestamp.class,
            29, // "YYYY-MM-DD HH:MI:SS.fff +HHMM" → 29 chars
            9,
            29,
            false,
            false),
        // TIMESTAMP_TZ — compact format without fractional seconds
        Arguments.of(
            "'2026-01-01 10:11:12 +00:00'::TIMESTAMP_TZ",
            "TIMESTAMP_TZ_OUTPUT_FORMAT",
            "YYYY-MM-DD HH24:MI:SS TZHTZM",
            Types.TIMESTAMP_WITH_TIMEZONE,
            "TIMESTAMPTZ",
            Timestamp.class,
            25, // "YYYY-MM-DD HH:MI:SS +HHMM" → 25 chars
            9,
            25,
            false,
            false));
  }

  @ParameterizedTest(name = "should describe metadata for {0} with format {2}")
  @MethodSource("formatCases")
  void shouldDescribeMetadataWithExplicitFormat(
      String sqlExpression,
      String formatParam,
      String formatValue,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive)
      throws Exception {
    try (Connection conn = openConnection()) {
      try (Statement alter = conn.createStatement()) {
        alter.execute("ALTER SESSION SET " + formatParam + " = '" + formatValue + "'");
      }
      try (PreparedStatement stmt = conn.prepareStatement("SELECT " + sqlExpression + " AS col")) {
        assertColumnMetadata(
            stmt.getMetaData(),
            sqlExpression,
            expectedType,
            expectedTypeName,
            expectedClass,
            expectedPrecision,
            expectedScale,
            expectedDisplaySize,
            expectedSigned,
            expectedCaseSensitive);
      }
    }
  }
}
