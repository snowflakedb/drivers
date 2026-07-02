package net.snowflake.client.internal.jdbc;

import java.sql.Timestamp;
import java.time.LocalDateTime;
import java.time.ZoneId;
import java.time.ZoneOffset;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.util.TimeZone;
import lombok.Getter;
import net.snowflake.client.internal.util.SnowflakeUtil;

/**
 * Timestamp with {@code toString()} overridden to render the value in its carried timezone (the
 * session timezone, the stored TZ offset, or UTC). The default timezone is UTC if none is
 * specified. Ported 1:1 from snowflake-jdbc's {@code net.snowflake.client.jdbc
 * .SnowflakeTimestampWithTimezone}.
 */
public class SnowflakeTimestampWithTimezone extends Timestamp {
  private static final long serialVersionUID = 1L;

  // Date/time portion of the toString() pattern; the fractional-second digits are appended
  // dynamically after trailing zeros are trimmed.
  private static final String BASE_FORMAT = "uuuu-MM-dd HH:mm:ss.";

  /** The timezone this timestamp is rendered in. */
  @Getter private TimeZone timezone = TimeZone.getTimeZone("UTC");

  public SnowflakeTimestampWithTimezone(long seconds, int nanoseconds, TimeZone timezone) {
    super(seconds);
    this.setNanos(nanoseconds);
    this.timezone = timezone;
  }

  public SnowflakeTimestampWithTimezone(Timestamp ts, TimeZone timezone) {
    this(ts.getTime(), ts.getNanos(), timezone);
  }

  public SnowflakeTimestampWithTimezone(Timestamp ts) {
    this(ts.getTime(), ts.getNanos(), TimeZone.getTimeZone("UTC"));
  }

  /**
   * Converts this timestamp to a zoned date time.
   *
   * @return the zoned date time corresponding to this timestamp.
   */
  public ZonedDateTime toZonedDateTime() {
    return ZonedDateTime.ofInstant(toInstant(), this.timezone.toZoneId());
  }

  /**
   * Returns a string representation rendered in the carried timezone, trimming trailing zeros from
   * the fractional seconds (printing 8 trailing digits when nanos are zero).
   *
   * @return a string representation of the object
   */
  @Override
  public synchronized String toString() {
    int trailingZeros = 0;
    int tmpNanos = this.getNanos();
    if (tmpNanos == 0) {
      trailingZeros = 8;
    } else {
      while (tmpNanos % 10 == 0) {
        tmpNanos /= 10;
        trailingZeros++;
      }
    }
    final String baseFormat = BASE_FORMAT;
    StringBuilder buf = new StringBuilder(baseFormat.length() + 9 - trailingZeros);
    buf.append(baseFormat);
    for (int i = 0; i < 9 - trailingZeros; ++i) {
      buf.append("S");
    }
    DateTimeFormatter formatter = DateTimeFormatter.ofPattern(buf.toString());

    ZoneOffset offset = ZoneId.of(timezone.getID()).getRules().getOffset(this.toInstant());
    LocalDateTime ldt =
        LocalDateTime.ofEpochSecond(
            SnowflakeUtil.getSecondsFromMillis(this.getTime()), this.getNanos(), offset);
    return ldt.format(formatter);
  }
}
