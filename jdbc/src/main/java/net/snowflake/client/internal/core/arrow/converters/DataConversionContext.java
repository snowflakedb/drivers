package net.snowflake.client.internal.core.arrow.converters;

import java.sql.Time;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;

public interface DataConversionContext {
  // TODO: Populate from the session's DATE_OUTPUT_FORMAT parameter.
  // TODO: Once populated, implement Snowflake-to-Java format translation before use in
  //  DateConverter (Snowflake "YYYY-MM-DD" != Java "yyyy-MM-dd").
  /** Returns the date output format as a Java DateTimeFormatter pattern. */
  default String getDateOutputFormat() {
    return "yyyy-MM-dd";
  }

  /**
   * Returns the formatter for TIME values, built from the session {@code TIME_OUTPUT_FORMAT}.
   * Default mirrors snowflake-jdbc's {@code "HH24:MI:SS"} (no fractional seconds).
   */
  default SnowflakeDateTimeFormat getTimeFormatter() {
    return SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI:SS");
  }

  /**
   * Whether the session timezone should be applied when materializing TIME values. Mirrors the
   * {@code JDBC_USE_SESSION_TIMEZONE} session parameter. When true, {@code getTime()} anchors the
   * wall-clock fields in the JVM default timezone (via {@code Time.valueOf(LocalTime)}) so {@code
   * toString()} reads correctly there; when false a UTC-anchored {@code Time} built from
   * millis-since-midnight is returned. Defaults to false, matching snowflake-jdbc.
   */
  default boolean isUseSessionTimezone() {
    return false;
  }

  /**
   * Whether bound {@code java.sql.Time} values are interpreted as wall-clock time. Mirrors the
   * {@code CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME} session parameter (default false in
   * snowflake-jdbc). When true, {@code setTime(Time)} reads the local-timezone wall-clock fields
   * via {@code toLocalTime().toNanoOfDay()}; when false {@code Time.getTime()} is treated as UTC
   * epoch milliseconds reduced modulo a day.
   */
  default boolean isTreatTimeAsWallClockTime() {
    return false;
  }

  /**
   * Converts a bound {@link Time} to nanoseconds-since-midnight for the server, matching
   * snowflake-jdbc exactly. The bind-side mirror of {@code TimeConverter}'s nanos-to-{@link Time}
   * materialization, governed by {@link #isTreatTimeAsWallClockTime()}.
   *
   * <ul>
   *   <li>{@code isTreatTimeAsWallClockTime() == false} (the default, mirrors {@code
   *       CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME=false} via {@code
   *       SfTimestampUtil.getTimeInNanoseconds}): treat {@link Time#getTime()} as UTC epoch
   *       milliseconds reduced modulo a day.
   *   <li>{@code isTreatTimeAsWallClockTime() == true}: read the JVM-local wall-clock fields via
   *       {@code Time#toLocalTime().toNanoOfDay()}. Like snowflake-jdbc this drops sub-second
   *       milliseconds, because {@code Time#toLocalTime()} is {@code LocalTime.of(h, m, s)}.
   * </ul>
   */
  default long timeToNanosOfDay(Time x) {
    if (isTreatTimeAsWallClockTime()) {
      return x.toLocalTime().toNanoOfDay();
    }
    long msSinceMidnight = ((x.getTime() % 86_400_000L) + 86_400_000L) % 86_400_000L;
    return msSinceMidnight * 1_000_000L;
  }
}
