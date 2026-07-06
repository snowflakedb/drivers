package net.snowflake.client.internal.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.sql.Timestamp;
import java.time.Instant;
import java.time.ZonedDateTime;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link SnowflakeTimestampWithTimezone#toString()}. The rendering is anchored in
 * the carried timezone and is therefore independent of the JVM default timezone (the
 * DST-independence guarantee from snowflake-jdbc).
 */
public class SnowflakeTimestampWithTimezoneTest {
  private static final TimeZone UTC = TimeZone.getTimeZone("UTC");

  private static SnowflakeTimestampWithTimezone of(String instant, int nanos, TimeZone tz) {
    long millis = Instant.parse(instant).toEpochMilli();
    return new SnowflakeTimestampWithTimezone(millis, nanos, tz);
  }

  @Test
  public void shouldRenderFullNanosInCarriedUtcZone() {
    assertEquals(
        "2024-01-15 12:34:56.123456789", of("2024-01-15T12:34:56Z", 123_456_789, UTC).toString());
  }

  @Test
  public void shouldTrimTrailingZerosFromFraction() {
    // 123_000_000ns has 6 trailing zeros → 3 printed fractional digits.
    assertEquals(
        "2024-01-15 12:34:56.123", of("2024-01-15T12:34:56Z", 123_000_000, UTC).toString());
    // 12_345_600ns (0.0123456s) has 2 trailing zeros → 7 printed digits, matching snowflake-jdbc.
    assertEquals(
        "2018-03-11 01:10:34.0123456", of("2018-03-11T01:10:34Z", 12_345_600, UTC).toString());
  }

  @Test
  public void shouldRenderInCarriedZoneRatherThanUtc() {
    // 2024-01-15 12:34:56Z in America/New_York (EST, UTC-5 in January) → 07:34:56.
    assertEquals(
        "2024-01-15 07:34:56.123456789",
        of("2024-01-15T12:34:56Z", 123_456_789, TimeZone.getTimeZone("America/New_York"))
            .toString());
  }

  @Test
  public void shouldRenderPreEpochInstantViaCeilingSeconds() {
    // getTime()=-1000 exercises the negative-millis ceil path in getSecondsFromMillis.
    assertEquals("1969-12-31 23:59:59.000000001", of("1969-12-31T23:59:59Z", 1, UTC).toString());
  }

  @Test
  public void shouldExposeCarriedTimezone() {
    String tz = "Australia/Sydney";
    SnowflakeTimestampWithTimezone ts =
        new SnowflakeTimestampWithTimezone(new Timestamp(1647472208000L), TimeZone.getTimeZone(tz));
    assertEquals(tz, ts.getTimezone().getID());
  }

  @Test
  public void shouldConvertToZonedDateTime() {
    SnowflakeTimestampWithTimezone ts =
        new SnowflakeTimestampWithTimezone(
            new Timestamp(1647472208000L), TimeZone.getTimeZone("Australia/Sydney"));
    ZonedDateTime zdt = ts.toZonedDateTime();
    assertEquals("2022-03-17T10:10:08+11:00[Australia/Sydney]", zdt.toString());
  }
}
