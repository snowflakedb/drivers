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

  @Test
  void shouldConvertNestedChildArraysAndPreserveNativeByteOrder() {
    ByteBuffer parentValidity = ByteBuffer.allocateDirect(1);
    ByteBuffer childValues = ByteBuffer.allocateDirect(8);
    Object child = new Object[] {null, null, childValues, new Object[0]};
    Object parent = new Object[] {parentValidity, null, null, new Object[] {child}};
    NativeArrowBatch batch = NativeArrowBatch.fromNative(new Object[] {2, new Object[] {parent}});

    assertEquals(2, batch.getRowCount());
    NativeArrowBatch.NativeArrowArray parentArray = batch.getColumns()[0];
    assertSame(parentValidity, parentArray.getValidity());
    assertEquals(ByteOrder.nativeOrder(), parentArray.getValidity().order());
    assertEquals(1, parentArray.getChildren().length);

    NativeArrowBatch.NativeArrowArray childArray = parentArray.getChildren()[0];
    assertSame(childValues, childArray.getValues());
    assertEquals(ByteOrder.nativeOrder(), childArray.getValues().order());
  }

  @Test
  void shouldRejectMalformedNativeBatchShapes() {
    assertThrows(IllegalStateException.class, () -> NativeArrowBatch.fromNative("not an array"));
    assertThrows(IllegalStateException.class, () -> NativeArrowBatch.fromNative(new Object[] {1}));
    assertThrows(
        IllegalStateException.class,
        () -> NativeArrowBatch.fromNative(new Object[] {"not an int", new Object[0]}));
    assertThrows(
        IllegalStateException.class,
        () -> NativeArrowBatch.fromNative(new Object[] {1, "not an Object[]"}));
  }

  @Test
  void shouldRejectMalformedNativeColumnAndNonDirectBuffers() {
    assertThrows(
        IllegalStateException.class,
        () -> NativeArrowBatch.fromNative(new Object[] {1, new Object[] {"not an array"}}));
    assertThrows(
        IllegalStateException.class,
        () ->
            NativeArrowBatch.fromNative(
                new Object[] {1, new Object[] {new Object[] {null, null, null}}}));
    assertThrows(
        IllegalStateException.class,
        () ->
            NativeArrowBatch.fromNative(
                new Object[] {
                  1, new Object[] {new Object[] {null, null, "not a buffer", new Object[0]}}
                }));
    assertThrows(
        IllegalStateException.class,
        () ->
            NativeArrowBatch.fromNative(
                new Object[] {
                  1, new Object[] {new Object[] {null, null, ByteBuffer.allocate(4), new Object[0]}}
                }));
  }
}
