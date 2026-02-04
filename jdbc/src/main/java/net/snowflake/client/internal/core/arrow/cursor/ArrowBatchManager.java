package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import java.sql.SQLException;
import org.apache.arrow.vector.VectorSchemaRoot;

public final class ArrowBatchManager {
  private final CursorState cursor;
  private final ArrowResources resources;
  private final SchemaState schema;

  public ArrowBatchManager(CursorState cursor, ArrowResources resources, SchemaState schema) {
    this.cursor = cursor;
    this.resources = resources;
    this.schema = schema;
  }

  public boolean fetchNextRow() throws SQLException {
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
      throw new SQLException("Unable to advance Arrow results", e);
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

  private boolean advanceToNextBatch() throws IOException, SQLException {
    VectorSchemaRoot nextRoot = loadNextNonEmptyBatch();
    if (nextRoot == null) {
      cursor.setAfterLast();
      return false;
    }
    resources.setCurrentRoot(nextRoot);
    cursor.startNewBatch(resources.getCurrentRoot().getRowCount());
    schema.resetConverterCache();
    return true;
  }
}
