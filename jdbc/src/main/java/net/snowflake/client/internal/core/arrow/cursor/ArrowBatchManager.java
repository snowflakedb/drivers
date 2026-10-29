package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;

public final class ArrowBatchManager {
  private final CursorState cursor;
  private final ArrowResources resources;

  public ArrowBatchManager(CursorState cursor, ArrowResources resources) {
    this.cursor = cursor;
    this.resources = resources;
  }

  public boolean fetchNextRow() {
    if (cursor.isAfterLast()) {
      return false;
    }
    try {
      if (cursor.hasLoadedBatch() && cursor.hasNextRowInBatch()) {
        cursor.incrementRowInBatch();
        return true;
      }
      return advanceToNextBatch();
    } catch (IOException e) {
      throw new SFSQLException("Unable to advance Arrow results", e);
    }
  }

  private ColumnBatch loadNextNonEmptyBatch() throws IOException {
    while (resources.loadNextBatch()) {
      ColumnBatch root = resources.getReaderRoot();
      if (root.getRowCount() > 0) {
        return root;
      }
    }
    return null;
  }

  private boolean advanceToNextBatch() throws IOException {
    // The reader rebinds one column tree per batch instead of rebuilding it, so the
    // converters built against that tree stay valid and the cache is not reset here.
    ColumnBatch nextRoot = loadNextNonEmptyBatch();
    if (nextRoot == null) {
      cursor.setAfterLast();
      return false;
    }
    resources.setCurrentRoot(nextRoot);
    cursor.startNewBatch(resources.getCurrentRoot().getRowCount());
    return true;
  }
}
