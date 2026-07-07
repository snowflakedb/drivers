package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import java.math.RoundingMode;
import java.sql.Timestamp;
import java.time.Duration;
import java.util.TimeZone;
import lombok.experimental.UtilityClass;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;

@UtilityClass
public class ArrowResultUtil {
  private static final int[] POWERS_OF_10 = {
    1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000
  };

  public static final int MAX_SCALE_POWERS_OF_10 = 9;

  private static final BigDecimal NANO_IN_SECOND = BigDecimal.valueOf(1_000_000_000);

  public static long powerOfTen(int pow) {
    long val = 1;
    while (pow > MAX_SCALE_POWERS_OF_10) {
      val *= POWERS_OF_10[MAX_SCALE_POWERS_OF_10];
      pow -= MAX_SCALE_POWERS_OF_10;
    }
    return val * POWERS_OF_10[pow];
  }

  public static String getStringFormat(int scale) {
    return "%." + scale + 'f';
  }

  public static Duration getDurationFromNanos(BigDecimal numNanos) {
    int sign = numNanos.signum();
    numNanos = numNanos.abs();
    // Duration.ofSeconds overflows on negative second values, so convert the magnitude and
    // re-apply the sign via negated().
    Duration duration =
        Duration.ofSeconds(
            numNanos.divide(NANO_IN_SECOND, RoundingMode.FLOOR).longValueExact(),
            numNanos.remainder(NANO_IN_SECOND).longValueExact());
    return sign >= 0 ? duration : duration.negated();
  }

  /**
   * Generate a Java {@link Timestamp} from a scaled epoch value, using the JVM default timezone and
   * a plain (non-session-zone) result. Mirrors snowflake-jdbc's {@code
   * ArrowResultUtil.toJavaTimestamp(long, int)}.
   *
   * @param epoch the value since epoch, scaled by {@code 10^scale}
   * @param scale the scale of the value
   * @return the timestamp
   */
  public static Timestamp toJavaTimestamp(long epoch, int scale) {
    return toJavaTimestamp(epoch, scale, TimeZone.getDefault(), false);
  }

  /**
   * Generate a Java {@link Timestamp} from a compact (scaled {@code Int64}) epoch value. Decomposes
   * into whole seconds plus a nanosecond fraction, normalizing the negative-epoch case so the
   * fraction stays in {@code [0, 10^9)} (e.g. {@code -1232.234} → seconds {@code -1233}, fraction
   * {@code 766_000_000}). Mirrors snowflake-jdbc's {@code ArrowResultUtil.toJavaTimestamp(long,
   * int, TimeZone, boolean)}.
   *
   * @param epoch the value since epoch, scaled by {@code 10^scale}
   * @param scale the scale of the value
   * @param sessionTimezone the session timezone carried by the result when {@code
   *     useSessionTimezone} is set
   * @param useSessionTimezone whether to return a {@link SnowflakeTimestampWithTimezone}
   * @return the timestamp
   */
  public static Timestamp toJavaTimestamp(
      long epoch, int scale, TimeZone sessionTimezone, boolean useSessionTimezone) {
    long seconds = epoch / powerOfTen(scale);
    int fraction = (int) ((epoch % powerOfTen(scale)) * powerOfTen(9 - scale));
    if (fraction < 0) {
      // handle negative case here
      seconds--;
      fraction += 1000000000;
    }
    return createTimestamp(seconds, fraction, sessionTimezone, useSessionTimezone);
  }

  /**
   * Create a Java {@link Timestamp} from whole seconds since epoch and a nanosecond fraction. For
   * example {@code 1232.234} is {@code seconds=1232, fraction=234_000_000}; {@code -1232.234} is
   * {@code seconds=-1233, fraction=766_000_000}; {@code -0.13} is {@code seconds=-1,
   * fraction=870_000_000}. When {@code useSessionTz} is set, returns a {@link
   * SnowflakeTimestampWithTimezone} carrying {@code timezone} for rendering; otherwise a plain
   * {@link Timestamp}. Mirrors snowflake-jdbc's {@code ArrowResultUtil.createTimestamp}.
   */
  public static Timestamp createTimestamp(
      long seconds, int fraction, TimeZone timezone, boolean useSessionTz) {
    if (useSessionTz) {
      return new SnowflakeTimestampWithTimezone(seconds * powerOfTen(3), fraction, timezone);
    }
    Timestamp ts = new Timestamp(seconds * powerOfTen(3));
    ts.setNanos(fraction);
    return ts;
  }

  /**
   * Move {@code ts} from {@code oldTZ} to {@code newTZ} by the plain offset difference, preserving
   * nanos. No-op when the zones share rules.
   */
  public static Timestamp moveToTimeZone(Timestamp ts, TimeZone oldTZ, TimeZone newTZ) {
    long offset = ArrowDateUtil.moveToTimeZoneOffset(ts.getTime(), oldTZ, newTZ);
    if (offset == 0) {
      return ts;
    }
    int nanos = ts.getNanos();
    ts = new Timestamp(ts.getTime() + offset);
    ts.setNanos(nanos);
    return ts;
  }

  /**
   * Whether the given seconds-since-epoch value falls outside the range a Java {@link Timestamp}
   * can represent in milliseconds.
   */
  public static boolean isTimestampOverflow(long seconds) {
    return seconds < Long.MIN_VALUE / powerOfTen(3) || seconds > Long.MAX_VALUE / powerOfTen(3);
  }

  /**
   * Decode a {@code TIMESTAMP_TZ} timezone index into the fixed-offset {@link TimeZone} it stands
   * for. The index is biased by {@code 1440} (minutes in a day), so {@code offsetMinutes = index -
   * 1440}; e.g. {@code 1440} → {@code GMT+00:00}, {@code 1140} → {@code GMT-05:00}, {@code 1770} →
   * {@code GMT+05:30}. The resulting {@code GMT±HH:MM} name is what the TZ formatter's {@code
   * TZH:TZM}/{@code TZHTZM} tokens render, so the construction must match the server bias exactly.
   * Ported verbatim from snowflake-common's {@code SFTimestamp.convertTimezoneIndexToTimeZone}.
   */
  public static TimeZone convertTimezoneIndexToTimeZone(int timezoneIndex) {
    timezoneIndex -= 1440;
    boolean negate = (timezoneIndex < 0);
    timezoneIndex = Math.abs(timezoneIndex);
    int hour = timezoneIndex / 60;
    int min = timezoneIndex % 60;
    String tzName = String.format("GMT%s%02d:%02d", negate ? "-" : "+", hour, min);
    return TimeZone.getTimeZone(tzName);
  }
}
