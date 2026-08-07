package net.snowflake.client.internal.api.implementation.parameters;

import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.util.StringUtil;

/** Centralized, typed access to all parameters for a single connection. */
public interface ParametersRegistry {

  SFLogger logger = SFLoggerFactory.getLogger(ParametersRegistry.class);
  FrozenParametersRegistry EMPTY = new FrozenParametersRegistry(null);

  /** Raw value for {@code param}, or {@code null}/empty when unset. */
  String getRawValue(Property param, String defaultValue);

  /** Immutable, serializable snapshot of every known parameter. */
  FrozenParametersRegistry freeze();

  default String get(Parameter param) {
    return get(param, param.getDefaultVal());
  }

  default String get(Property param, String defaultValue) {
    String value = getRawValue(param, defaultValue);
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    return value;
  }

  default String getOrThrow(Property param) {
    String value = getRawValue(param, null);
    if (StringUtil.isNullOrEmpty(value)) {
      throw new SFSQLException(
          ErrorCode.INTERNAL_ERROR, "Required parameter not found: " + param.getKey());
    }
    return value;
  }

  default boolean getBool(Parameter param) {
    return getBool(param, Boolean.parseBoolean(param.getDefaultVal()));
  }

  default boolean getBool(Property param, boolean defaultValue) {
    String value = getRawValue(param, Boolean.toString(defaultValue));
    if (StringUtil.isNullOrEmpty(value)) {
      return defaultValue;
    }
    return Boolean.parseBoolean(value.trim());
  }

  default int getInt(Parameter param) {
    return getInt(param, Integer.parseInt(param.getDefaultVal()));
  }

  default int getInt(Property param, int defaultValue) {
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
}
