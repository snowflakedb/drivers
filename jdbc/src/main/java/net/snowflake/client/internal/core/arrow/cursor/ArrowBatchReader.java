package net.snowflake.client.internal.core.arrow.cursor;

import java.io.Closeable;
import java.io.IOException;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;

public interface ArrowBatchReader extends Closeable {
  boolean loadNextBatch() throws IOException;

  ColumnBatch getBatch() throws IOException;
}
