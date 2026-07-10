package net.snowflake.client.internal.api.implementation.parameters;

import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.util.StringUtil;

/**
 * Centralized access to all parameters for a single connection.
 *
 * <p>Parameters are fetched from core one at a time via {@code connectionGetParameter}, so values
 * always reflect the current session state (e.g. after {@code ALTER SESSION}).
 */
@RequiredArgsConstructor
public class ParametersRegistry {
  private static final SFLogger logger = SFLoggerFactory.getLogger(ParametersRegistry.class);

  private final CoreDriverApi coreDriverApi;
  private final ConnectionHandle handle;

  public String get(Parameter param) {
    return get(param, param.getDefaultVal());
  }

  public String get(Property param, String defaultValue) {
    String value = getRawValue(param, defaultValue);
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    return value;
  }

  public String getOrThrow(Property param) throws SQLException {
    String value = getRawValue(param, null);
    if (StringUtil.isNullOrEmpty(value)) {
      throw new SnowflakeSQLException(
          ErrorCode.INTERNAL_ERROR, "Required parameter not found: " + param.getKey());
    }
    return value;
  }

  public boolean getBool(Parameter param) {
    return getBool(param, Boolean.parseBoolean(param.getDefaultVal()));
  }

  public boolean getBool(Property param, boolean defaultValue) {
    String value = getRawValue(param, Boolean.toString(defaultValue));
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    return Boolean.parseBoolean(value.trim());
  }

  public int getInt(Parameter param) {
    return getInt(param, Integer.parseInt(param.getDefaultVal()));
  }

  public int getInt(Property param, int defaultValue) {
    String value = getRawValue(param, Integer.toString(defaultValue));
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    try {
      return Integer.parseInt(value.trim());
    } catch (NumberFormatException e) {
      logger.warn(
          "Non-integer value '{}' for {}; defaulting to {}",
          value,
          param.getKey(),
          defaultValue,
          e);
      return defaultValue;
    }
  }

  private String getRawValue(Property param, String defaultValue) {
    try {
      ConnectionGetParameterResponse response =
          coreDriverApi.connectionGetParameter(handle, param.getKey());
      if (response != null && response.hasValue()) {
        return response.getValue();
      }
    } catch (SQLException e) {
      logger.warn(
          "Failed to read {} session parameter; defaulting to {}", param.getKey(), defaultValue, e);
    }
    return null;
  }
}
