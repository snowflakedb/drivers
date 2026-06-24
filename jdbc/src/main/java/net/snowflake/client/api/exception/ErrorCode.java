package net.snowflake.client.api.exception;

import lombok.AllArgsConstructor;
import lombok.Getter;

@Getter
@AllArgsConstructor
public enum ErrorCode {
  INTERNAL_ERROR(200001, null, null),
  INVALID_VALUE_CONVERT(
      200038, null, "Cannot convert value in the driver from type:{0} to type:{1}, value={2}."),
  ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED(200023, "0A000", null);

  private final int messageCode;
  private final String sqlState;
  private final String messageTemplate;
}
