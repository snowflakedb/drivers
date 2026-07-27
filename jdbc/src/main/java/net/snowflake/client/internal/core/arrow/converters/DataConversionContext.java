package net.snowflake.client.internal.core.arrow.converters;

import java.sql.Time;
import java.util.TimeZone;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;

public interface DataConversionContext {
  /**
   * Returns the formatter for DATE values, built from the session {@code DATE_OUTPUT_FORMAT}.
   * Default mirrors snowflake-jdbc's {@code "YYYY-MM-DD"}.
   */
  default SnowflakeDateTimeFormat getDateFormatter() {
    return SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD");
  }

  /**
   * The session timezone ({@code TIMEZONE} parameter), used as the source timezone when {@code
   * JDBC_FORMAT_DATE_WITH_TIMEZONE} shifts a DATE into a caller-supplied timezone. Default mirrors
   * snowflake-jdbc's {@code "America/Los_Angeles"}.
   */
  default TimeZone getSessionTimeZone() {
    return TimeZone.getTimeZone(Parameter.TIMEZONE.getDefaultVal());
  }

  /**
   * Whether DATE values are shifted by the timezone offset between the session timezone and the
   * caller-supplied timezone. Mirrors the {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} session parameter
   * (default false in snowflake-jdbc). When false, DATE getters return the raw epoch-day date with
   * no timezone adjustment.
   */
  default boolean isFormatDateWithTimezone() {
    return false;
  }

  /**
   * Connection-time fallback default for the date-with-timezone behavior. Mirrors snowflake-jdbc's
   * {@code SFBaseSession.getDefaultFormatDateWithTimezone()}, which defaults to {@code true} (set
   * from the client-only {@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE} property). When true, the
   * DATE converter's {@code toString}/{@code toObject}/{@code toTimestamp} ignore the runtime
   * {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} and use their own caller default — so the runtime flag
   * affects only the explicit {@code getDate(col, Calendar)} path. When false, those getters honor
   * the runtime flag instead.
   */
  default boolean isDefaultFormatDateWithTimezone() {
    return true;
  }

  /**
   * Whether scale-0 {@code FIXED}/{@code NUMBER} columns are surfaced as integers ({@code long}
   * from {@code getObject}, {@code BIGINT} from the result-set metadata) rather than as {@code
   * BigDecimal}/{@code DECIMAL}. Mirrors snowflake-jdbc's {@code JDBC_TREAT_DECIMAL_AS_INT} session
   * parameter, which defaults to {@code true}. The value converters and {@code
   * SnowflakeResultSetMetaDataImpl} both read this single flag so the reported column type and the
   * materialized object stay consistent.
   */
  default boolean isTreatDecimalAsInt() {
    return true;
  }

  /**
   * Mirrors snowflake-jdbc's {@code JDBC_GET_DATE_USE_NULL_TIMEZONE} (default true in {@code
   * SFBaseSession.getDateUseNullTimezone}). Consulted only by the no-Calendar {@code getDate(int)}:
   * when true it passes a null timezone to the converter (the raw epoch-day date), when false it
   * passes {@code TimeZone.getDefault()} so the date is timezone-shifted whenever {@code
   * JDBC_FORMAT_DATE_WITH_TIMEZONE} is also set. Connection-time only.
   */
  default boolean isGetDateUseNullTimezone() {
    return true;
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
   * The default generic {@code TIMESTAMP_OUTPUT_FORMAT}, used as the fallback for the three
   * per-type timestamp formatters. Mirrors snowflake-jdbc's {@code ResultUtil} default.
   */
  String DEFAULT_TIMESTAMP_OUTPUT_FORMAT = "DY, DD MON YYYY HH24:MI:SS TZHTZM";

  /**
   * Formatter for {@code TIMESTAMP_NTZ} {@code getString}, built from {@code
   * TIMESTAMP_NTZ_OUTPUT_FORMAT} with {@code TIMESTAMP_OUTPUT_FORMAT} as the fallback. Consumed by
   * the NTZ converter (TODO P1). Default mirrors snowflake-jdbc's empty-per-type →
   * generic-fallback.
   */
  default SnowflakeDateTimeFormat getTimestampNTZFormatter() {
    return SnowflakeDateTimeFormat.fromSqlFormat(DEFAULT_TIMESTAMP_OUTPUT_FORMAT);
  }

  /**
   * Formatter for {@code TIMESTAMP_LTZ} {@code getString}, built from {@code
   * TIMESTAMP_LTZ_OUTPUT_FORMAT} with {@code TIMESTAMP_OUTPUT_FORMAT} as the fallback. Consumed by
   * the LTZ converter (TODO P2).
   */
  default SnowflakeDateTimeFormat getTimestampLTZFormatter() {
    return SnowflakeDateTimeFormat.fromSqlFormat(DEFAULT_TIMESTAMP_OUTPUT_FORMAT);
  }

  /**
   * Formatter for {@code TIMESTAMP_TZ} {@code getString}, built from {@code
   * TIMESTAMP_TZ_OUTPUT_FORMAT} with {@code TIMESTAMP_OUTPUT_FORMAT} as the fallback. Consumed by
   * the TZ converter (TODO P3).
   */
  default SnowflakeDateTimeFormat getTimestampTZFormatter() {
    return SnowflakeDateTimeFormat.fromSqlFormat(DEFAULT_TIMESTAMP_OUTPUT_FORMAT);
  }

  /**
   * Whether {@code TIMESTAMP_NTZ} values keep their UTC wall-clock instead of being re-anchored to
   * the client/session timezone. Mirrors the client-only {@code JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC}
   * property (default false in snowflake-jdbc). Consumed by the NTZ read path (TODO P1).
   */
  default boolean isTreatNTZAsUTC() {
    return false;
  }

  /**
   * Whether {@code TIMESTAMP_NTZ} values are re-anchored from UTC into the caller's timezone.
   * Mirrors the {@code CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ} session parameter (default true in
   * snowflake-jdbc). Consumed by the NTZ read path (TODO P1).
   */
  default boolean isHonorClientTZForTimestampNTZ() {
    return true;
  }

  /**
   * The concrete timestamp type a bare {@code setTimestamp} binds as. Mirrors the {@code
   * CLIENT_TIMESTAMP_TYPE_MAPPING} session parameter (default {@code TIMESTAMP_LTZ} in
   * snowflake-jdbc). Consumed by the write/bind path (TODO P4).
   */
  default String getTimestampMappedType() {
    return "TIMESTAMP_LTZ";
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
