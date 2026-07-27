package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.util.Properties;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class Jdk14LoggerBootstrapTest {

  private static final String URL =
      "jdbc:snowflake://account.snowflakecomputing.com/?user=u&tracing=ALL";

  @BeforeEach
  @AfterEach
  void resetLoggingState() {
    JDK14Logger.resetLoggerInitForTests();
    SFLoggerFactory.resetLoggerImplementationForTests();
    System.clearProperty("net.snowflake.jdbc.loggerImpl");
    System.clearProperty("java.util.logging.config.file");
  }

  @Test
  void shouldDoNothingWhenTracingIsAbsent() throws IOException {
    Jdk14LoggerBootstrap.initFromConnectionIfConfigured(
        "jdbc:snowflake://account.snowflakecomputing.com/?user=u", new Properties());

    JDK14Logger logger = new JDK14Logger(Jdk14LoggerBootstrapTest.class.getName());
    assertFalse(logger.isDebugEnabled());
  }

  @Test
  void shouldInstantiateLoggerWhenTracingIsSet() throws IOException {
    System.setProperty("net.snowflake.jdbc.loggerImpl", "net.snowflake.client.log.JDK14Logger");
    SFLoggerFactory.resetLoggerImplementationForTests();

    Properties info = new Properties();
    info.setProperty("tracing", "ALL");

    Jdk14LoggerBootstrap.initFromConnectionIfConfigured(URL, info);

    JDK14Logger logger = new JDK14Logger(Jdk14LoggerBootstrapTest.class.getName());
    assertTrue(logger.isDebugEnabled());
  }

  @Test
  void shouldSkipWhenExternalJulConfigFileIsSet() throws IOException {
    System.setProperty("java.util.logging.config.file", "/tmp/logging.properties");

    Jdk14LoggerBootstrap.initFromConnectionIfConfigured(URL, new Properties());

    JDK14Logger logger = new JDK14Logger(Jdk14LoggerBootstrapTest.class.getName());
    assertFalse(logger.isDebugEnabled());
  }

  @Test
  void shouldSkipWhenSlf4jDeliveryBackendIsConfigured() throws IOException {
    System.setProperty("net.snowflake.jdbc.loggerImpl", "net.snowflake.client.log.SLF4JLogger");
    SFLoggerFactory.resetLoggerImplementationForTests();

    Jdk14LoggerBootstrap.initFromConnectionIfConfigured(URL, new Properties());

    JDK14Logger logger = new JDK14Logger(Jdk14LoggerBootstrapTest.class.getName());
    assertFalse(logger.isDebugEnabled());
  }
}
