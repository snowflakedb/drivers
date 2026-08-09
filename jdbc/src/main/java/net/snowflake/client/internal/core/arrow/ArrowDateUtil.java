package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.util.Calendar;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;

public class ArrowDateUtil {
  /** Number of milliseconds in a day, mirroring snowflake-jdbc's {@code ResultUtil}. */
  public static final long MILLIS_IN_ONE_DAY = 86400000L;

  /** Milliseconds-since-epoch of 1582-10-05; dates before this need the Julian→Gregorian shift. */
  private static final long GREGORIAN_CUTOVER_MILLIS = -12220156800000L;

  /**
   * Convert an epoch-day count to a {@link Date} with no timezone adjustment. Mirrors
   * snowflake-jdbc's {@code ArrowResultUtil.getDate(int)}.
   *
   * @param day days since 1970-01-01
   * @return the date at local midnight for {@code day}
   */
  public static Date getDate(int day) {
    return Date.valueOf(LocalDate.ofEpochDay(day));
  }

  /**
   * Convert an epoch-day count to a {@link Date}, shifting from {@code oldTz} to {@code newTz} and
   * applying the Julian→Gregorian correction for pre-1582-10-05 dates. Mirrors snowflake-jdbc's
   * {@code ArrowResultUtil.getDate(int, TimeZone, TimeZone)}.
   *
   * @param day days since 1970-01-01
   * @param oldTz the source (session) timezone
   * @param newTz the target (JVM/Calendar) timezone
   * @return the adjusted date
   */
  public static Date getDate(int day, TimeZone oldTz, TimeZone newTz) {
    try {
      long milliSecsSinceEpoch = (long) day * MILLIS_IN_ONE_DAY;
      long milliSecsSinceEpochNew =
          milliSecsSinceEpoch + moveToTimeZoneOffset(milliSecsSinceEpoch, oldTz, newTz);

      Date preDate = new Date(milliSecsSinceEpochNew);

      // if date is on or before 1582-10-04, apply the difference
      // by (H-H/4-2) where H is the hundreds digit of the year according to:
      // http://en.wikipedia.org/wiki/Gregorian_calendar
      return adjustDate(preDate);
    } catch (NumberFormatException ex) {
      throw SFSQLException.fromErrorCode(ErrorCode.INTERNAL_ERROR, "Invalid date value: " + day);
    }
  }

  /**
   * Compute the wire value for a DATE bind: milliseconds-since-epoch of {@code date} measured in
   * {@code tz}, with the Julian→Gregorian correction backed out for pre-1582-10-05 dates. This is
   * the bind-side inverse of {@link #getDate(int, TimeZone, TimeZone)} and mirrors snowflake-jdbc's
   * {@code setDate} path.
   *
   * @param date the date to bind
   * @param tz the timezone the value is interpreted in (JVM default, or the Calendar's zone)
   * @return milliseconds-since-epoch to send to the server
   */
  public static long dateToBindMillis(java.util.Date date, TimeZone tz) {
    return date.getTime() + tz.getOffset(date.getTime()) - msDiffJulianToGregorian(date);
  }

  /**
   * Compute the wire value for a plain {@code setTimestamp(int, Timestamp)} bind: the instant as an
   * epoch-nanoseconds decimal string, with the Julian→Gregorian correction backed out for
   * pre-1582-10-05 instants. No client-side timezone is applied — {@code TIMESTAMP_LTZ} and {@code
   * TIMESTAMP_NTZ} produce the identical numeric string; only the bind type name differs. Mirrors
   * snowflake-jdbc's {@code SnowflakePreparedStatementV1.setTimestampWithType}.
   *
   * @param ts the timestamp to bind (must be non-null)
   * @return epoch-nanoseconds decimal string to send to the server
   */
  public static String timestampToBindString(Timestamp ts) {
    return String.valueOf(
        BigDecimal.valueOf((ts.getTime() - msDiffJulianToGregorian(ts)) / 1000)
            .scaleByPowerOfTen(9)
            .add(BigDecimal.valueOf(ts.getNanos())));
  }

  /**
   * Compute the wire value for a {@code setTimestamp(int, Timestamp, Calendar)} bind that resolves
   * to {@code TIMESTAMP_LTZ} or {@code TIMESTAMP_NTZ}: the instant is shifted by the Calendar
   * timezone's offset before being encoded as an epoch-nanoseconds decimal string. Unlike the plain
   * setter, the Calendar overload applies <b>no</b> Julian→Gregorian correction. Mirrors
   * snowflake-jdbc's {@code SnowflakePreparedStatementV1.setTimestamp(int, Timestamp, Calendar)}
   * (non-TZ branch).
   *
   * @param ts the timestamp to bind (must be non-null)
   * @param tz the Calendar's timezone
   * @return epoch-nanoseconds decimal string to send to the server
   */
  public static String timestampWithCalendarToBindString(Timestamp ts, TimeZone tz) {
    long milliSecSinceEpoch = ts.getTime() + tz.getOffset(ts.getTime());
    return String.valueOf(
        BigDecimal.valueOf(milliSecSinceEpoch / 1000)
            .scaleByPowerOfTen(9)
            .add(BigDecimal.valueOf(ts.getNanos())));
  }

  /**
   * Compute the wire value for a {@code setTimestamp(int, Timestamp, Calendar)} bind that resolves
   * to {@code TIMESTAMP_TZ}: {@code "<epoch_nanos> <offsetCode>"}, where the instant is <b>not</b>
   * shifted (only encoded as epoch-nanoseconds) and {@code offsetCode = offsetMinutesFromUtc +
   * 1440}. The {@code +1440} bias must match the read-side decode in the timestamp-with-timezone
   * converter. This is the only way to bind a {@code TIMESTAMP_TZ}. Applies no Julian→Gregorian
   * correction. Mirrors snowflake-jdbc's {@code SnowflakePreparedStatementV1.setTimestamp(int,
   * Timestamp, Calendar)} (TZ branch).
   *
   * @param ts the timestamp to bind (must be non-null)
   * @param tz the Calendar's timezone, whose offset at the instant becomes the stored offset
   * @return {@code "<epoch_nanos> <offsetCode>"} to send to the server
   */
  public static String timestampTzToBindString(Timestamp ts, TimeZone tz) {
    long milliSecSinceEpoch = ts.getTime();
    String value =
        String.valueOf(
            BigDecimal.valueOf(milliSecSinceEpoch / 1000)
                .scaleByPowerOfTen(9)
                .add(BigDecimal.valueOf(ts.getNanos())));
    int offset = tz.getOffset(milliSecSinceEpoch) / 60000 + 1440;
    return value + " " + offset;
  }

  /**
   * Offset (in millis) to move {@code milliSecsSinceEpoch} from {@code oldTZ} to {@code newTZ}.
   * Mirrors snowflake-jdbc's {@code ArrowResultUtil.moveToTimeZoneOffset}; uses {@code
   * Calendar.getInstance(oldTZ)} in place of the reference's {@code CalendarCache} (a perf-only
   * optimization absent from this module).
   */
  static long moveToTimeZoneOffset(long milliSecsSinceEpoch, TimeZone oldTZ, TimeZone newTZ) {
    if (oldTZ.hasSameRules(newTZ)) {
      // same time zone
      return 0;
    }
    int offsetMillisInOldTZ = oldTZ.getOffset(milliSecsSinceEpoch);

    Calendar calendar = Calendar.getInstance(oldTZ);
    calendar.setTimeInMillis(milliSecsSinceEpoch);

    int millisecondWithinDay =
        ((calendar.get(Calendar.HOUR_OF_DAY) * 60 + calendar.get(Calendar.MINUTE)) * 60
                    + calendar.get(Calendar.SECOND))
                * 1000
            + calendar.get(Calendar.MILLISECOND);

    int era = calendar.get(Calendar.ERA);
    int year = calendar.get(Calendar.YEAR);
    int month = calendar.get(Calendar.MONTH);
    int dayOfMonth = calendar.get(Calendar.DAY_OF_MONTH);
    int dayOfWeek = calendar.get(Calendar.DAY_OF_WEEK);

    int offsetMillisInNewTZ =
        newTZ.getOffset(era, year, month, dayOfMonth, dayOfWeek, millisecondWithinDay);

    return offsetMillisInOldTZ - offsetMillisInNewTZ;
  }

  /**
   * For dates before 1582-10-05, calculate the number of millis to adjust. Mirrors snowflake-jdbc's
   * {@code ResultUtil.msDiffJulianToGregorian}.
   *
   * @param date date to inspect
   * @return millis to adjust by (0 for dates on or after the cutover)
   */
  public static long msDiffJulianToGregorian(java.util.Date date) {
    // if date is before 1582-10-05, apply the difference
    // by (H-(H/4)-2) where H is the hundreds digit of the year according to:
    // http://en.wikipedia.org/wiki/Gregorian_calendar
    if (date.getTime() < GREGORIAN_CUTOVER_MILLIS) {
      // NOTE (faithful port of legacy's TODO-flagged hazard): the date's millis were computed in
      // one timezone, but Calendar.getInstance() resolves year/month/day in the JVM default zone.
      // Near a local-midnight boundary the resolved day can roll to the adjacent date, flipping the
      // month==1 && day<=28 heuristic below and shifting the correction by a full day. Low impact
      // (only pre-1582 dates), kept as-is for parity with snowflake-jdbc's ResultUtil.
      Calendar cal = Calendar.getInstance();
      cal.setTime(date);
      int year = cal.get(Calendar.YEAR);
      int month = cal.get(Calendar.MONTH);
      int dayOfMonth = cal.get(Calendar.DAY_OF_MONTH);

      // for dates on or before 02/28, use the previous year otherwise use the current year.
      if (month == 0 || (month == 1 && dayOfMonth <= 28)) {
        year = year - 1;
      }

      int hundreds = year / 100;
      int differenceInDays = hundreds - (hundreds / 4) - 2;

      return differenceInDays * MILLIS_IN_ONE_DAY;
    } else {
      return 0;
    }
  }

  /**
   * Adjust a date for the Julian→Gregorian shift (dates before 1582-10-05). Mirrors
   * snowflake-jdbc's {@code ResultUtil.adjustDate}.
   *
   * @param date date to adjust
   * @return the adjusted date (or the input when no adjustment is needed)
   */
  public static Date adjustDate(Date date) {
    long milliToAdjust = msDiffJulianToGregorian(date);
    if (milliToAdjust != 0) {
      return new Date(date.getTime() + milliToAdjust);
    } else {
      return date;
    }
  }

  /**
   * Adjust a timestamp for the Julian→Gregorian shift (dates before 1582-10-05), preserving
   * sub-second nanos. Mirrors snowflake-jdbc's {@code ResultUtil.adjustTimestamp}.
   *
   * @param timestamp timestamp to adjust
   * @return the adjusted timestamp (or the input when no adjustment is needed)
   */
  public static Timestamp adjustTimestamp(Timestamp timestamp) {
    long milliToAdjust = msDiffJulianToGregorian(timestamp);
    if (milliToAdjust != 0) {
      Timestamp newTimestamp = new Timestamp(timestamp.getTime() + milliToAdjust);
      newTimestamp.setNanos(timestamp.getNanos());
      return newTimestamp;
    } else {
      return timestamp;
    }
  }

  /**
   * Render a {@link Date} via the session date formatter in the JVM default timezone. Mirrors
   * snowflake-jdbc's {@code ResultUtil.getDateAsString}.
   *
   * @param date the date to render
   * @param dateFormatter the session {@code DATE_OUTPUT_FORMAT} formatter
   * @return formatted date string
   */
  public static String getDateAsString(Date date, SnowflakeDateTimeFormat dateFormatter) {
    return dateFormatter.format(date, TimeZone.getDefault());
  }

  private ArrowDateUtil() {}
}
