package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import java.util.List;
import net.snowflake.client.internal.core.arrow.vector.Column.Field;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.unicore.NativeArrowBatch;
import net.snowflake.client.internal.unicore.NativeArrowStream;

public final class NativeBatchReader implements ArrowBatchReader {
  private final NativeBatchSource source;
  private final List<Field> schema;
  private ColumnBatch current;

  public NativeBatchReader(NativeArrowStream stream) {
    this(adapt(stream));
  }

  NativeBatchReader(NativeBatchSource source) {
    this.source = source;
    try {
      this.schema = ColumnBatch.decodeSchema(source.schema());
      this.current = ColumnBatch.empty(schema);
    } catch (RuntimeException failure) {
      source.close();
      throw failure;
    }
  }

  @Override
  public boolean loadNextBatch() {
    releaseCurrent();
    NativeArrowBatch batch = source.nextBatch();
    if (batch == null) {
      current = ColumnBatch.empty(schema);
      return false;
    }
    current = ColumnBatch.view(schema, batch);
    return true;
  }

  @Override
  public ColumnBatch getBatch() throws IOException {
    return current;
  }

  @Override
  public void close() {
    releaseCurrent();
    current = null;
    source.close();
  }

  private void releaseCurrent() {
    if (current != null) {
      current.close();
    }
  }

  private static NativeBatchSource adapt(final NativeArrowStream stream) {
    return new NativeBatchSource() {
      @Override
      public byte[] schema() {
        return stream.schema();
      }

      @Override
      public NativeArrowBatch nextBatch() {
        return stream.nextBatch();
      }

      @Override
      public void close() {
        stream.close();
      }
    };
  }
}
