package net.snowflake.client.internal.api.implementation.statement;

import java.sql.BatchUpdateException;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;

abstract class BatchCountAccumulator {
  static BatchCountAccumulator forIntCounts(int batchSize) {
    return new IntBatchCountAccumulator(batchSize);
  }

  static BatchCountAccumulator forLongCounts(int batchSize) {
    return new LongBatchCountAccumulator(batchSize);
  }

  abstract void recordSuccess(int index, long rowsAffected) throws SQLException;

  abstract void recordFailure(int index);

  abstract BatchExecutionResult toResult();

  abstract BatchExecutionResult shapeArrayBoundCounts(long rowsAffected, int batchSize)
      throws SQLException;

  abstract BatchUpdateException toBatchUpdateException(SQLException firstException);

  private static final class IntBatchCountAccumulator extends BatchCountAccumulator {
    private final int[] counts;

    private IntBatchCountAccumulator(int batchSize) {
      this.counts = new int[batchSize];
    }

    @Override
    void recordSuccess(int index, long rowsAffected) throws SQLException {
      if (rowsAffected < 0) {
        counts[index] = Statement.SUCCESS_NO_INFO;
        return;
      }
      if (rowsAffected > Integer.MAX_VALUE) {
        throw new SQLException(
            "Batch update count exceeds Integer.MAX_VALUE at index " + index + ": " + rowsAffected);
      }
      counts[index] = (int) rowsAffected;
    }

    @Override
    void recordFailure(int index) {
      counts[index] = Statement.EXECUTE_FAILED;
    }

    @Override
    BatchExecutionResult toResult() {
      return new BatchExecutionResult(counts, null);
    }

    @Override
    BatchExecutionResult shapeArrayBoundCounts(long rowsAffected, int batchSize)
        throws SQLException {
      if (rowsAffected == batchSize) {
        Arrays.fill(counts, 1);
        return toResult();
      }
      if (rowsAffected > Integer.MAX_VALUE) {
        throw new SQLException(
            "Batch update count exceeds Integer.MAX_VALUE for array-bound batch: " + rowsAffected);
      }
      return new BatchExecutionResult(new int[] {(int) rowsAffected}, null);
    }

    @Override
    BatchUpdateException toBatchUpdateException(SQLException firstException) {
      return new BatchUpdateException(
          firstException.getMessage(),
          firstException.getSQLState(),
          firstException.getErrorCode(),
          counts,
          firstException);
    }
  }

  private static final class LongBatchCountAccumulator extends BatchCountAccumulator {
    private final long[] counts;

    private LongBatchCountAccumulator(int batchSize) {
      this.counts = new long[batchSize];
    }

    @Override
    void recordSuccess(int index, long rowsAffected) {
      counts[index] = rowsAffected < 0 ? Statement.SUCCESS_NO_INFO : rowsAffected;
    }

    @Override
    void recordFailure(int index) {
      counts[index] = Statement.EXECUTE_FAILED;
    }

    @Override
    BatchExecutionResult toResult() {
      return new BatchExecutionResult(null, counts);
    }

    @Override
    BatchExecutionResult shapeArrayBoundCounts(long rowsAffected, int batchSize) {
      if (rowsAffected == batchSize) {
        Arrays.fill(counts, 1L);
        return toResult();
      }
      return new BatchExecutionResult(null, new long[] {rowsAffected});
    }

    @Override
    BatchUpdateException toBatchUpdateException(SQLException firstException) {
      return new BatchUpdateException(
          firstException.getMessage(),
          firstException.getSQLState(),
          firstException.getErrorCode(),
          counts,
          firstException);
    }
  }
}
