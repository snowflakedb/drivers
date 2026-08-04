package net.snowflake.client.internal.log;

import java.util.function.BooleanSupplier;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.unicore.CoreLoggingBridge;

/**
 * Routes wrapper logs through {@code sf_core} and back onto the configured delivery logger. See
 * {@code docs/logging/logging-architecture.md}.
 */
@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
public class CoreLogger extends AbstractSFLogger {

  /** Test seam over {@link CoreLoggingBridge#logEvent}. */
  interface CoreLogEventSink {
    int send(int level, String message, String file, int line, String function, String loggerName);
  }

  private static final CoreLogEventSink DEFAULT_SINK = CoreLoggingBridge::logEvent;

  private volatile boolean nativeUnavailable = false;

  private final String name;
  private final SFLogger delegate;
  private final CoreLogEventSink sink;
  private final BooleanSupplier troubleshooting;

  public CoreLogger(String name) {
    this(
        name,
        SFLoggerFactory.createDeliveryLogger(name),
        DEFAULT_SINK,
        CoreLoggingBridge::isTroubleshooting);
  }

  public CoreLogger(Class<?> clazz) {
    this(clazz.getName());
  }

  @Override
  protected boolean isLevelEnabled(LogLevel level) {
    if (troubleshooting.getAsBoolean()) {
      return true;
    }
    switch (level) {
      case ERROR:
        return delegate.isErrorEnabled();
      case WARN:
        return delegate.isWarnEnabled();
      case INFO:
        return delegate.isInfoEnabled();
      case DEBUG:
        return delegate.isDebugEnabled();
      default:
        return false;
    }
  }

  @Override
  protected void logPlain(LogLevel level, String msg) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      route(level, msg);
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  @Override
  protected void logFormat(LogLevel level, String msg, Object... arguments) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      LogFormatter.Formatted formatted = LogFormatter.format(msg, arguments);
      Throwable throwable = formatted.getThrowable();
      if (LogFormatter.deferThrowableDetailToDebug(level, throwable)) {
        route(level, LogFormatter.withTypeOnlyCause(formatted.getMessage(), throwable));
        if (isLevelEnabled(LogLevel.DEBUG)) {
          route(LogLevel.DEBUG, LogFormatter.appendThrowable(formatted.getMessage(), throwable));
        }
      } else {
        route(level, LogFormatter.appendThrowable(formatted.getMessage(), throwable));
      }
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  @Override
  protected void logThrowable(LogLevel level, String msg, Throwable t) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      if (LogFormatter.deferThrowableDetailToDebug(level, t)) {
        route(level, LogFormatter.withTypeOnlyCause(msg, t));
        if (isLevelEnabled(LogLevel.DEBUG)) {
          route(LogLevel.DEBUG, LogFormatter.appendThrowable(msg, t));
        }
      } else {
        route(level, LogFormatter.appendThrowable(msg, t));
      }
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  private void route(LogLevel level, String message) {
    if (!nativeUnavailable) {
      try {
        if (sink.send(toCoreLevel(level), message, "", 0, "", name)
            == CoreLoggingBridge.CORE_DELIVERED) {
          return;
        }
      } catch (Throwable t) {
        nativeUnavailable = true;
      }
    }
    try {
      fallback(level, message);
    } catch (Throwable ignored) {
      // Last resort: both core round-trip and direct delivery failed.
    }
  }

  private void fallback(LogLevel level, String message) {
    switch (level) {
      case ERROR:
        delegate.error(message);
        break;
      case WARN:
        delegate.warn(message);
        break;
      case INFO:
        delegate.info(message);
        break;
      case DEBUG:
        delegate.debug(message);
        break;
      default:
        break;
    }
  }

  private static int toCoreLevel(LogLevel level) {
    switch (level) {
      case ERROR:
        return 0;
      case WARN:
        return 1;
      case INFO:
        return 2;
      case DEBUG:
        return 3;
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
  }
}
