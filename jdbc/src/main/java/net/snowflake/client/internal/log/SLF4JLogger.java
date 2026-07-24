package net.snowflake.client.internal.log;

import org.slf4j.spi.LocationAwareLogger;

public class SLF4JLogger extends AbstractDeliveryLogger {
  private static final String FQCN = SLF4JLogger.class.getName();

  private final org.slf4j.Logger slf4jLogger;
  private final boolean locationAware;

  public SLF4JLogger(Class<?> clazz) {
    this(org.slf4j.LoggerFactory.getLogger(requireLoggerClass(clazz)));
  }

  public SLF4JLogger(String name) {
    this(org.slf4j.LoggerFactory.getLogger(requireLoggerName(name)));
  }

  SLF4JLogger(org.slf4j.Logger slf4jLogger) {
    this.slf4jLogger = slf4jLogger;
    this.locationAware = slf4jLogger instanceof LocationAwareLogger;
  }

  @Override
  protected boolean isLevelEnabled(LogLevel level) {
    switch (level) {
      case ERROR:
        return slf4jLogger.isErrorEnabled();
      case WARN:
        return slf4jLogger.isWarnEnabled();
      case INFO:
        return slf4jLogger.isInfoEnabled();
      case DEBUG:
        return slf4jLogger.isDebugEnabled();
      default:
        return false;
    }
  }

  @Override
  protected void deliver(LogLevel level, String message, Throwable throwable) {
    if (locationAware) {
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
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
  }

  private static int toLocationAwareLevel(LogLevel level) {
    switch (level) {
      case ERROR:
        return LocationAwareLogger.ERROR_INT;
      case WARN:
        return LocationAwareLogger.WARN_INT;
      case INFO:
        return LocationAwareLogger.INFO_INT;
      case DEBUG:
        return LocationAwareLogger.DEBUG_INT;
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
}
