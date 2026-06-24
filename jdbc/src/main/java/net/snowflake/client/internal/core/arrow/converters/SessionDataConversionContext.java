package net.snowflake.client.internal.core.arrow.converters;

import java.sql.SQLException;
import java.util.Map;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

public final class SessionDataConversionContext implements DataConversionContext {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(SessionDataConversionContext.class);

  // snowflake-jdbc's default TIME_OUTPUT_FORMAT (see ResultUtil).
  private static final String DEFAULT_TIME_OUTPUT_FORMAT = "HH24:MI:SS";

  private final String dateOutputFormat;
  private final SnowflakeDateTimeFormat timeFormatter;
  private final boolean useSessionTimezone;
  private final boolean treatTimeAsWallClockTime;

  private SessionDataConversionContext(
      String dateOutputFormat,
      SnowflakeDateTimeFormat timeFormatter,
      boolean useSessionTimezone,
      boolean treatTimeAsWallClockTime) {
    this.dateOutputFormat = dateOutputFormat;
    this.timeFormatter = timeFormatter;
    this.useSessionTimezone = useSessionTimezone;
    this.treatTimeAsWallClockTime = treatTimeAsWallClockTime;
  }

  public static DataConversionContext fromConnection(
      CoreDriverApi coreDriverApi, ConnectionHandle handle) {
    Map<String, String> params;
    try {
      ConnectionGetAllParametersResponse response =
          coreDriverApi.connectionGetAllParameters(handle);
      params = response.getParametersMap();
    } catch (SQLException e) {
      logger.debug("Falling back to default conversion context: {}", e.getMessage());
      return new DataConversionContext() {};
    }

    String dateFormat = translateDateFormat(params.get("DATE_OUTPUT_FORMAT"));
    SnowflakeDateTimeFormat timeFormatter = buildTimeFormatter(params.get("TIME_OUTPUT_FORMAT"));
    // Both default to false in snowflake-jdbc (JDBC_USE_SESSION_TIMEZONE and
    // CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME, see SFSessionProperty).
    boolean useSessionTimezone = parseBoolean(params.get("JDBC_USE_SESSION_TIMEZONE"), false);
    boolean treatTimeAsWallClockTime =
        parseBoolean(params.get("CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME"), false);

    return new SessionDataConversionContext(
        dateFormat, timeFormatter, useSessionTimezone, treatTimeAsWallClockTime);
  }

  @Override
  public String getDateOutputFormat() {
    return dateOutputFormat;
  }

  @Override
  public SnowflakeDateTimeFormat getTimeFormatter() {
    return timeFormatter;
  }

  @Override
  public boolean isUseSessionTimezone() {
    return useSessionTimezone;
  }

  @Override
  public boolean isTreatTimeAsWallClockTime() {
    return treatTimeAsWallClockTime;
  }

  static boolean parseBoolean(String value, boolean defaultValue) {
    if (value == null || value.isEmpty()) {
      return defaultValue;
    }
    return Boolean.parseBoolean(value.trim());
  }

  static SnowflakeDateTimeFormat buildTimeFormatter(String snowflakeFormat) {
    String format =
        (snowflakeFormat == null || snowflakeFormat.isEmpty())
            ? DEFAULT_TIME_OUTPUT_FORMAT
            : snowflakeFormat;
    return SnowflakeDateTimeFormat.fromSqlFormat(format);
  }

  static String translateDateFormat(String snowflakeFormat) {
    if (snowflakeFormat == null || snowflakeFormat.isEmpty()) {
      return "yyyy-MM-dd";
    }
    return snowflakeFormat.replace("YYYY", "yyyy").replace("DD", "dd");
  }
}
