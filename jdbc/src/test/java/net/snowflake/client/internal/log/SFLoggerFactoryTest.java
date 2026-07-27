package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

class SFLoggerFactoryTest {

  @AfterEach
  void tearDown() {
    SFLoggerFactory.resetLoggerImplementationForTests();
    System.clearProperty("net.snowflake.jdbc.loggerImpl");
  }

  @Test
  void shouldReturnCoreLoggerFromGetLoggerByName() {
    SFLogger sflogger = SFLoggerFactory.getLogger("SFLoggerFactoryTest");
    assertInstanceOf(CoreLogger.class, sflogger);
  }

  @Test
  void shouldReturnCoreLoggerFromGetLoggerByClass() {
    SFLogger sflogger = SFLoggerFactory.getLogger(SFLoggerFactoryTest.class);
    assertInstanceOf(CoreLogger.class, sflogger);
  }

  @Test
  void shouldReturnJdk14LoggerFromGetDeliveryLoggerByDefault() {
    SFLogger sflogger = SFLoggerFactory.getDeliveryLogger("SFLoggerFactoryTest");
    assertInstanceOf(JDK14Logger.class, sflogger);
  }

  @Test
  void shouldReturnSlf4jLoggerFromGetDeliveryLoggerWhenConfigured() {
    System.setProperty("net.snowflake.jdbc.loggerImpl", "net.snowflake.client.log.SLF4JLogger");
    SFLoggerFactory.resetLoggerImplementationForTests();

    SFLogger sflogger = SFLoggerFactory.getDeliveryLogger("SFLoggerFactoryTest");
    assertInstanceOf(SLF4JLogger.class, sflogger);
  }

  @Test
  void shouldReportJulAsDefaultLoggerImplementationName() {
    assertEquals("JUL", SFLoggerFactory.getLoggerImplementationName());
  }

  @Test
  void shouldReportSlf4jWhenLoggerImplPropertySet() {
    System.setProperty("net.snowflake.jdbc.loggerImpl", "net.snowflake.client.log.SLF4JLogger");
    SFLoggerFactory.resetLoggerImplementationForTests();

    assertEquals("SLF4J", SFLoggerFactory.getLoggerImplementationName());
  }
}
