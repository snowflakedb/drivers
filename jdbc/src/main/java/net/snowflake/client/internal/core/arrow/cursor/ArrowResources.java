package net.snowflake.client.internal.core.arrow.cursor;

import static net.snowflake.client.api.exception.ErrorCode.INTERNAL_ERROR;

import java.io.Closeable;
import java.io.IOException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;

public final class ArrowResources {
  private ArrowBatchReader reader;
  private ColumnBatch currentRoot;

  public ArrowResources(ArrowBatchReader reader) {
    this.reader = reader;
  }

  public ColumnBatch getActiveRoot() {
    if (currentRoot != null) {
      return currentRoot;
    }
    try {
      return reader.getBatch();
    } catch (IOException e) {
      throw SFSQLException.fromErrorCode(INTERNAL_ERROR, "Unable to read Arrow schema", e);
    }
  }

  void setCurrentRoot(ColumnBatch root) {
    currentRoot = root;
  }

  ColumnBatch getCurrentRoot() {
    return currentRoot;
  }

  boolean loadNextBatch() throws IOException {
    return reader.loadNextBatch();
  }

  ColumnBatch getReaderRoot() throws IOException {
    return reader.getBatch();
  }

  public void closeAll() {
    SFSQLException failure = null;
    failure = closeResource(failure, this::closeReader);
    if (failure != null) {
      throw failure;
    }
  }

  public void closeReader() throws IOException {
    if (reader != null) {
      reader.close();
    }
  }

  public void reset() {
    currentRoot = null;
    reader = null;
  }

  private SFSQLException appendCloseFailure(
      SFSQLException failure, String message, Exception cause) {
    SFSQLException next = new SFSQLException(message, cause);
    if (failure == null) {
      return next;
    }
    failure.addSuppressed(next);
    return failure;
  }

  private SFSQLException closeResource(SFSQLException failure, Closeable action) {
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
