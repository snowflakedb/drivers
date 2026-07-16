package net.snowflake.client.internal.log;

import lombok.AccessLevel;
import lombok.Getter;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.unicore.CoreLoggingBridge;
import net.snowflake.client.internal.util.MaskedException;

/**
 * An {@link SFLogger} that routes wrapper (Java) logs through {@code sf_core} so they share the
 * single tracing pipeline (file, OTLP, in-band telemetry) with core's own logs, then come back onto
 * this logger's SLF4J logger via the bridge's {@code SFLoggerLayer}. See {@code
 * doc/logging/logging-architecture.md}.
 *
 * <p>An SLF4J-backed {@link #delegate} remains the single source of truth for levels — every log
 * method gates on it before crossing JNI, so a filtered message costs nothing — and is also the
 * fallback sink: when core reports the pipeline is not live (or the native lib is unavailable) the
 * already-formatted, masked message is emitted straight onto it, so early logs are never lost.
 */
@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
public class CoreLogger implements SFLogger {

  /** Test seam over {@link CoreLoggingBridge#logEvent}. */
  interface CoreLogEventSink {
    int send(int level, String message, String file, int line, String function, String loggerName);
  }

  private static final CoreLogEventSink DEFAULT_SINK = CoreLoggingBridge::logEvent;

  /**
   * Latched when the native call throws (the bridge lib is genuinely unavailable, e.g. failed to
   * load); a non-zero return is the pipeline "not live yet" signal and is retried instead. A failed
   * native load never recovers in-process, so once latched this logger goes straight to {@link
   * #delegate}. Per-instance (not static) so it stays a plain field, not global mutable state — a
   * fresh logger costs at most one extra failed native call before latching.
   */
  private volatile boolean nativeUnavailable = false;

  private final String name;
  private final SFLogger delegate;
  private final CoreLogEventSink sink;

  public CoreLogger(String name) {
    this(name, new SLF4JLogger(name), DEFAULT_SINK);
  }

  public CoreLogger(Class<?> clazz) {
    this(clazz.getName());
  }

  @Override
  public boolean isDebugEnabled() {
    return delegate.isDebugEnabled();
  }

  @Override
  public boolean isErrorEnabled() {
    return delegate.isErrorEnabled();
  }

  @Override
  public boolean isInfoEnabled() {
    return delegate.isInfoEnabled();
  }

  @Override
  public boolean isWarnEnabled() {
    return delegate.isWarnEnabled();
  }

  @Override
  public void debug(String msg, boolean isMasked) {
    logPlain(LogLevel.DEBUG, msg, isMasked);
  }

  @Override
  public void debug(String msg, Object... arguments) {
    logFormat(LogLevel.DEBUG, msg, arguments);
  }

  @Override
  public void debug(String msg, Throwable t) {
    logThrowable(LogLevel.DEBUG, msg, t);
  }

  @Override
  public void error(String msg, boolean isMasked) {
    logPlain(LogLevel.ERROR, msg, isMasked);
  }

  @Override
  public void error(String msg, Object... arguments) {
    logFormat(LogLevel.ERROR, msg, arguments);
  }

  @Override
  public void error(String msg, Throwable t) {
    logThrowable(LogLevel.ERROR, msg, t);
  }

  @Override
  public void info(String msg, boolean isMasked) {
    logPlain(LogLevel.INFO, msg, isMasked);
  }

  @Override
  public void info(String msg, Object... arguments) {
    logFormat(LogLevel.INFO, msg, arguments);
  }

  @Override
  public void info(String msg, Throwable t) {
    logThrowable(LogLevel.INFO, msg, t);
  }

  @Override
  public void warn(String msg, boolean isMasked) {
    logPlain(LogLevel.WARN, msg, isMasked);
  }

  @Override
  public void warn(String msg, Object... arguments) {
    logFormat(LogLevel.WARN, msg, arguments);
  }

  @Override
  public void warn(String msg, Throwable t) {
    logThrowable(LogLevel.WARN, msg, t);
  }

  private void logPlain(LogLevel level, String msg, boolean isMasked) {
    if (!level.isEnabled(delegate)) {
      return;
    }
    route(level, maskIf(msg, isMasked));
  }

  private void logFormat(LogLevel level, String msg, Object... arguments) {
    if (!level.isEnabled(delegate)) {
      return;
    }
    LogFormatter.Formatted formatted = LogFormatter.format(msg, arguments);
    route(level, LogFormatter.appendThrowable(formatted.getMessage(), formatted.getThrowable()));
  }

  private void logThrowable(LogLevel level, String msg, Throwable t) {
    if (!level.isEnabled(delegate)) {
      return;
    }
    route(
        level,
        LogFormatter.appendThrowable(
            LogFormatter.mask(msg), t == null ? null : new MaskedException(t)));
  }

  private static String maskIf(String msg, boolean isMasked) {
    return isMasked ? LogFormatter.mask(msg) : msg;
  }

  /**
   * Send the fully-formatted, masked message to core; fall back to the delegate when core is not
   * live. Source location (file/line/function) is left empty: Java 8 has no cheap single-frame
   * access (StackWalker is 9+) and a full stack capture per log is too costly. ponytail: upgrade to
   * StackWalker to carry source location once the driver targets Java 9+.
   */
  private void route(LogLevel level, String message) {
    if (!nativeUnavailable) {
      try {
        if (sink.send(level.getCoreLevel(), message, "", 0, "", name)
            == CoreLoggingBridge.CORE_DELIVERED) {
          return;
        }
        // Pipeline not live yet; deliver straight onto the delegate below.
      } catch (Throwable t) {
        // Native bridge unavailable; latch and deliver straight onto the delegate below.
        nativeUnavailable = true;
      }
    }
    try {
      level.fallback(delegate, message);
    } catch (Throwable ignored) {
      // Last resort: both the core round-trip and direct SLF4J delivery failed.
    }
  }

  /** sf_core wire levels; DEBUG is finest. */
  @Getter
  @RequiredArgsConstructor
  private enum LogLevel {
    ERROR(0),
    WARN(1),
    INFO(2),
    DEBUG(3);

    private final int coreLevel;

    boolean isEnabled(SFLogger delegate) {
      switch (this) {
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

    void fallback(SFLogger delegate, String message) {
      switch (this) {
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
  }
}
