package net.snowflake.client.internal.jdbc;

import java.sql.Time;
import java.sql.Timestamp;
import java.time.LocalDateTime;
import java.time.ZoneId;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.util.TimeZone;
import lombok.Getter;
import net.snowflake.client.internal.util.SnowflakeUtil;

/**
 * Time with {@code toString()} overridden to display time values in the session timezone. Only
 * relevant for timestamp values fetched as times; normal time objects carry no timezone. Ported 1:1
 * from snowflake-jdbc's {@code net.snowflake.client.jdbc.SnowflakeTimeWithTimezone}.
 */
public class SnowflakeTimeWithTimezone extends Time {

  @Getter int nano = 0;
  boolean useSessionTimeZone = false;
  @Getter ZoneOffset offset = ZoneOffset.UTC;

  public SnowflakeTimeWithTimezone(long time, int nanos, boolean useSessionTimeZone) {
    super(time);
    this.nano = nanos;
    this.useSessionTimeZone = useSessionTimeZone;
  }

  public SnowflakeTimeWithTimezone(
      Timestamp ts, TimeZone sessionTimeZone, boolean useSessionTimeZone) {
    super(ts.getTime());
    this.nano = ts.getNanos();
    this.useSessionTimeZone = useSessionTimeZone;
    if (sessionTimeZone != null) {
      this.offset = ZoneId.of(sessionTimeZone.getID()).getRules().getOffset(ts.toInstant());
    }
  }

  /**
   * Returns a string representation in the session's timezone so as to display "wallclock time".
   *
   * @return a string representation of the object
   */
  @Override
  public synchronized String toString() {
    if (!useSessionTimeZone) {
      return super.toString();
    }
    DateTimeFormatter formatter = DateTimeFormatter.ofPattern("HH:mm:ss");
    LocalDateTime ldt =
        LocalDateTime.ofEpochSecond(
            SnowflakeUtil.getSecondsFromMillis(this.getTime()), this.nano, this.offset);
    return ldt.format(formatter);
  }
}
