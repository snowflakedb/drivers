package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import org.junit.jupiter.api.Test;

public class SFLoggerFactoryTest {
  @Test
  public void shouldReturnCoreLoggerFromGetLoggerByName() {
    SFLogger sflogger = SFLoggerFactory.getLogger("SFLoggerFactoryTest");
    assertInstanceOf(CoreLogger.class, sflogger);
  }

  @Test
  public void shouldReturnCoreLoggerFromGetLoggerByClass() {
    SFLogger sflogger = SFLoggerFactory.getLogger(SFLoggerFactoryTest.class);
    assertInstanceOf(CoreLogger.class, sflogger);
  }

  @Test
  public void shouldReturnPlainSlf4jLoggerFromGetDeliveryLogger() {
    SFLogger sflogger = SFLoggerFactory.getDeliveryLogger("SFLoggerFactoryTest");
    assertInstanceOf(SLF4JLogger.class, sflogger);
  }
}
