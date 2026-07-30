package net.snowflake.client.internal.log;

import java.io.IOException;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.Set;
import java.util.logging.ConsoleHandler;
import java.util.logging.FileHandler;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.Logger;
import lombok.RequiredArgsConstructor;

/**
 * {@link SFLogger} backed by {@link java.util.logging} (JUL).
 *
 * <p>Log level mapping: ERROR→SEVERE, WARN→WARNING, INFO→INFO, DEBUG→FINE.
 */
@RequiredArgsConstructor
public class JDK14Logger extends AbstractDeliveryLogger {

  public static final String STDOUT = "STDOUT";

  private static final String JAVA_LOGGING_CONSOLE_STD_OUT = "JAVA_LOGGING_CONSOLE_STD_OUT";
  private static final String JAVA_LOGGING_CONSOLE_STD_OUT_THRESHOLD =
      "JAVA_LOGGING_CONSOLE_STD_OUT_THRESHOLD";
  private static final Set<String> LOG_METHODS =
      Collections.unmodifiableSet(
          new HashSet<>(
              Arrays.asList("debug", "error", "info", "trace", "warn", "logPlain", "deliver")));

  private static final StdOutConsoleHandler STD_OUT_CONSOLE_HANDLER = new StdOutConsoleHandler();
  private static boolean isLoggerInit = false;

  private final Logger jdkLogger;

  static {
    if ("true".equalsIgnoreCase(System.getProperty(JAVA_LOGGING_CONSOLE_STD_OUT))) {
      useStdOutConsoleHandler(System.getProperty(JAVA_LOGGING_CONSOLE_STD_OUT_THRESHOLD));
    }
  }

  public JDK14Logger(String name) {
    this(Logger.getLogger(name));
  }

  public static void useStdOutConsoleHandler(String threshold) {
    Level thresholdLevel = threshold != null ? parseLevel(threshold) : null;
    Logger rootLogger = Logger.getLogger("");
    for (Handler handler : rootLogger.getHandlers()) {
      if (handler instanceof ConsoleHandler) {
        rootLogger.removeHandler(handler);
        rootLogger.addHandler(
            thresholdLevel != null
                ? new StdErrOutThresholdAwareConsoleHandler(thresholdLevel)
                : STD_OUT_CONSOLE_HANDLER);
        break;
      }
    }
  }

  static void resetToDefaultConsoleHandler() {
    Logger rootLogger = Logger.getLogger("");
    for (Handler handler : rootLogger.getHandlers()) {
      if (handler instanceof StdErrOutThresholdAwareConsoleHandler
          || handler instanceof StdOutConsoleHandler) {
        rootLogger.removeHandler(handler);
        rootLogger.addHandler(new ConsoleHandler());
        break;
      }
    }
  }

  public static void addHandler(Handler handler) {
    clientLogger().addHandler(handler);
  }

  public static void removeHandler(Handler handler) {
    clientLogger().removeHandler(handler);
  }

  public static void setUseParentHandlers(boolean value) {
    clientLogger().setUseParentHandlers(value);
  }

  public static void setLevel(Level level) {
    clientLogger().setLevel(level);
  }

  public static Level getLevel() {
    return clientLogger().getLevel();
  }

  public static synchronized void instantiateLogger(Level level, String logPath)
      throws IOException {
    if (!isLoggerInit) {
      loggerInit(level, logPath);
      isLoggerInit = true;
    }
  }

  static synchronized void resetLoggerInitForTests() {
    isLoggerInit = false;
    resetLogger(clientLogger());
    resetLogger(Logger.getLogger(SFFormatter.INFORMATICA_V1_CLASS_NAME_PREFIX));
  }

  private static void resetLogger(Logger logger) {
    for (Handler handler : logger.getHandlers()) {
      logger.removeHandler(handler);
    }
    logger.setLevel(null);
  }

  @Override
  protected boolean isLevelEnabled(LogLevel level) {
    return jdkLogger.isLoggable(toJulLevel(level));
  }

  @Override
  protected void deliver(LogLevel level, String message, Throwable throwable) {
    String[] source = findSourceInStack();
    jdkLogger.logp(toJulLevel(level), source[0], source[1], message, throwable);
  }

  private static Level toJulLevel(LogLevel level) {
    switch (level) {
      case ERROR:
        return Level.SEVERE;
      case WARN:
        return Level.WARNING;
      case INFO:
        return Level.INFO;
      case DEBUG:
        return Level.FINE;
      default:
        throw new IllegalArgumentException("Unsupported log level: " + level);
    }
  }

  private static Level parseLevel(String threshold) {
    try {
      return Level.parse(threshold);
    } catch (Exception e) {
      throw new UnknownJavaUtilLoggingLevelException(threshold);
    }
  }

  private static Logger clientLogger() {
    return Logger.getLogger(SFFormatter.CLASS_NAME_PREFIX);
  }

  private String[] findSourceInStack() {
    StackTraceElement[] stackTraces = Thread.currentThread().getStackTrace();
    String[] results = new String[2];
    for (int i = 0; i < stackTraces.length; i++) {
      if (LOG_METHODS.contains(stackTraces[i].getMethodName())) {
        for (int j = i; j < stackTraces.length; j++) {
          if (!LOG_METHODS.contains(stackTraces[j].getMethodName())) {
            results[0] = stackTraces[j].getClassName();
            results[1] = stackTraces[j].getMethodName();
            return results;
          }
        }
      }
    }
    return results;
  }

  private static void loggerInit(Level level, String outputPath) throws IOException {
    Logger informaticaLogger = Logger.getLogger(SFFormatter.INFORMATICA_V1_CLASS_NAME_PREFIX);
    informaticaLogger.setLevel(level);
    setLevel(level);

    Handler handler =
        STDOUT.equalsIgnoreCase(outputPath)
            ? consoleHandler(level)
            : fileHandler(level, outputPath);
    addHandler(handler);
    informaticaLogger.addHandler(handler);
  }

  private static Handler consoleHandler(Level level) {
    ConsoleHandler consoleHandler = new ConsoleHandler();
    consoleHandler.setLevel(level);
    consoleHandler.setFormatter(new SFFormatter());
    return consoleHandler;
  }

  private static Handler fileHandler(Level level, String outputPath) throws IOException {
    FileHandler fileHandler =
        new FileHandler(
            outputPath,
            readIntProperty("snowflake.jdbc.log.size", 1_000_000_000),
            readIntProperty("snowflake.jdbc.log.count", 2),
            true);
    fileHandler.setFormatter(new SFFormatter());
    fileHandler.setLevel(level);
    return fileHandler;
  }

  private static int readIntProperty(String key, int defaultValue) {
    String raw = System.getProperty(key);
    if (raw == null) {
      return defaultValue;
    }
    try {
      return Integer.parseInt(raw);
    } catch (NumberFormatException ignored) {
      return defaultValue;
    }
  }
}
