package net.snowflake.client.internal.api.implementation.exception;

import java.text.MessageFormat;
import java.util.Arrays;
import lombok.experimental.UtilityClass;
import net.snowflake.client.api.exception.ErrorCode;

/**
 * Renders the message of an {@link ErrorCode} from its {@link MessageFormat} template. Without a
 * template, or without an error code for the vendor code, the message is the code followed by the
 * arguments.
 */
@UtilityClass
public class ErrorMessageRenderer {

  public static String render(ErrorCode errorCode, Object... params) {
    Object[] args = params == null ? new Object[0] : params;
    String template = errorCode.getMessageTemplate();
    if (template != null) {
      return MessageFormat.format(template, args);
    }
    return fallback(String.valueOf(errorCode), args);
  }

  public static String render(int vendorCode, Object... params) {
    for (ErrorCode errorCode : ErrorCode.values()) {
      if (errorCode.getMessageCode() == vendorCode) {
        return render(errorCode, params);
      }
    }
    return fallback(String.valueOf(vendorCode), params == null ? new Object[0] : params);
  }

  private static String fallback(String code, Object[] args) {
    if (args.length == 0) {
      return code;
    }
    return String.format("%s: %s", code, Arrays.toString(args));
  }
}
