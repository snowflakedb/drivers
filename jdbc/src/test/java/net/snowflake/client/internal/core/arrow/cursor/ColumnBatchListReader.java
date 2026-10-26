package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import java.util.List;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;

public final class ColumnBatchListReader implements ArrowBatchReader {
  private final List<ColumnBatch> batches;
  private int next;
  private ColumnBatch current;

  public ColumnBatchListReader(List<ColumnBatch> batches) {
    this.batches = batches;
    this.current = batches.isEmpty() ? null : batches.get(0);
  }

  @Override
  public boolean loadNextBatch() {
    if (next >= batches.size()) {
      return false;
    }
    current = batches.get(next);
    next++;
    return true;
  }

  @Override
  public ColumnBatch getBatch() {
    return current;
  }

  @Override
  public void close() throws IOException {
    for (ColumnBatch batch : batches) {
      batch.close();
    }
  }
}
