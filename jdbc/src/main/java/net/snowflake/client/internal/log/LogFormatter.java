package net.snowflake.client.internal.log;

import java.io.PrintWriter;
import java.io.StringWriter;
import java.util.function.Supplier;
import lombok.Value;
import lombok.experimental.UtilityClass;
import net.snowflake.client.internal.util.MaskedException;
import net.snowflake.client.internal.util.SecretDetector;
import org.slf4j.helpers.FormattingTuple;
import org.slf4j.helpers.MessageFormatter;

/**
 * Shared message formatting and secret masking for JDBC loggers.
 *
 * <p>Extracted so {@link SLF4JLogger}, {@link JDK14Logger}, and {@link CoreLogger} (which ships the
 * formatted string across JNI to core before it round-trips back) produce identical, masked output
 * instead of each re-implementing SLF4J placeholder handling.
 */
@UtilityClass
class LogFormatter {

  @Value
  static class Formatted {
    String message;
    Throwable throwable;
  }

  // TODO(SNOW-3725887): secrets obfuscation should be done in core
  static String mask(String msg) {
    return SecretDetector.maskSecrets(msg);
  }

  // TODO(SNOW-3725887): secrets obfuscation should be done in core
  static Formatted format(String msg, Object... arguments) {
    FormattingTuple ft = MessageFormatter.arrayFormat(msg, evaluateLambdaArgs(arguments));
    String message = SecretDetector.maskSecrets(ft.getMessage());
    Throwable masked = ft.getThrowable() == null ? null : new MaskedException(ft.getThrowable());
    return new Formatted(message, masked);
  }

  /** Append the exception type when the full cause must not appear at WARN/INFO. */
  static String withTypeOnlyCause(String message, Throwable throwable) {
    if (throwable == null) {
      return message;
    }
    return message + ": " + throwable.getClass().getName();
  }

  static boolean deferThrowableDetailToDebug(AbstractSFLogger.LogLevel level, Throwable throwable) {
    return throwable != null
        && (level == AbstractSFLogger.LogLevel.WARN || level == AbstractSFLogger.LogLevel.INFO);
  }

  /** Core carries one string; append a masked stack trace when needed. */
  static String appendThrowable(String message, Throwable throwable) {
    if (throwable == null) {
      return message;
    }
    StringWriter sw = new StringWriter();
    throwable.printStackTrace(new PrintWriter(sw));
    return message + System.lineSeparator() + SecretDetector.maskSecrets(sw.toString());
  }

  private static Object[] evaluateLambdaArgs(Object... args) {
    if (args == null || args.length == 0) {
      return new Object[0];
    }
    final Object[] result = new Object[args.length];
    for (int i = 0; i < args.length; i++) {
      result[i] = args[i] instanceof Supplier ? ((Supplier<?>) args[i]).get() : args[i];
    }
    return result;
  }
}
