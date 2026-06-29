package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Time;
import java.time.LocalTime;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.ResourceLock;
import org.junit.jupiter.api.parallel.Resources;

// These tests mutate the JVM default timezone; lock the shared TIME_ZONE resource so they are
// serialized against each other and any other timezone-sensitive test if parallel execution is
// ever enabled.
@ResourceLock(Resources.TIME_ZONE)
public class DataConversionContextTest {
  // CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME=false (the default).
  private static final DataConversionContext UTC = new DataConversionContext() {};
  // CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME=true.
  private static final DataConversionContext WALL_CLOCK =
      new DataConversionContext() {
        @Override
        public boolean isTreatTimeAsWallClockTime() {
          return true;
        }
      };

  @Test
  public void shouldDefaultGetDateUseNullTimezoneToTrue() {
    // Mirrors snowflake-jdbc's SFBaseSession.getDateUseNullTimezone default (true), which makes the
    // no-Calendar getDate(int) pass a null timezone (raw epoch-day date).
    assertTrue(UTC.isGetDateUseNullTimezone());
  }

  @Test
  public void shouldUseUtcEpochModuloDayForTimeToNanosByDefault() {
    // Default (CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME=false) matches snowflake-jdbc's
    // SfTimestampUtil.getTimeInNanoseconds: Time.getTime() is treated as UTC epoch millis reduced
    // modulo a day, independent of the JVM default TZ.
    TimeZone original = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Asia/Tokyo")); // +09:00
    try {
      Time t = Time.valueOf("10:30:00"); // epoch millis = 1970-01-01 10:30 JST = 01:30 UTC
      long expectedMsOfDayUtc = ((t.getTime() % 86_400_000L) + 86_400_000L) % 86_400_000L;
      assertEquals(expectedMsOfDayUtc * 1_000_000L, UTC.timeToNanosOfDay(t));
    } finally {
      TimeZone.setDefault(original);
    }
  }

  @Test
  public void shouldReadLocalWallClockFieldsForTimeToNanosWhenWallClock() {
    // CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME=true reads JVM-local wall-clock fields via
    // Time#toLocalTime().toNanoOfDay(), so the result is the wall-clock time regardless of TZ.
    TimeZone original = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Asia/Tokyo")); // +09:00
    try {
      Time t = Time.valueOf("10:30:00");
      long expectedNanos = LocalTime.of(10, 30, 0).toNanoOfDay();
      assertEquals(expectedNanos, WALL_CLOCK.timeToNanosOfDay(t));
    } finally {
      TimeZone.setDefault(original);
    }
  }

  @Test
  public void shouldTruncateSubSecondMillisForTimeToNanosWhenWallClock() {
    // Parity: snowflake-jdbc's wall-clock branch uses Time#toLocalTime() (LocalTime.of(h, m, s)),
    // which drops sub-second milliseconds. We replicate that truncation exactly.
    LocalTime wallClock = LocalTime.of(10, 30, 0);
    Time t = new Time(Time.valueOf(wallClock).getTime() + 123L);
    long expectedNanos = wallClock.toNanoOfDay(); // 123 ms dropped, matching snowflake-jdbc
    assertEquals(expectedNanos, WALL_CLOCK.timeToNanosOfDay(t));
  }
}
