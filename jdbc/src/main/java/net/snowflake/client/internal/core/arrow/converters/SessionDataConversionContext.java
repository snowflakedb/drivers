package net.snowflake.client.internal.core.arrow.converters;

import java.util.TimeZone;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Getter;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;

@Getter
@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class SessionDataConversionContext implements DataConversionContext {

  private final SnowflakeDateTimeFormat dateFormatter;
  private final TimeZone sessionTimeZone;
  private final boolean formatDateWithTimezone;
  private final boolean defaultFormatDateWithTimezone;
  private final boolean getDateUseNullTimezone;
  private final SnowflakeDateTimeFormat timeFormatter;
  private final boolean useSessionTimezone;
  private final boolean treatTimeAsWallClockTime;
  private final SnowflakeDateTimeFormat timestampNTZFormatter;
  private final SnowflakeDateTimeFormat timestampLTZFormatter;
  private final SnowflakeDateTimeFormat timestampTZFormatter;
  private final boolean treatNTZAsUTC;
  private final boolean honorClientTZForTimestampNTZ;
  private final String timestampMappedType;
  private final boolean treatDecimalAsInt;

  public static DataConversionContext from(ParametersRegistry params) {
    SnowflakeDateTimeFormat dateFormatter =
        buildDateFormatter(params.get(Parameter.DATE_OUTPUT_FORMAT));
    TimeZone sessionTimeZone = buildSessionTimeZone(params.get(Parameter.TIMEZONE));
    SnowflakeDateTimeFormat timeFormatter =
        buildTimeFormatter(params.get(Parameter.TIME_OUTPUT_FORMAT));

    boolean formatDateWithTimezone = params.getBool(Parameter.JDBC_FORMAT_DATE_WITH_TIMEZONE);
    boolean defaultFormatDateWithTimezone =
        params.getBool(Parameter.JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE);
    boolean getDateUseNullTimezone = params.getBool(Parameter.JDBC_GET_DATE_USE_NULL_TIMEZONE);
    boolean useSessionTimezone = params.getBool(Parameter.JDBC_USE_SESSION_TIMEZONE);
    boolean treatTimeAsWallClockTime =
        params.getBool(Parameter.CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME);

    String genericTimestampFormat = params.get(Parameter.TIMESTAMP_OUTPUT_FORMAT);
    SnowflakeDateTimeFormat timestampNTZFormatter =
        buildTimestampFormatter(
            params.get(Parameter.TIMESTAMP_NTZ_OUTPUT_FORMAT, genericTimestampFormat));
    SnowflakeDateTimeFormat timestampLTZFormatter =
        buildTimestampFormatter(
            params.get(Parameter.TIMESTAMP_LTZ_OUTPUT_FORMAT, genericTimestampFormat));
    SnowflakeDateTimeFormat timestampTZFormatter =
        buildTimestampFormatter(
            params.get(Parameter.TIMESTAMP_TZ_OUTPUT_FORMAT, genericTimestampFormat));

    boolean honorClientTZForTimestampNTZ =
        params.getBool(Parameter.CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ);
    String timestampMappedType = params.get(Parameter.CLIENT_TIMESTAMP_TYPE_MAPPING);
    boolean treatNTZAsUTC = params.getBool(Parameter.JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC);
    boolean treatDecimalAsInt = params.getBool(Parameter.JDBC_TREAT_DECIMAL_AS_INT);

    return new SessionDataConversionContext(
        dateFormatter,
        sessionTimeZone,
        formatDateWithTimezone,
        defaultFormatDateWithTimezone,
        getDateUseNullTimezone,
        timeFormatter,
        useSessionTimezone,
        treatTimeAsWallClockTime,
        timestampNTZFormatter,
        timestampLTZFormatter,
        timestampTZFormatter,
        treatNTZAsUTC,
        honorClientTZForTimestampNTZ,
        timestampMappedType,
        treatDecimalAsInt);
  }

  static SnowflakeDateTimeFormat buildDateFormatter(String snowflakeFormat) {
    return SnowflakeDateTimeFormat.fromSqlFormat(snowflakeFormat);
  }

  static TimeZone buildSessionTimeZone(String timezone) {
    return TimeZone.getTimeZone(timezone);
  }

  static SnowflakeDateTimeFormat buildTimeFormatter(String snowflakeFormat) {
    return SnowflakeDateTimeFormat.fromSqlFormat(snowflakeFormat);
  }

  static SnowflakeDateTimeFormat buildTimestampFormatter(String format) {
    return SnowflakeDateTimeFormat.fromSqlFormat(format);
  }
}
