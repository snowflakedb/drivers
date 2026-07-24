package net.snowflake.client.internal.log;

import lombok.AccessLevel;
import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.RequiredArgsConstructor;

/** Used to create {@link SFLogger} instances. */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class SFLoggerFactory {

  private static final String LOGGER_IMPL_PROPERTY = "net.snowflake.jdbc.loggerImpl";

  private static LoggerImpl loggerImplementation;

  @Getter
  @RequiredArgsConstructor
  enum LoggerImpl {
    SLF4JLOGGER("net.snowflake.client.log.SLF4JLogger", "SLF4J"),
    JDK14LOGGER("net.snowflake.client.log.JDK14Logger", "JUL");

    private final String loggerImplClassName;
    private final String implementationName;

    static LoggerImpl fromString(String loggerImplClassName) {
      if (loggerImplClassName == null) {
        return null;
      }
      for (LoggerImpl impl : values()) {
        if (loggerImplClassName.equalsIgnoreCase(impl.loggerImplClassName)) {
          return impl;
        }
      }
      return null;
    }

    SFLogger createDeliveryLogger(String name) {
      switch (this) {
        case SLF4JLOGGER:
          return new SLF4JLogger(name);
        case JDK14LOGGER:
        default:
          return new JDK14Logger(name);
      }
    }
  }

  public static SFLogger getLogger(Class<?> clazz) {
    return new CoreLogger(clazz);
  }

  public static SFLogger getLogger(String name) {
    return new CoreLogger(name);
  }

  /** Plain delivery logger; must not be a {@link CoreLogger} or records loop. */
  public static SFLogger getDeliveryLogger(String name) {
    return createDeliveryLogger(name);
  }

  public static String getLoggerImplementationName() {
    return ensureLoggerImplementationResolved().implementationName;
  }

  static SFLogger createDeliveryLogger(String name) {
    return ensureLoggerImplementationResolved().createDeliveryLogger(name);
  }

  static void resetLoggerImplementationForTests() {
    loggerImplementation = null;
  }

  private static LoggerImpl ensureLoggerImplementationResolved() {
    if (loggerImplementation == null) {
      LoggerImpl resolved = LoggerImpl.fromString(System.getProperty(LOGGER_IMPL_PROPERTY));
      loggerImplementation = resolved != null ? resolved : LoggerImpl.JDK14LOGGER;
    }
    return loggerImplementation;
  }
}
