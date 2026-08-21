package net.snowflake.client.internal.api.implementation.parameters;

import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.util.StringUtil;

/** Centralized, typed access to all parameters for a single connection. */
public interface ParametersRegistry {

  SFLogger logger = SFLoggerFactory.getLogger(ParametersRegistry.class);
  FrozenParametersRegistry EMPTY = new FrozenParametersRegistry(null);

  /** Typed value for {@code param}, or {@code null} when unset. */
  ConfigSetting getTypedValue(Property param);

  /** Immutable, serializable snapshot of every known parameter. */
  FrozenParametersRegistry freeze();

  /** Raw display-string value for {@code param}, or {@code defaultValue}/empty when unset. */
  default String getRawValue(Property param, String defaultValue) {
    ConfigSetting value = getTypedValue(param);
    if (value == null) {
      return defaultValue;
    }
    if (value.hasStringValue()) {
      return value.getStringValue();
    }
    if (value.hasIntValue()) {
      return Long.toString(value.getIntValue());
    }
    if (value.hasBoolValue()) {
      return Boolean.toString(value.getBoolValue());
    }
    if (value.hasDoubleValue()) {
      return Double.toString(value.getDoubleValue());
    }
    return defaultValue;
  }

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
    ConfigSetting value = getTypedValue(param);
    if (value == null) {
      return defaultValue;
    }
    if (value.hasBoolValue()) {
      return value.getBoolValue();
    }
    if (value.hasStringValue()) {
      return Boolean.parseBoolean(value.getStringValue().trim());
    }
    return defaultValue;
  }

  default int getInt(Parameter param) {
    return getInt(param, Integer.parseInt(param.getDefaultVal()));
  }

  default int getInt(Property param, int defaultValue) {
    ConfigSetting value = getTypedValue(param);
    if (value == null) {
      return defaultValue;
    }
    if (value.hasIntValue()) {
      return (int) value.getIntValue();
    }
    if (value.hasStringValue()) {
      try {
        return Integer.parseInt(value.getStringValue().trim());
      } catch (NumberFormatException e) {
        logger.warn(
            "Non-integer value '{}' for {}; defaulting to {}",
            value.getStringValue(),
            param.getKey(),
            defaultValue,
            e);
        return defaultValue;
      }
    }
    return defaultValue;
  }
}
