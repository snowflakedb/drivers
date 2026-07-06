package net.snowflake.client.internal.jdbc;

import java.sql.Date;
import java.text.DateFormat;
import java.text.SimpleDateFormat;
import java.util.TimeZone;

/**
 * Date with {@code toString()} overridden to display date values in the session timezone. Only
 * relevant for timestamp values fetched as dates; normal date objects carry no timezone. Ported 1:1
 * from snowflake-jdbc's {@code net.snowflake.client.jdbc.SnowflakeDateWithTimezone}.
 */
public class SnowflakeDateWithTimezone extends Date {

  // Dates fetched from timestamps always render as a plain calendar date, independent of any
  // session DATE_OUTPUT_FORMAT (matches snowflake-jdbc).
  private static final String DATE_FORMAT = "yyyy-MM-dd";

  TimeZone timezone = TimeZone.getDefault();
  boolean useSessionTimezone = false;

  public SnowflakeDateWithTimezone(long date, TimeZone timezone, boolean useSessionTimezone) {
    super(date);
    this.timezone = timezone;
    this.useSessionTimezone = useSessionTimezone;
  }

  /**
   * Returns a string representation in the carried timezone so as to display "wallclock time".
   *
   * @return a string representation of the object
   */
  @Override
  public synchronized String toString() {
    if (!useSessionTimezone) {
      return super.toString();
    }
    DateFormat formatter = new SimpleDateFormat(DATE_FORMAT);
    formatter.setTimeZone(this.timezone);
    return formatter.format(this);
  }
}
