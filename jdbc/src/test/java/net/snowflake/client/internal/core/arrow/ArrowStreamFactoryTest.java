package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import org.junit.jupiter.api.Test;

class ArrowStreamFactoryTest {

  @Test
  void shouldDecodeLittleEndianLong() {
    byte[] bytes =
        ByteBuffer.allocate(Long.BYTES)
            .order(ByteOrder.LITTLE_ENDIAN)
            .putLong(0x0123456789ABCDEFL)
            .array();

    assertEquals(0x0123456789ABCDEFL, ArrowStreamFactory.pointerFromBytes(bytes));
  }

  @Test
  void shouldRoundTripKnownValues() {
    long[] values = {0L, 1L, -1L, 42L, Long.MAX_VALUE, Long.MIN_VALUE};
    for (long value : values) {
      byte[] bytes =
          ByteBuffer.allocate(Long.BYTES).order(ByteOrder.LITTLE_ENDIAN).putLong(value).array();
      assertEquals(value, ArrowStreamFactory.pointerFromBytes(bytes));
    }
  }

  @Test
  void shouldInterpretBytesAsLittleEndian() {
    long value = 0x0102030405060708L;
    byte[] littleEndian =
        ByteBuffer.allocate(Long.BYTES).order(ByteOrder.LITTLE_ENDIAN).putLong(value).array();
    byte[] bigEndian =
        ByteBuffer.allocate(Long.BYTES).order(ByteOrder.BIG_ENDIAN).putLong(value).array();

    assertEquals(value, ArrowStreamFactory.pointerFromBytes(littleEndian));
    assertNotEquals(value, ArrowStreamFactory.pointerFromBytes(bigEndian));
  }
}
