package net.snowflake.client.internal.core.arrow.cursor;

import java.io.IOException;
import java.sql.SQLException;
import java.util.List;
import org.apache.arrow.c.ArrowArrayStream;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.vector.FieldVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.ipc.ArrowReader;

public final class ArrowResources {
  private VectorSchemaRoot currentRoot;
  private boolean currentRootOwned = false;
  private VectorSchemaRoot prefetchedRoot;
  private ArrowArrayStream stream;
  private ArrowReader reader;
  private BufferAllocator allocator;

  public ArrowResources(ArrowArrayStream stream, BufferAllocator allocator, ArrowReader reader) {
    this.stream = stream;
    this.allocator = allocator;
    this.reader = reader;
  }

  public VectorSchemaRoot getActiveRoot() throws SQLException {
    if (currentRoot != null) {
      return currentRoot;
    }
    try {
      return reader.getVectorSchemaRoot();
    } catch (IOException e) {
      throw new SQLException("Unable to read Arrow schema", e);
    }
  }

  void setCurrentRoot(VectorSchemaRoot root, boolean owned) {
    if (currentRoot != null && currentRootOwned) {
      currentRoot.close();
    }
    currentRoot = root;
    currentRootOwned = owned;
  }

  VectorSchemaRoot getCurrentRoot() {
    return currentRoot;
  }

  int getCurrentRootRowCount() {
    return currentRoot.getRowCount();
  }

  void setPrefetchedRoot(VectorSchemaRoot root) {
    prefetchedRoot = root;
  }

  VectorSchemaRoot takePrefetchedRoot() {
    VectorSchemaRoot root = prefetchedRoot;
    prefetchedRoot = null;
    return root;
  }

  boolean loadNextBatch() throws IOException {
    return reader.loadNextBatch();
  }

  VectorSchemaRoot getReaderRoot() throws IOException {
    return reader.getVectorSchemaRoot();
  }

  VectorSchemaRoot copyVectorSchemaRoot(VectorSchemaRoot source) {
    VectorSchemaRoot copy = VectorSchemaRoot.create(source.getSchema(), allocator);
    int rowCount = source.getRowCount();
    copy.setRowCount(rowCount);
    List<FieldVector> sourceVectors = source.getFieldVectors();
    List<FieldVector> targetVectors = copy.getFieldVectors();
    for (int vectorIndex = 0; vectorIndex < sourceVectors.size(); vectorIndex++) {
      FieldVector sourceVector = sourceVectors.get(vectorIndex);
      FieldVector targetVector = targetVectors.get(vectorIndex);
      targetVector.allocateNew();
      for (int rowIndex = 0; rowIndex < rowCount; rowIndex++) {
        targetVector.copyFromSafe(rowIndex, rowIndex, sourceVector);
      }
      targetVector.setValueCount(rowCount);
    }
    return copy;
  }

  public void closeCurrentRootIfOwned() {
    if (currentRootOwned && currentRoot != null) {
      currentRoot.close();
    }
  }

  public void closePrefetchedRoot() {
    if (prefetchedRoot != null) {
      prefetchedRoot.close();
    }
  }

  public void closeReader() throws IOException {
    if (reader != null) {
      reader.close();
    }
  }

  public void closeStream() {
    if (stream != null) {
      stream.close();
    }
  }

  public void closeAllocator() {
    if (allocator != null) {
      allocator.close();
    }
  }

  public void reset() {
    currentRoot = null;
    currentRootOwned = false;
    prefetchedRoot = null;
    reader = null;
    stream = null;
    allocator = null;
  }
}
