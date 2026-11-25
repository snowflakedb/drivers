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
      if (cursor.isDrained()) {
        cursor.setAfterLast();
        return false;
      }
      return advanceToNextBatch();
    } catch (IOException e) {
      throw new SFSQLException("Unable to advance Arrow results", e);
    }
  }

  /**
   * Reads the first non-empty batch now, while the result set is still being built, and leaves the
   * cursor before the first row. The first {@link #fetchNextRow()} then advances into that batch.
   * An empty reader is remembered so that {@code next()} does not read it again.
   */
  public void openFirstBatch() {
    try {
      ColumnBatch first = loadNextNonEmptyBatch();
      if (first == null) {
        cursor.markDrained();
        return;
      }
      resources.setCurrentRoot(first);
      cursor.holdBatch(first.getRowCount());
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
