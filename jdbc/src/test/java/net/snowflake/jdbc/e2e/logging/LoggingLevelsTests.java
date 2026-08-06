package net.snowflake.jdbc.e2e.logging;

import static java.util.logging.Level.INFO;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Properties;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.stream.Collectors;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;

/**
 * E2E coverage for independent wrapper vs core log levels. Captures via JUL because each {@code
 * CoreLogger} snapshots its delivery delegate at host-class load time - earlier tests in the same
 * JVM fork may have loaded driver classes under the default JUL backend before we could switch to
 * SLF4J, so level gates and capture must use JUL.
 */
@Isolated("mutates JUL logger levels")
@SkipOldDriver("Universal driver core logging bridge")
public class LoggingLevelsTests extends SnowflakeIntegrationTestBase {

  private static final String WRAPPER_LOGGER_PREFIX = "net.snowflake.client";
  private static final String CORE_LOGGER_NAME = "net.snowflake.client.CoreLogger";

  /** JUL mapping used by JDK14Logger: DEBUG → FINE. */
  private static final Level DEBUG = Level.FINE;

  private CapturingHandler logHandler;
  private Logger wrapperRootLogger;
  private Logger coreLogger;

  @BeforeEach
  void setUpLogCapture() {
    logHandler = new CapturingHandler();
    logHandler.setLevel(Level.ALL);

    wrapperRootLogger = Logger.getLogger(WRAPPER_LOGGER_PREFIX);
    coreLogger = Logger.getLogger(CORE_LOGGER_NAME);

    // Parent handler receives both wrapper children and CoreLogger (useParentHandlers=true).
    wrapperRootLogger.addHandler(logHandler);
  }

  @AfterEach
  void tearDownLogCapture() {
    wrapperRootLogger.removeHandler(logHandler);
    logHandler.close();
  }

  @Test
  public void shouldEmitInfoLogsAtDefaultLevels() throws Exception {
    // Given Default logging levels
    configureLogLevels(INFO, INFO);
    Properties props = queryTextLoggingProperties();

    try (Connection connection = openConnection(props);
        Statement statement = connection.createStatement()) {
      logHandler.clear();

      // When Query "SELECT 1 AS value" is executed
      try (ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
        assertTrue(resultSet.next(), "Expected one row");
      }

      // Then Core logger emits an INFO log
      assertTrue(hasLevel(coreRecords(), INFO), "Expected core INFO log during query execution");

      // And Wrapper logger emits an INFO log
      assertTrue(
          hasLevel(wrapperRecords(), INFO), "Expected wrapper INFO log during query execution");
    }
  }

  @Test
  public void shouldEmitCoreDebugWhenCoreLogLevelIsDebug() throws Exception {
    // Given Logging is configured with wrapper log level INFO and core log level DEBUG
    configureLogLevels(INFO, DEBUG);
    Properties props = queryTextLoggingProperties();

    try (Connection connection = openConnection(props);
        Statement statement = connection.createStatement()) {
      logHandler.clear();

      // When Query "SELECT 1 AS value" is executed
      try (ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
        assertTrue(resultSet.next(), "Expected one row");
      }

      // Then Core logger emits a DEBUG log
      assertTrue(hasLevel(coreRecords(), DEBUG), "Expected core DEBUG log during query execution");

      // And Wrapper logger does not emit a DEBUG log but emits INFO log
      List<LogRecord> wrapper = wrapperRecords();
      assertFalse(
          hasLevel(wrapper, DEBUG), "Wrapper should not emit DEBUG when wrapper log level is INFO");
      assertTrue(hasLevel(wrapper, INFO), "Expected wrapper INFO log during query execution");
    }
  }

  @Test
  public void shouldEmitWrapperDebugWithoutCoreDebugWhenWrapperLogLevelIsDebug() throws Exception {
    // Given Logging is configured with wrapper log level DEBUG and core log level INFO
    configureLogLevels(DEBUG, INFO);

    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {
      logHandler.clear();

      // When Query "SELECT 1 AS value" is executed
      try (ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
        assertTrue(resultSet.next(), "Expected one row");
      }

      // Then Wrapper logger emits a DEBUG log
      assertTrue(
          hasLevel(wrapperRecords(), DEBUG), "Expected wrapper DEBUG log during query execution");

      // And Core logger does not emit a DEBUG log but emits INFO log
      List<LogRecord> core = coreRecords();
      assertFalse(hasLevel(core, DEBUG), "Core should not emit DEBUG when core log level is INFO");
      assertTrue(hasLevel(core, INFO), "Expected core INFO log during query execution");
    }
  }

  @Test
  public void shouldEmitWrapperAndCoreDebugWhenBothLevelsAreDebug() throws Exception {
    // Given Logging is configured with wrapper log level DEBUG and core log level DEBUG
    configureLogLevels(DEBUG, DEBUG);

    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {
      logHandler.clear();

      // When Query "SELECT 1 AS value" is executed
      try (ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
        assertTrue(resultSet.next(), "Expected one row");
      }

      // Then Wrapper logger emits a DEBUG log
      assertTrue(
          hasLevel(wrapperRecords(), DEBUG), "Expected wrapper DEBUG log during query execution");

      // And Core logger emits a DEBUG log
      assertTrue(hasLevel(coreRecords(), DEBUG), "Expected core DEBUG log during query execution");
    }
  }

  private void configureLogLevels(Level wrapperLevel, Level coreLevel) {
    wrapperRootLogger.setLevel(wrapperLevel);
    coreLogger.setLevel(coreLevel);
  }

  private static Properties queryTextLoggingProperties() {
    Properties props = new Properties();
    props.setProperty("log_query_text", "true");
    return props;
  }

  private List<LogRecord> wrapperRecords() {
    return logHandler.records.stream()
        .filter(
            r ->
                r.getLoggerName() != null
                    && r.getLoggerName().startsWith(WRAPPER_LOGGER_PREFIX)
                    && !r.getLoggerName().startsWith(CORE_LOGGER_NAME))
        .collect(Collectors.toList());
  }

  private List<LogRecord> coreRecords() {
    return logHandler.records.stream()
        .filter(r -> r.getLoggerName() != null && r.getLoggerName().startsWith(CORE_LOGGER_NAME))
        .collect(Collectors.toList());
  }

  private static boolean hasLevel(List<LogRecord> records, Level level) {
    return records.stream().anyMatch(r -> r.getLevel().equals(level));
  }

  private static final class CapturingHandler extends Handler {
    private final List<LogRecord> records = Collections.synchronizedList(new ArrayList<>());

    @Override
    public void publish(LogRecord record) {
      if (isLoggable(record)) {
        records.add(record);
      }
    }

    @Override
    public void flush() {}

    @Override
    public void close() {
      records.clear();
    }

    void clear() {
      records.clear();
    }
  }
}
