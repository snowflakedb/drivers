package net.snowflake.client.internal.api.implementation.statement;

import java.sql.SQLException;

final class PreparedBatchExecutor {
  BatchExecutionResult executeBatch(
      PreparedBatchState batchState,
      boolean arrayBindingEnabled,
      BatchCountAccumulator countAccumulator,
      BindingExecutor bindingExecutor)
      throws SQLException {
    if (batchState.isEmpty()) {
      return countAccumulator.toResult();
    }
    if (arrayBindingEnabled) {
      return executeArrayBoundBatch(batchState, countAccumulator, bindingExecutor);
    }
    return executePerEntryBatch(batchState, countAccumulator, bindingExecutor);
  }

  private BatchExecutionResult executeArrayBoundBatch(
      PreparedBatchState batchState,
      BatchCountAccumulator countAccumulator,
      BindingExecutor bindingExecutor)
      throws SQLException {
    long rowsAffected = bindingExecutor.execute(batchState.toArrayBoundColumns());
    return countAccumulator.shapeArrayBoundCounts(rowsAffected, batchState.batchSize());
  }

  private BatchExecutionResult executePerEntryBatch(
      PreparedBatchState batchState,
      BatchCountAccumulator countAccumulator,
      BindingExecutor bindingExecutor)
      throws SQLException {
    SQLException firstException = null;

    for (int i = 0; i < batchState.batchSize(); i++) {
      try {
        long rowsAffected = bindingExecutor.execute(batchState.rowAt(i));
        countAccumulator.recordSuccess(i, rowsAffected);
      } catch (SQLException e) {
        if (firstException == null) {
          firstException = e;
        }
        countAccumulator.recordFailure(i);
      }
    }

    if (firstException != null) {
      throw countAccumulator.toBatchUpdateException(firstException);
    }
    return countAccumulator.toResult();
  }

  interface BindingExecutor {
    long execute(PreparedStatementBinding[] bindings) throws SQLException;
  }
}
