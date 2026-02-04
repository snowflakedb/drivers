package net.snowflake.client.internal.core.arrow.cursor;

import java.io.Closeable;
import java.io.IOException;
import java.sql.SQLException;
import org.apache.arrow.c.ArrowArrayStream;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.ipc.ArrowReader;

public final class ArrowResources {
  private ArrowArrayStream stream;
  private ArrowReader reader;
  private BufferAllocator allocator;
  private VectorSchemaRoot currentRoot;

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

  void setCurrentRoot(VectorSchemaRoot root) {
    currentRoot = root;
  }

  VectorSchemaRoot getCurrentRoot() {
    return currentRoot;
  }

  boolean loadNextBatch() throws IOException {
    return reader.loadNextBatch();
  }

  VectorSchemaRoot getReaderRoot() throws IOException {
    return reader.getVectorSchemaRoot();
  }

  public void closeAll() throws SQLException {
    SQLException failure = null;
    failure = closeResource(failure, this::closeReader);
    failure = closeResource(failure, this::closeStream);
    failure = closeResource(failure, this::closeAllocator);
    if (failure != null) {
      throw failure;
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
    reader = null;
    stream = null;
    allocator = null;
  }

  private SQLException appendCloseFailure(SQLException failure, String message, Exception cause) {
    SQLException next = new SQLException(message, cause);
    if (failure == null) {
      return next;
    }
    failure.addSuppressed(next);
    return failure;
  }

  private SQLException closeResource(SQLException failure, Closeable action) {
    if (action == null) {
      return failure;
    }
    try {
      action.close();
      return failure;
    } catch (Exception e) {
      return appendCloseFailure(failure, "Failure while calling " + action, e);
    }
  }
}
