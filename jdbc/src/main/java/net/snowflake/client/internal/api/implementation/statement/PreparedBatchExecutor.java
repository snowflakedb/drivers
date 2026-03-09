package net.snowflake.client.internal.api.implementation.statement;

import java.sql.SQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

final class PreparedBatchExecutor {
  private static final SFLogger logger = SFLoggerFactory.getLogger(PreparedBatchExecutor.class);

  BatchExecutionResult executeBatch(
      PreparedBatchState batchState,
      boolean arrayBindingEnabled,
      BatchCountAccumulator countAccumulator,
      BindingExecutor bindingExecutor)
      throws SQLException {
    if (batchState.isEmpty()) {
      logger.debug("Skipping prepared batch execution because the batch is empty.");
      return countAccumulator.toResult();
    }
    logger.debug(
        "Executing prepared batch: batchSize={}, executionStrategy={}",
        batchState.batchSize(),
        arrayBindingEnabled ? "array-bind" : "per-entry");
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
    logger.debug(
        "Prepared array-bound batch executed: batchSize={}, rowsAffected={}",
        batchState.batchSize(),
        rowsAffected);
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
      logger.debug(
          "Prepared per-entry batch completed with failures: batchSize={}", batchState.batchSize());
      throw countAccumulator.toBatchUpdateException(firstException);
    }
    logger.debug(
        "Prepared per-entry batch executed successfully: batchSize={}", batchState.batchSize());
    return countAccumulator.toResult();
  }

  interface BindingExecutor {
    long execute(PreparedStatementBinding[] bindings) throws SQLException;
  }
}
