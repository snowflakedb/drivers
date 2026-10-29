package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;

public class NativeBufferTest {

  @Test
  public void shouldThrowCarrierWhenAddressIsReadAfterClose() {
    try (NativeBuffer buffer = NativeBuffer.copyOf(new byte[] {1, 2, 3})) {
      assertNotEquals(0L, buffer.address());
      buffer.close();
      SFSQLException error = assertThrows(SFSQLException.class, buffer::address);
      assertEquals("Native buffer is closed", error.getMessage());
    }
  }

  @Test
  public void shouldTrackAllocatedBytesAcrossCopyAndClose() {
    long baseline = NativeBuffer.allocatedBytes();
    try (NativeBuffer buffer = NativeBuffer.copyOf(new byte[] {1, 2, 3, 4})) {
      assertEquals(baseline + 4, NativeBuffer.allocatedBytes());
      buffer.close();
      assertEquals(baseline, NativeBuffer.allocatedBytes());
    }
    assertEquals(baseline, NativeBuffer.allocatedBytes());
  }

  @Test
  public void shouldAllowClosingTwiceWithoutRaising() {
    long baseline = NativeBuffer.allocatedBytes();
    try (NativeBuffer buffer = NativeBuffer.copyOf(new byte[] {5})) {
      buffer.close();
      buffer.close();
      assertEquals(baseline, NativeBuffer.allocatedBytes());
      assertThrows(SFSQLException.class, buffer::address);
    }
    assertEquals(baseline, NativeBuffer.allocatedBytes());
  }

  @Test
  public void shouldAllocateZeroLengthPayload() {
    long baseline = NativeBuffer.allocatedBytes();
    try (NativeBuffer buffer = NativeBuffer.copyOf(new byte[0])) {
      assertNotEquals(0L, buffer.address());
      assertEquals(baseline, NativeBuffer.allocatedBytes());
    }
    assertEquals(baseline, NativeBuffer.allocatedBytes());
  }

  @Test
  public void shouldRejectNullPayloadWithoutLeaking() {
    long baseline = NativeBuffer.allocatedBytes();
    assertThrows(SFSQLException.class, () -> NativeBuffer.copyOf(null));
    assertEquals(baseline, NativeBuffer.allocatedBytes());
  }

  @Test
  public void shouldKeepLiveBuffersIndependentWhenOneIsClosed() {
    long baseline = NativeBuffer.allocatedBytes();
    try (NativeBuffer first = NativeBuffer.copyOf(new byte[] {1, 2});
        NativeBuffer second = NativeBuffer.copyOf(new byte[] {3, 4, 5})) {
      assertNotEquals(first.address(), second.address());
      assertEquals(baseline + 5, NativeBuffer.allocatedBytes());

      long secondAddress = second.address();
      first.close();

      assertEquals(baseline + 3, NativeBuffer.allocatedBytes());
      assertEquals(secondAddress, second.address());
      assertThrows(SFSQLException.class, first::address);
    }
    assertEquals(baseline, NativeBuffer.allocatedBytes());
  }
}
