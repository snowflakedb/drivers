package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicReference;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.unicore.NativeArrowStream;
import org.junit.jupiter.api.Test;

/**
 * Pins the contract between {@link NativeBatchReader} and the native stream: one column tree is
 * rebound per batch, the previous batch's buffers are unreferenced before the native side is asked
 * for the next one (it frees them there), and the converters built against that stable tree are
 * reused for the whole result set.
 */
class NativeBatchLifetimeTest {

  private static final int I32 = 4;

  /** Values slot of the single int32 node, matching {@code Column.BUFFERS_PER_NODE} ordering. */
  private static final int VALUES_SLOT = 2;

  @Test
  void shouldReuseRootAndConvertersAcrossBatches() throws Exception {
    assertLifetimeAcross(Arrays.asList(new int[] {1}, new int[] {2}), false);
  }

  @Test
  void shouldReuseRootAndConvertersAcrossEmptyBatch() throws Exception {
    assertLifetimeAcross(Arrays.asList(new int[] {1}, new int[] {}, new int[] {2}), true);
  }

  @Test
  void shouldClearPreviousBatchBeforeExhaustion() throws Exception {
    RecordingBatchSource source = new RecordingBatchSource(Arrays.asList(new int[] {7}));
    AtomicReference<ColumnBatch> previous = new AtomicReference<ColumnBatch>();
    source.setPreviousBatch(previous);

    ArrowResources resources = new ArrowResources(new NativeBatchReader(source));
    CursorState cursor = new CursorState();
    ArrowBatchManager manager = new ArrowBatchManager(cursor, resources);

    assertTrue(manager.fetchNextRow());
    previous.set(resources.getCurrentRoot());
    assertNotNull(previous.get());

    assertFalse(manager.fetchNextRow());
    assertTrue(cursor.isAfterLast());
    assertTrue(source.observedClearedPreviousOnNextBatch());
  }

  @Test
  void shouldPropagateSourceFailureAndKeepBatchClearedWhenLoadingAfterClose() {
    ClosingBatchSource source = new ClosingBatchSource();
    NativeBatchReader reader = new NativeBatchReader(source);

    reader.close();

    SFSQLException failure = assertThrows(SFSQLException.class, reader::loadNextBatch);
    assertEquals("Native Arrow stream is closed", failure.getMessage());
    assertEquals(0, reader.getBatch().getRowCount());
  }

  private static void assertLifetimeAcross(List<int[]> batches, boolean expectEmptySkip)
      throws Exception {
    RecordingBatchSource source = new RecordingBatchSource(batches);
    AtomicReference<ColumnBatch> previous = new AtomicReference<ColumnBatch>();
    source.setPreviousBatch(previous);

    ArrowResources resources = new ArrowResources(new NativeBatchReader(source));
    CursorState cursor = new CursorState();
    SchemaState schema = new SchemaState(resources.getActiveRoot());
    ArrowBatchManager manager = new ArrowBatchManager(cursor, resources);

    assertTrue(manager.fetchNextRow());
    ColumnBatch firstRoot = resources.getCurrentRoot();
    previous.set(firstRoot);
    ArrowVectorConverter firstConverter = schema.getConverter(1, firstRoot);
    assertTrue(schema.hasCachedConverter(1));
    assertEquals(1, firstRoot.column(0).getInt(0));

    assertTrue(manager.fetchNextRow());
    ColumnBatch secondRoot = resources.getCurrentRoot();
    // The tree is rebound, not rebuilt, so both the root and its converters are reused.
    assertSame(firstRoot, secondRoot);
    assertTrue(schema.hasCachedConverter(1));
    assertSame(firstConverter, schema.getConverter(1, secondRoot));
    assertEquals(2, secondRoot.column(0).getInt(0));

    assertTrue(source.observedClearedPreviousOnNextBatch());
    if (expectEmptySkip) {
      assertTrue(source.nextBatchCallCount() >= 3);
    }
    assertFalse(manager.fetchNextRow());
    assertTrue(cursor.isAfterLast());
  }

  /** One direct int32 values buffer, as {@code jdbc_bridge} would hand it over. */
  private static ByteBuffer int32Values(int[] values) {
    ByteBuffer data =
        ByteBuffer.allocateDirect(values.length * Integer.BYTES).order(ByteOrder.LITTLE_ENDIAN);
    for (int index = 0; index < values.length; index++) {
      data.putInt(index * Integer.BYTES, values[index]);
    }
    return data;
  }

  private static byte[] fixedInt32Schema() {
    ByteBuffer encoded = ByteBuffer.allocate(128).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(1);
    writeString(encoded, "col");
    encoded.putInt(3);
    writeString(encoded, "logicalType");
    writeString(encoded, "FIXED");
    writeString(encoded, "precision");
    writeString(encoded, "10");
    writeString(encoded, "scale");
    writeString(encoded, "0");
    encoded.put((byte) 1);
    encoded.put((byte) I32);
    byte[] bytes = new byte[encoded.position()];
    encoded.flip();
    encoded.get(bytes);
    return bytes;
  }

  private static void writeString(ByteBuffer encoded, String value) {
    byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
    encoded.putInt(bytes.length);
    encoded.put(bytes);
  }

  /**
   * Stands in for the native stream. Every {@code nextBatch} asserts the reader already dropped the
   * previous binding, which is what makes freeing the batch behind it safe.
   */
  private static final class RecordingBatchSource implements NativeBatchSource {
    private final byte[] schema = fixedInt32Schema();
    private final List<int[]> batches;
    private int next;
    private AtomicReference<ColumnBatch> previousBatch;
    private boolean observedClearedPrevious;
    private int nextBatchCalls;

    private RecordingBatchSource(List<int[]> batches) {
      this.batches = new ArrayList<int[]>(batches);
    }

    private void setPreviousBatch(AtomicReference<ColumnBatch> previousBatch) {
      this.previousBatch = previousBatch;
    }

    private boolean observedClearedPreviousOnNextBatch() {
      return observedClearedPrevious;
    }

    private int nextBatchCallCount() {
      return nextBatchCalls;
    }

    @Override
    public byte[] schema() {
      return schema;
    }

    @Override
    public int nextBatch(ByteBuffer[] slots) {
      nextBatchCalls++;
      ColumnBatch previous = previousBatch == null ? null : previousBatch.get();
      if (previous != null) {
        Column column = previous.column(0);
        assertEquals(0, previous.getRowCount());
        assertTrue(column.isNull(0));
        assertThrows(IndexOutOfBoundsException.class, () -> column.getInt(0));
        observedClearedPrevious = true;
      }
      Arrays.fill(slots, null);
      if (next >= batches.size()) {
        return NativeArrowStream.END_OF_STREAM;
      }
      int[] values = batches.get(next++);
      slots[VALUES_SLOT] = int32Values(values);
      return values.length;
    }

    @Override
    public void close() {}
  }

  private static final class ClosingBatchSource implements NativeBatchSource {
    private final byte[] schema = fixedInt32Schema();
    private boolean closed;

    @Override
    public byte[] schema() {
      return schema;
    }

    @Override
    public int nextBatch(ByteBuffer[] slots) {
      if (closed) {
        throw new SFSQLException(ErrorCode.INTERNAL_ERROR, "Native Arrow stream is closed");
      }
      Arrays.fill(slots, null);
      return NativeArrowStream.END_OF_STREAM;
    }

    @Override
    public void close() {
      closed = true;
    }
  }
}
