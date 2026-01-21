package net.snowflake.client.jdbc;

import java.util.Arrays;

public class SFException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  private final ErrorCode errorCode;

  public SFException(ErrorCode errorCode, Object... params) {
    super(buildMessage(errorCode, params));
    this.errorCode = errorCode;
  }

  public ErrorCode getErrorCode() {
    return errorCode;
  }

  private static String buildMessage(ErrorCode errorCode, Object... params) {
    if (params == null || params.length == 0) {
      return String.valueOf(errorCode);
    }
    return String.format("%s: %s", errorCode, Arrays.toString(params));
  }
}
