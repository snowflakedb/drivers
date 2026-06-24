package net.snowflake.client.api.exception;

import java.text.MessageFormat;
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
    Object[] args = params == null ? new Object[0] : params;
    String template = errorCode.getMessageTemplate();
    if (template != null) {
      return MessageFormat.format(template, args);
    }
    if (args.length == 0) {
      return String.valueOf(errorCode);
    }
    return String.format("%s: %s", errorCode, Arrays.toString(args));
  }
}
