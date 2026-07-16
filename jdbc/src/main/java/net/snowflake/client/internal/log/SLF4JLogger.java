package net.snowflake.client.internal.log;

import net.snowflake.client.internal.util.MaskedException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
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
      String message = isMasked ? LogFormatter.mask(msg) : msg;
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
      LogFormatter.Formatted formatted = LogFormatter.format(msg, arguments);
      logToSlf4j(level, formatted.getMessage(), formatted.getThrowable());
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  private void logThrowable(LogLevel level, String msg, Throwable t) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      String message = LogFormatter.mask(msg);
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
      case OFF:
        throw new IllegalArgumentException("OFF level must not be emitted");
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
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
    DEBUG
  }
}
