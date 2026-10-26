package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicReference;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.unicore.NativeArrowBatch;
import net.snowflake.client.internal.unicore.NativeArrowBatch.NativeArrowArray;
import org.junit.jupiter.api.Test;

class NativeBatchLifetimeTest {

  private static final int I32 = 4;

  @Test
  void shouldClosePreviousBatchAndClearConvertersBeforeNextNonEmptyBatch() throws Exception {
    assertLifetimeAcross(Arrays.asList(intBatch(new int[] {1}), intBatch(new int[] {2})), false);
  }

  @Test
  void shouldClosePreviousBatchAndClearConvertersAcrossEmptyBatch() throws Exception {
    assertLifetimeAcross(
        Arrays.asList(intBatch(new int[] {1}), intBatch(new int[] {}), intBatch(new int[] {2})),
        true);
  }

  @Test
  void shouldClosePreviousBatchBeforeExhaustion() throws Exception {
    RecordingBatchSource source = new RecordingBatchSource(Arrays.asList(intBatch(new int[] {7})));
    AtomicReference<ColumnBatch> previous = new AtomicReference<ColumnBatch>();
    source.setPreviousBatch(previous);

    ArrowResources resources = new ArrowResources(new NativeBatchReader(source));
    CursorState cursor = new CursorState();
    SchemaState schema = new SchemaState(resources.getActiveRoot());
    source.setSchemaState(schema);
    ArrowBatchManager manager = new ArrowBatchManager(cursor, resources, schema);

    assertTrue(manager.fetchNextRow());
    previous.set(resources.getCurrentRoot());
    assertNotNull(previous.get());

    assertFalse(manager.fetchNextRow());
    assertTrue(cursor.isAfterLast());
    assertTrue(source.observedClosedPreviousOnNextBatch());
  }

  private static void assertLifetimeAcross(List<NativeArrowBatch> batches, boolean expectEmptySkip)
      throws Exception {
    RecordingBatchSource source = new RecordingBatchSource(batches);
    AtomicReference<ColumnBatch> previous = new AtomicReference<ColumnBatch>();
    source.setPreviousBatch(previous);

    ArrowResources resources = new ArrowResources(new NativeBatchReader(source));
    CursorState cursor = new CursorState();
    SchemaState schema = new SchemaState(resources.getActiveRoot());
    source.setSchemaState(schema);
    ArrowBatchManager manager = new ArrowBatchManager(cursor, resources, schema);

    assertTrue(manager.fetchNextRow());
    ColumnBatch firstRoot = resources.getCurrentRoot();
    previous.set(firstRoot);
    ArrowVectorConverter firstConverter = schema.getConverter(1, firstRoot);
    assertTrue(schema.hasCachedConverter(1));

    assertTrue(manager.fetchNextRow());
    ColumnBatch secondRoot = resources.getCurrentRoot();
    assertNotSame(firstRoot, secondRoot);
    ArrowVectorConverter secondConverter = schema.getConverter(1, secondRoot);
    assertNotSame(firstConverter, secondConverter);
    assertTrue(source.observedClosedPreviousOnNextBatch());
    if (expectEmptySkip) {
      assertTrue(source.nextBatchCallCount() >= 3);
    }
    assertFalse(manager.fetchNextRow());
    assertTrue(cursor.isAfterLast());
  }

  private static NativeArrowBatch intBatch(int[] values) {
    ByteBuffer data =
        ByteBuffer.allocateDirect(values.length * Integer.BYTES).order(ByteOrder.LITTLE_ENDIAN);
    for (int index = 0; index < values.length; index++) {
      data.putInt(index * Integer.BYTES, values[index]);
    }
    return new NativeArrowBatch(
        values.length,
        new NativeArrowArray[] {new NativeArrowArray(null, null, data, new NativeArrowArray[0])});
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

  private static final class RecordingBatchSource implements NativeBatchSource {
    private final byte[] schema = fixedInt32Schema();
    private final List<NativeArrowBatch> batches;
    private int next;
    private AtomicReference<ColumnBatch> previousBatch;
    private SchemaState schemaState;
    private boolean observedClosedPrevious;
    private int nextBatchCalls;

    private RecordingBatchSource(List<NativeArrowBatch> batches) {
      this.batches = new ArrayList<NativeArrowBatch>(batches);
    }

    private void setPreviousBatch(AtomicReference<ColumnBatch> previousBatch) {
      this.previousBatch = previousBatch;
    }

    private void setSchemaState(SchemaState schemaState) {
      this.schemaState = schemaState;
    }

    private boolean observedClosedPreviousOnNextBatch() {
      return observedClosedPrevious;
    }

    private int nextBatchCallCount() {
      return nextBatchCalls;
    }

    @Override
    public byte[] schema() {
      return schema;
    }

    @Override
    public NativeArrowBatch nextBatch() {
      nextBatchCalls++;
      ColumnBatch previous = previousBatch == null ? null : previousBatch.get();
      if (previous != null) {
        assertThrows(IllegalStateException.class, () -> previous.column(0).getInt(0));
        observedClosedPrevious = true;
      }
      if (schemaState != null) {
        assertFalse(schemaState.hasCachedConverter(1));
      }
      if (next >= batches.size()) {
        return null;
      }
      return batches.get(next++);
    }

    @Override
    public void close() {}
  }
}
