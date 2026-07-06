package net.snowflake.client.internal.core.arrow.converters;

import java.sql.SQLException;
import java.util.Map;
import java.util.Properties;
import java.util.TimeZone;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Getter;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.util.StringUtil;

@Getter
@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class SessionDataConversionContext implements DataConversionContext {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(SessionDataConversionContext.class);

  // snowflake-jdbc's default TIME_OUTPUT_FORMAT (see ResultUtil).
  private static final String DEFAULT_TIME_OUTPUT_FORMAT = "HH24:MI:SS";

  // snowflake-jdbc's default DATE_OUTPUT_FORMAT (see ResultUtil).
  private static final String DEFAULT_DATE_OUTPUT_FORMAT = "YYYY-MM-DD";

  // snowflake-jdbc's default TIMEZONE session parameter.
  private static final String DEFAULT_SESSION_TIMEZONE = "America/Los_Angeles";

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

  public static DataConversionContext fromConnection(
      CoreDriverApi coreDriverApi, ConnectionHandle handle, Properties clientProperties) {
    Map<String, String> params;
    try {
      ConnectionGetAllParametersResponse response =
          coreDriverApi.connectionGetAllParameters(handle);
      params = response.getParametersMap();
    } catch (SQLException e) {
      logger.debug("Falling back to default conversion context: {}", e.getMessage());
      return new DataConversionContext() {};
    }

    SnowflakeDateTimeFormat dateFormatter = buildDateFormatter(params.get("DATE_OUTPUT_FORMAT"));
    TimeZone sessionTimeZone = buildSessionTimeZone(params.get("TIMEZONE"));
    SnowflakeDateTimeFormat timeFormatter = buildTimeFormatter(params.get("TIME_OUTPUT_FORMAT"));
    // JDBC_FORMAT_DATE_WITH_TIMEZONE, JDBC_USE_SESSION_TIMEZONE and
    // CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME default false in snowflake-jdbc (see SFSessionProperty).
    // NOTE: this reads the ACCOUNT-level default of JDBC_FORMAT_DATE_WITH_TIMEZONE (often true),
    // not the client property. Only getDate(col, Calendar) consults it; the DATE converter's
    // toString/toObject/toTimestamp use the client-side value instead (see
    // DateConverter.getUseDateFormat) to match legacy.
    boolean formatDateWithTimezone =
        parseBoolean(params.get("JDBC_FORMAT_DATE_WITH_TIMEZONE"), false);
    // TODO(SNOW-3243330): the JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE and
    // JDBC_GET_DATE_USE_NULL_TIMEZONE properties are CLIENT-ONLY and the server never echoes them,
    // so reading them here always falls through to the defaults below — a customer cannot override
    // them today, unlike snowflake-jdbc. Full parity needs the resolved client Properties threaded
    // into fromConnection(...) and read first (then the server map, then these defaults), plus
    // exposing them on SnowflakeSessionProperty / the DataSource. Until then the reads below are
    // effectively inert.
    boolean defaultFormatDateWithTimezone =
        parseBoolean(params.get("JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE"), true);
    boolean getDateUseNullTimezone =
        parseBoolean(params.get("JDBC_GET_DATE_USE_NULL_TIMEZONE"), true);
    boolean useSessionTimezone = parseBoolean(params.get("JDBC_USE_SESSION_TIMEZONE"), false);
    boolean treatTimeAsWallClockTime =
        parseBoolean(params.get("CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME"), false);

    // Timestamp output formatters: each per-type format falls back to the generic
    // TIMESTAMP_OUTPUT_FORMAT when unset/empty, which itself falls back to its hard default.
    // Mirrors
    // snowflake-jdbc's ResultUtil.specializedFormatter +
    // SnowflakeDateTimeFormat.effectiveSpecializedTimestampFormat.
    String genericTimestampFormat =
        orDefault(params.get("TIMESTAMP_OUTPUT_FORMAT"), DEFAULT_TIMESTAMP_OUTPUT_FORMAT);
    SnowflakeDateTimeFormat timestampNTZFormatter =
        buildTimestampFormatter(params.get("TIMESTAMP_NTZ_OUTPUT_FORMAT"), genericTimestampFormat);
    SnowflakeDateTimeFormat timestampLTZFormatter =
        buildTimestampFormatter(params.get("TIMESTAMP_LTZ_OUTPUT_FORMAT"), genericTimestampFormat);
    SnowflakeDateTimeFormat timestampTZFormatter =
        buildTimestampFormatter(params.get("TIMESTAMP_TZ_OUTPUT_FORMAT"), genericTimestampFormat);
    // CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ and CLIENT_TIMESTAMP_TYPE_MAPPING are server-echoed
    // session parameters (defaults true / TIMESTAMP_LTZ).
    boolean honorClientTZForTimestampNTZ =
        parseBoolean(params.get("CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ"), true);
    String timestampMappedType =
        orDefault(params.get("CLIENT_TIMESTAMP_TYPE_MAPPING"), "TIMESTAMP_LTZ");
    // JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC is client-side in snowflake-jdbc (SessionUtil reads it from
    // the Properties bag). We read the client Properties first, then the server param map (the
    // reference driver applies the server-echoed value, which ALTER SESSION drives), then the
    // default — so a customer override wins and, absent one, we match the server value
    // (SNOW-3243330).
    boolean treatNTZAsUTC =
        clientThenServer(clientProperties, params, "JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC", false);

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
        timestampMappedType);
  }

  static boolean parseBoolean(String value, boolean defaultValue) {
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    return Boolean.parseBoolean(value.trim());
  }

  /**
   * TODO revisit this in the near future Resolve a boolean session flag client-property-first, then
   * the server parameter map, then the default. Mirrors snowflake-jdbc, where a client-only
   * property (set in the {@link Properties} bag) takes precedence and otherwise the server-echoed
   * session value applies. Property keys are matched case-insensitively (the JDBC {@code
   * Properties} bag uses the documented mixed casing).
   */
  static boolean clientThenServer(
      Properties clientProperties, Map<String, String> params, String key, boolean defaultValue) {
    if (clientProperties != null) {
      String clientValue = clientProperties.getProperty(key);
      if (clientValue == null) {
        clientValue = clientProperties.getProperty(key.toLowerCase());
      }
      if (clientValue != null && !clientValue.isEmpty()) {
        return Boolean.parseBoolean(clientValue.trim());
      }
    }
    return parseBoolean(params.get(key), defaultValue);
  }

  static SnowflakeDateTimeFormat buildDateFormatter(String snowflakeFormat) {
    return SnowflakeDateTimeFormat.fromSqlFormat(
        orDefault(snowflakeFormat, DEFAULT_DATE_OUTPUT_FORMAT));
  }

  static TimeZone buildSessionTimeZone(String timezone) {
    return TimeZone.getTimeZone(orDefault(timezone, DEFAULT_SESSION_TIMEZONE));
  }

  static SnowflakeDateTimeFormat buildTimeFormatter(String snowflakeFormat) {
    return SnowflakeDateTimeFormat.fromSqlFormat(
        orDefault(snowflakeFormat, DEFAULT_TIME_OUTPUT_FORMAT));
  }

  /**
   * Build a per-type timestamp formatter, falling back to the (already-resolved) generic
   * TIMESTAMP_OUTPUT_FORMAT when the specialized format is null/empty. Mirrors snowflake-jdbc's
   * {@code SnowflakeDateTimeFormat.effectiveSpecializedTimestampFormat}.
   */
  static SnowflakeDateTimeFormat buildTimestampFormatter(String specializedFormat, String generic) {
    return SnowflakeDateTimeFormat.fromSqlFormat(orDefault(specializedFormat, generic));
  }

  private static String orDefault(String value, String defaultValue) {
    return StringUtil.isNullOrEmpty(value) ? defaultValue : value;
  }
}
