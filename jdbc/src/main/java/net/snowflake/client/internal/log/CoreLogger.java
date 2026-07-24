package net.snowflake.client.internal.log;

import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.unicore.CoreLoggingBridge;
import net.snowflake.client.internal.util.MaskedException;

/**
 * Routes wrapper logs through {@code sf_core} and back onto the configured delivery logger. See
 * {@code doc/logging/logging-architecture.md}.
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

  public CoreLogger(String name) {
    this(name, SFLoggerFactory.createDeliveryLogger(name), DEFAULT_SINK);
  }

  public CoreLogger(Class<?> clazz) {
    this(clazz.getName());
  }

  @Override
  protected boolean isLevelEnabled(LogLevel level) {
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
  protected void logPlain(LogLevel level, String msg, boolean isMasked) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      route(level, isMasked ? LogFormatter.mask(msg) : msg);
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
      route(level, LogFormatter.appendThrowable(formatted.getMessage(), formatted.getThrowable()));
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
      route(
          level,
          LogFormatter.appendThrowable(
              LogFormatter.mask(msg), t == null ? null : new MaskedException(t)));
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
        delegate.error(message, false);
        break;
      case WARN:
        delegate.warn(message, false);
        break;
      case INFO:
        delegate.info(message, false);
        break;
      case DEBUG:
        delegate.debug(message, false);
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
