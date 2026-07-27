package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Paths;
import java.util.logging.Level;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

class JDK14LoggerTest {

  @AfterEach
  void tearDown() {
    JDK14Logger.resetLoggerInitForTests();
    System.clearProperty("snowflake.jdbc.log.size");
    System.clearProperty("snowflake.jdbc.log.count");
    System.clearProperty("net.snowflake.jdbc.loggerImpl");
  }

  @Test
  void shouldEnableDebugAfterInstantiateLogger() throws IOException {
    System.setProperty("snowflake.jdbc.log.size", "100000");
    System.setProperty("snowflake.jdbc.log.count", "3");
    System.setProperty("net.snowflake.jdbc.loggerImpl", "net.snowflake.client.log.JDK14Logger");

    JDK14Logger logger = new JDK14Logger(JDK14LoggerTest.class.getName());
    assertFalse(logger.isDebugEnabled());
    assertTrue(logger.isInfoEnabled());

    Level tracingLevel = Level.parse("ALL");
    String logOutputPath =
        Paths.get(System.getProperty("java.io.tmpdir"), "snowflake_jdbc_ud.log").toString();
    JDK14Logger.instantiateLogger(tracingLevel, logOutputPath);
    assertTrue(logger.isDebugEnabled());
  }
}
