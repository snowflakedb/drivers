package net.snowflake.client.internal.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.sql.Time;
import java.sql.Timestamp;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;

/** Unit tests for {@link SnowflakeTimeWithTimezone#toString()}. */
public class SnowflakeTimeWithTimezoneTest {

  @Test
  public void shouldFormatUtcAnchoredWallClockWhenUseSessionTimezone() {
    // 12:34:56 since midnight, offset defaults to UTC for the (long, nanos, boolean) constructor.
    long millisOfDay = (12 * 3600L + 34 * 60L + 56L) * 1000L;
    assertEquals("12:34:56", new SnowflakeTimeWithTimezone(millisOfDay, 0, true).toString());
  }

  @Test
  public void shouldFormatInSessionZoneFromTimestampConstructor() {
    Timestamp ts = new Timestamp(Instant.parse("2024-01-15T12:34:56Z").toEpochMilli());
    // America/New_York is UTC-5 in January → 07:34:56 wall-clock.
    assertEquals(
        "07:34:56",
        new SnowflakeTimeWithTimezone(ts, TimeZone.getTimeZone("America/New_York"), true)
            .toString());
  }

  @Test
  public void shouldDefaultOffsetToUtcWhenSessionTimeZoneIsNullInTimestampConstructor() {
    Timestamp ts = new Timestamp(Instant.parse("2024-01-15T12:34:56.123456789Z").toEpochMilli());
    ts.setNanos(123_456_789);
    SnowflakeTimeWithTimezone time = new SnowflakeTimeWithTimezone(ts, null, true);
    assertEquals(123_456_789, time.getNano());
    assertEquals(ZoneOffset.UTC, time.getOffset());
    assertEquals("12:34:56", time.toString());
  }

  @Test
  public void shouldDelegateToSuperWhenNotUseSessionTimezone() {
    long millisOfDay = (12 * 3600L + 34 * 60L + 56L) * 1000L;
    // useSessionTimeZone=false → identical to plain java.sql.Time (JVM-default rendering).
    assertEquals(
        new Time(millisOfDay).toString(),
        new SnowflakeTimeWithTimezone(millisOfDay, 0, false).toString());
  }
}
