package net.snowflake.client.internal.core.arrow.cursor;

import java.nio.ByteBuffer;
import java.util.List;
import net.snowflake.client.internal.core.arrow.vector.Column.Field;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.unicore.NativeArrowStream;

/**
 * Reads native Arrow batches into one reusable {@link ColumnBatch}.
 *
 * <p>The column tree and the buffer hand-off array are built once from the schema and rebound per
 * batch, so {@link #getBatch()} returns the same instance for the life of the result set and the
 * converters built against it stay valid. Each batch clears the previous binding before asking for
 * the next one, because the native stream frees the batch behind it at that point.
 *
 * <p>A {@link #loadNextBatch()} that runs after {@link #close()} fails in the native stream with a
 * closed-stream exception, and leaves the batch cleared.
 */
public final class NativeBatchReader implements ArrowBatchReader {
  private final NativeBatchSource source;
  private final List<Field> schema;
  private final ByteBuffer[] slots;
  private final ColumnBatch current;

  public NativeBatchReader(NativeArrowStream stream) {
    this(adapt(stream));
  }

  NativeBatchReader(NativeBatchSource source) {
    this.source = source;
    try {
      this.schema = ColumnBatch.decodeSchema(source.schema());
      this.current = ColumnBatch.empty(schema);
      this.slots = current.newSlots();
    } catch (RuntimeException failure) {
      source.close();
      throw failure;
    }
  }

  @Override
  public boolean loadNextBatch() {
    // Clear first: the native stream frees the previous batch inside nextBatch, so no
    // column may still reference its buffers once that call starts.
    current.clear();
    int rowCount = source.nextBatch(slots);
    if (rowCount == NativeArrowStream.END_OF_STREAM) {
      return false;
    }
    current.bind(slots, rowCount);
    return true;
  }

  @Override
  public ColumnBatch getBatch() {
    return current;
  }

  @Override
  public void close() {
    current.close();
    source.close();
  }

  private static NativeBatchSource adapt(final NativeArrowStream stream) {
    return new NativeBatchSource() {
      @Override
      public byte[] schema() {
        return stream.schema();
      }

      @Override
      public int nextBatch(ByteBuffer[] slots) {
        return stream.nextBatch(slots);
      }

      @Override
      public void close() {
        stream.close();
      }
    };
  }
}
