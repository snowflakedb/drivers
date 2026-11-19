package net.snowflake.client.internal.log;

import java.io.PrintWriter;
import java.io.StringWriter;
import java.lang.reflect.Array;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.Set;
import java.util.function.Supplier;
import lombok.Value;
import lombok.experimental.UtilityClass;

/**
 * Shared message formatting for JDBC loggers.
 *
 * <p>Substitutes {@code {}} the same way SLF4J {@code MessageFormatter} does, including {@code
 * \\{}} escapes and a trailing {@link Throwable} that is not consumed as a placeholder. {@link
 * SLF4JLogger}, {@link JDK14Logger}, and {@link CoreLogger} share this so JUL and SLF4J delivery
 * stay aligned without a runtime slf4j-api dependency.
 */
@UtilityClass
class LogFormatter {

  private static final String PLACEHOLDER = "{}";
  private static final char ESCAPE = '\\';

  @Value
  static class Formatted {
    String message;
    Throwable throwable;
  }

  static Formatted format(String msg, Object... arguments) {
    Object[] args = evaluateLambdaArgs(arguments);
    Throwable throwable = lastThrowable(args);
    if (throwable != null) {
      args = withoutLast(args);
    }
    return new Formatted(applyPlaceholders(msg, args), throwable);
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

  /** Core carries one string; append a stack trace when needed. */
  static String appendThrowable(String message, Throwable throwable) {
    if (throwable == null) {
      return message;
    }
    StringWriter sw = new StringWriter();
    throwable.printStackTrace(new PrintWriter(sw));
    return message + System.lineSeparator() + sw;
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

  private static Throwable lastThrowable(Object[] args) {
    if (args.length == 0) {
      return null;
    }
    Object last = args[args.length - 1];
    return last instanceof Throwable ? (Throwable) last : null;
  }

  private static Object[] withoutLast(Object[] args) {
    if (args.length == 1) {
      return new Object[0];
    }
    Object[] trimmed = new Object[args.length - 1];
    System.arraycopy(args, 0, trimmed, 0, trimmed.length);
    return trimmed;
  }

  private static String applyPlaceholders(String pattern, Object[] args) {
    if (pattern == null) {
      return null;
    }
    if (args.length == 0) {
      return pattern;
    }
    StringBuilder sbuf = new StringBuilder(pattern.length() + 50);
    int from = 0;
    int consumed = 0;
    while (consumed < args.length) {
      int placeholder = pattern.indexOf(PLACEHOLDER, from);
      if (placeholder == -1) {
        if (from == 0) {
          return pattern;
        }
        sbuf.append(pattern, from, pattern.length());
        return sbuf.toString();
      }
      if (isEscapedPlaceholder(pattern, placeholder)) {
        if (!isDoubleEscapedPlaceholder(pattern, placeholder)) {
          sbuf.append(pattern, from, placeholder - 1);
          sbuf.append('{');
          from = placeholder + 1;
          continue;
        }
        sbuf.append(pattern, from, placeholder - 1);
      } else {
        sbuf.append(pattern, from, placeholder);
      }
      appendValue(sbuf, args[consumed], newIdentitySet());
      consumed++;
      from = placeholder + 2;
    }
    sbuf.append(pattern, from, pattern.length());
    return sbuf.toString();
  }

  private static boolean isEscapedPlaceholder(String pattern, int placeholderIndex) {
    return placeholderIndex > 0 && pattern.charAt(placeholderIndex - 1) == ESCAPE;
  }

  private static boolean isDoubleEscapedPlaceholder(String pattern, int placeholderIndex) {
    return placeholderIndex >= 2 && pattern.charAt(placeholderIndex - 2) == ESCAPE;
  }

  private static Set<Object> newIdentitySet() {
    return Collections.newSetFromMap(new IdentityHashMap<>());
  }

  private static void appendValue(StringBuilder sbuf, Object value, Set<Object> visitedArrays) {
    if (value == null) {
      sbuf.append("null");
      return;
    }
    if (value.getClass().isArray()) {
      appendArray(sbuf, value, visitedArrays);
      return;
    }
    try {
      sbuf.append(value.toString());
    } catch (Throwable ignored) {
      sbuf.append("[FAILED toString()]");
    }
  }

  private static void appendArray(StringBuilder sbuf, Object array, Set<Object> visitedArrays) {
    sbuf.append('[');
    if (visitedArrays.add(array)) {
      int length = Array.getLength(array);
      for (int i = 0; i < length; i++) {
        if (i > 0) {
          sbuf.append(", ");
        }
        appendValue(sbuf, Array.get(array, i), visitedArrays);
      }
      visitedArrays.remove(array);
    } else {
      sbuf.append("...");
    }
    sbuf.append(']');
  }
}
