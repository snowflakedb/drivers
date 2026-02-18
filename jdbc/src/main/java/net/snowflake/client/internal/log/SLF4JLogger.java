package net.snowflake.client.internal.log;

import java.util.function.Supplier;
import net.snowflake.client.internal.util.MaskedException;
import net.snowflake.client.internal.util.SecretDetector;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.slf4j.helpers.FormattingTuple;
import org.slf4j.helpers.MessageFormatter;
import org.slf4j.spi.LocationAwareLogger;

public class SLF4JLogger implements SFLogger {
  private static final String FQCN = SLF4JLogger.class.getName();

  private final Logger slf4jLogger;
  private final boolean isLocationAwareLogger;

  public SLF4JLogger(Class<?> clazz) {
    this(LoggerFactory.getLogger(requireLoggerClass(clazz)));
  }

  public SLF4JLogger(String name) {
    this(LoggerFactory.getLogger(requireLoggerName(name)));
  }

  SLF4JLogger(Logger slf4jLogger) {
    this.slf4jLogger = slf4jLogger;
    this.isLocationAwareLogger = this.slf4jLogger instanceof LocationAwareLogger;
  }

  @Override
  public boolean isDebugEnabled() {
    return this.slf4jLogger.isDebugEnabled();
  }

  @Override
  public boolean isErrorEnabled() {
    return this.slf4jLogger.isErrorEnabled();
  }

  @Override
  public boolean isInfoEnabled() {
    return this.slf4jLogger.isInfoEnabled();
  }

  @Override
  public boolean isTraceEnabled() {
    return this.slf4jLogger.isTraceEnabled();
  }

  @Override
  public boolean isWarnEnabled() {
    return this.slf4jLogger.isWarnEnabled();
  }

  @Override
  public void debug(String msg, boolean isMasked) {
    logMessage(LogLevel.DEBUG, msg, isMasked);
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
    logMessage(LogLevel.ERROR, msg, isMasked);
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
    logMessage(LogLevel.INFO, msg, isMasked);
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
  public void trace(String msg, boolean isMasked) {
    logMessage(LogLevel.TRACE, msg, isMasked);
  }

  @Override
  public void trace(String msg, Object... arguments) {
    logFormat(LogLevel.TRACE, msg, arguments);
  }

  @Override
  public void trace(String msg, Throwable t) {
    logThrowable(LogLevel.TRACE, msg, t);
  }

  @Override
  public void warn(String msg, boolean isMasked) {
    logMessage(LogLevel.WARN, msg, isMasked);
  }

  @Override
  public void warn(String msg, Object... arguments) {
    logFormat(LogLevel.WARN, msg, arguments);
  }

  @Override
  public void warn(String msg, Throwable t) {
    logThrowable(LogLevel.WARN, msg, t);
  }

  private void logMessage(LogLevel level, String msg, boolean isMasked) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      String message = isMasked ? SecretDetector.maskSecrets(msg) : msg;
      logToSlf4j(level, message, null);
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  private void logFormat(LogLevel level, String msg, Object... arguments) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      FormattingTuple ft = MessageFormatter.arrayFormat(msg, evaluateLambdaArgs(arguments));
      String message = SecretDetector.maskSecrets(ft.getMessage());
      Throwable masked = ft.getThrowable() == null ? null : new MaskedException(ft.getThrowable());
      logToSlf4j(level, message, masked);
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  private void logThrowable(LogLevel level, String msg, Throwable t) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      String message = SecretDetector.maskSecrets(msg);
      Throwable masked = t == null ? null : new MaskedException(t);
      logToSlf4j(level, message, masked);
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  private boolean isLevelEnabled(LogLevel level) {
    if (level == null) {
      return false;
    }
    switch (level) {
      case ERROR:
        return slf4jLogger.isErrorEnabled();
      case WARN:
        return slf4jLogger.isWarnEnabled();
      case INFO:
        return slf4jLogger.isInfoEnabled();
      case DEBUG:
        return slf4jLogger.isDebugEnabled();
      case TRACE:
        return slf4jLogger.isTraceEnabled();
      case OFF:
      default:
        return false;
    }
  }

  private void logToSlf4j(LogLevel level, String message, Throwable throwable) {
    if (level == LogLevel.OFF) {
      return;
    }

    if (isLocationAwareLogger) {
      ((LocationAwareLogger) slf4jLogger)
          .log(null, FQCN, toLocationAwareLevel(level), message, null, throwable);
      return;
    }
    switch (level) {
      case ERROR:
        if (throwable == null) {
          slf4jLogger.error(message);
        } else {
          slf4jLogger.error(message, throwable);
        }
        break;
      case WARN:
        if (throwable == null) {
          slf4jLogger.warn(message);
        } else {
          slf4jLogger.warn(message, throwable);
        }
        break;
      case INFO:
        if (throwable == null) {
          slf4jLogger.info(message);
        } else {
          slf4jLogger.info(message, throwable);
        }
        break;
      case DEBUG:
        if (throwable == null) {
          slf4jLogger.debug(message);
        } else {
          slf4jLogger.debug(message, throwable);
        }
        break;
      case TRACE:
        if (throwable == null) {
          slf4jLogger.trace(message);
        } else {
          slf4jLogger.trace(message, throwable);
        }
        break;
      case OFF:
        return;
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
  }

  private static int toLocationAwareLevel(LogLevel level) {
    if (level == null) {
      throw new IllegalArgumentException("Log level must not be null");
    }
    switch (level) {
      case ERROR:
        return LocationAwareLogger.ERROR_INT;
      case WARN:
        return LocationAwareLogger.WARN_INT;
      case INFO:
        return LocationAwareLogger.INFO_INT;
      case DEBUG:
        return LocationAwareLogger.DEBUG_INT;
      case TRACE:
        return LocationAwareLogger.TRACE_INT;
      case OFF:
        throw new IllegalArgumentException("OFF level must not be emitted");
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
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

  private static Class<?> requireLoggerClass(Class<?> clazz) {
    if (clazz == null) {
      throw new IllegalArgumentException("Logger class must not be null");
    }
    return clazz;
  }

  private static String requireLoggerName(String name) {
    if (name == null) {
      throw new IllegalArgumentException("Logger name must not be null");
    }
    return name;
  }

  private enum LogLevel {
    OFF,
    ERROR,
    WARN,
    INFO,
    DEBUG,
    TRACE
  }
}
