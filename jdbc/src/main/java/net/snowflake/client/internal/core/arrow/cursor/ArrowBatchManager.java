package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import java.sql.SQLException;
import org.apache.arrow.vector.VectorSchemaRoot;

public final class ArrowBatchManager {
  private final CursorState cursor;
  private final BatchState batch;
  private final ArrowResources resources;
  private final SchemaState schema;

  public ArrowBatchManager(
      CursorState cursor, BatchState batch, ArrowResources resources, SchemaState schema) {
    this.cursor = cursor;
    this.batch = batch;
    this.resources = resources;
    this.schema = schema;
  }

  public boolean fetchNextRow() throws SQLException {
    if (cursor.isAfterLast()) {
      return false;
    }
    try {
      if (!batch.hasLoadedBatch()) {
        return moveToNextBatch();
      }
      if (batch.getCurrentRowInBatch() + 1 < batch.getCurrentBatchRowCount()) {
        batch.incrementRowInBatch();
        return true;
      }
      return moveToNextBatch();
    } catch (IOException e) {
      throw new SQLException("Unable to advance Arrow results", e);
    }
  }

  public void prefetchNextBatchForIsLast() throws SQLException {
    if (batch.hasPrefetchedBatch() || cursor.isAfterLast()) {
      return;
    }
    VectorSchemaRoot snapshot = resources.copyVectorSchemaRoot(resources.getActiveRoot());
    resources.setCurrentRoot(snapshot, true);
    schema.resetConverters();
    try {
      VectorSchemaRoot nextRoot = loadNextNonEmptyBatch();
      if (nextRoot == null) {
        cursor.setOnLastRow(true);
        return;
      }
      resources.setPrefetchedRoot(nextRoot);
      batch.setHasPrefetchedBatch(true);
      cursor.setOnLastRow(false);
    } catch (IOException e) {
      throw new SQLException("Unable to prefetch next batch", e);
    }
  }

  private VectorSchemaRoot loadNextNonEmptyBatch() throws IOException {
    while (resources.loadNextBatch()) {
      VectorSchemaRoot root = resources.getReaderRoot();
      if (root.getRowCount() > 0) {
        return root;
      }
    }
    return null;
  }

  private boolean moveToNextBatch() throws IOException, SQLException {
    VectorSchemaRoot nextRoot;
    if (batch.hasPrefetchedBatch()) {
      nextRoot = resources.takePrefetchedRoot();
      batch.setHasPrefetchedBatch(false);
    } else {
      nextRoot = loadNextNonEmptyBatch();
    }
    if (nextRoot == null) {
      cursor.setAfterLast();
      return false;
    }
    resources.setCurrentRoot(nextRoot, false);
    batch.setCurrentBatchRowCount(resources.getCurrentRootRowCount());
    batch.resetCurrentRowInBatch();
    batch.resetHasLoadedBatch();
    schema.resetConverters();
    schema.ensureInitialized(resources.getCurrentRoot());
    return true;
  }
}
