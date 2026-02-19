package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import org.junit.jupiter.api.Test;

public class SFLoggerFactoryTest {
  @Test
  public void testGetLoggerByNameDefault() {
    SFLogger sflogger = SFLoggerFactory.getLogger("SFLoggerFactoryTest");
    assertInstanceOf(SLF4JLogger.class, sflogger);
  }

  @Test
  public void testGetLoggerByClassDefault() {
    SFLogger sflogger = SFLoggerFactory.getLogger(SFLoggerFactoryTest.class);
    assertInstanceOf(SLF4JLogger.class, sflogger);
  }
}
