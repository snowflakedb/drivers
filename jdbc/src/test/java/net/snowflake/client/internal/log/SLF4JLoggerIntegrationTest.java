package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

public class SLF4JLoggerIntegrationTest {
  @Test
  public void testNullLoggerNameThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((String) null));
  }

  @Test
  public void testNullLoggerClassThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((Class<?>) null));
  }
}
