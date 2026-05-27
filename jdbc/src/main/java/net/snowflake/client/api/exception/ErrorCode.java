package net.snowflake.client.api.exception;

import lombok.AllArgsConstructor;
import lombok.Getter;

@Getter
@AllArgsConstructor
public enum ErrorCode {
  INTERNAL_ERROR(200001, null),
  INVALID_VALUE_CONVERT(200038, null),
  ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED(200023, "0A000");

  private final int messageCode;
  private final String sqlState;
}
