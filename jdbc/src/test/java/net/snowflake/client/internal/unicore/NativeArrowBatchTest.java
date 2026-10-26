package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import org.junit.jupiter.api.Test;

class NativeArrowBatchTest {

  @Test
  void shouldSetNativeByteOrderOnDirectBuffers() {
    ByteBuffer values = ByteBuffer.allocateDirect(4);
    Object column = new Object[] {null, null, values, new Object[0]};
    NativeArrowBatch batch = NativeArrowBatch.fromNative(new Object[] {1, new Object[] {column}});

    NativeArrowBatch.NativeArrowArray exported = batch.getColumns()[0];
    assertNull(exported.getValidity());
    assertNull(exported.getOffsets());
    assertSame(values, exported.getValues());
    assertEquals(ByteOrder.nativeOrder(), exported.getValues().order());
  }

  @Test
  void shouldViewNativeBuffersInPlace() {
    ByteBuffer values = ByteBuffer.allocateDirect(4).order(ByteOrder.LITTLE_ENDIAN);
    values.putInt(0, 42);
    Object raw =
        new Object[] {
          Integer.valueOf(1), new Object[] {new Object[] {null, null, values, new Object[0]}}
        };

    NativeArrowBatch batch = NativeArrowBatch.fromNative(raw);

    ByteBuffer decoded = batch.getColumns()[0].getValues();
    assertTrue(decoded.isDirect());
    assertSame(values, decoded);
    assertEquals(42, decoded.order(ByteOrder.LITTLE_ENDIAN).getInt(0));
  }

  @Test
  void shouldRejectAHeapCopy() {
    Object raw =
        new Object[] {
          Integer.valueOf(1),
          new Object[] {new Object[] {null, null, new byte[] {42, 0, 0, 0}, new Object[0]}}
        };

    IllegalStateException failure =
        assertThrows(IllegalStateException.class, () -> NativeArrowBatch.fromNative(raw));

    assertEquals("Native Arrow buffer must be a ByteBuffer", failure.getMessage());
  }

  @Test
  void shouldRejectAHeapByteBuffer() {
    ByteBuffer values = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN);
    values.putInt(0, 42);
    Object raw =
        new Object[] {
          Integer.valueOf(1), new Object[] {new Object[] {null, null, values, new Object[0]}}
        };

    IllegalStateException failure =
        assertThrows(IllegalStateException.class, () -> NativeArrowBatch.fromNative(raw));

    assertEquals("Native Arrow buffer must be direct", failure.getMessage());
  }
}
