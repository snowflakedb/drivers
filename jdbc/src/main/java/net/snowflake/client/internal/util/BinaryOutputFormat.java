package net.snowflake.client.internal.util;

import java.util.Base64;

/**
 * String encoding applied to BINARY values, mirroring the {@code BINARY_OUTPUT_FORMAT} session
 * parameter. Snowflake accepts the format name case-insensitively.
 */
public enum BinaryOutputFormat {
  HEX {
    @Override
    public String encode(byte[] value) {
      return HexUtil.bytesToHex(value);
    }
  },
  BASE64 {
    @Override
    public String encode(byte[] value) {
      return value == null ? null : Base64.getEncoder().encodeToString(value);
    }
  };

  public abstract String encode(byte[] value);

  /** Parses a session parameter value, defaulting to {@link #HEX} when no value is provided. */
  public static BinaryOutputFormat fromParameterValue(String value) {
    if (value == null) {
      return HEX;
    }
    for (BinaryOutputFormat format : values()) {
      if (format.name().equalsIgnoreCase(value.trim())) {
        return format;
      }
    }
    throw new IllegalArgumentException("Must be 'HEX' or 'BASE64'");
  }
}
