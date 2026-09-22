package net.snowflake.client.internal.api.implementation.statement;

import java.sql.Statement;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Column-major accumulation of parameter values across {@code addBatch()} calls. Each map entry
 * holds a {@code List<String>} — one element per accumulated row. The entry's bind type is promoted
 * from {@code "ANY"} to a real type when the first non-null value arrives in a column that has been
 * all-nulls so far.
 */
final class PreparedBatch {
  private static final SFLogger logger = SFLoggerFactory.getLogger(PreparedBatch.class);

  private final Map<Integer, ParameterValue> columns = new HashMap<>();
  private int rowCount = 0;

  /**
   * Append a row built from the per-row {@code currentValues} map. The first row establishes the
   * batch's column set (the indexes the caller bound); every subsequent row must supply the same
   * indexes, so a row that omits one trips {@link BatchColumnValidator}'s missing-value check.
   * Two-pass: every column is validated before any is mutated so a type-mismatch on a later column
   * doesn't leave earlier columns at a different length.
   */
  void addRow(Map<Integer, ParameterValue> currentValues) {
    Set<Integer> columnIndexes = rowCount == 0 ? currentValues.keySet() : columns.keySet();
    for (int parameterIndex : columnIndexes) {
      BatchColumnValidator.validate(
          parameterIndex, columns.get(parameterIndex), currentValues.get(parameterIndex));
    }
    for (int parameterIndex : columnIndexes) {
      commit(parameterIndex, currentValues);
    }
    rowCount++;
  }

  void clear() {
    columns.clear();
    rowCount = 0;
  }

  int size() {
    return rowCount;
  }

  boolean isEmpty() {
    return rowCount == 0;
  }

  private Map<Integer, ParameterValue> snapshot() {
    return new HashMap<>(columns);
  }

  /**
   * Executes array-bind-capable statements in one round trip and other statements one row at a
   * time. Array-bind counts expand to per-row {@code 1} when the server's aggregate count equals
   * {@code batchSize} (SNOW-14034), else {@link Statement#SUCCESS_NO_INFO}.
   */
  long[] executeAll(SnowflakePreparedStatementImpl stmt, String sql) {
    final int batchSize = size();
    stmt.clearBatchQueryIds();
    if (batchSize == 0) {
      stmt.finalizeBatch(null);
      return new long[0];
    }
    long[] result = new long[batchSize];
    SFBatchUpdateException pending = null;
    try {
      if (stmt.arrayBindSupported()) {
        long updateCount = serializeBatchAndExecute(stmt, sql);
        result = expandUpdateCounts(updateCount, batchSize);
        stmt.recordBatchQueryId();
      } else {
        CoreException firstFailure = executeRows(stmt, sql, result);
        if (firstFailure != null) {
          pending =
              SnowflakeStatementImpl.buildBatchFailureException(
                  firstFailure, toIntUpdateCounts(result));
        }
      }
    } catch (CoreException e) {
      pending = SnowflakeStatementImpl.buildBatchFailureException(e, allFailed(batchSize));
      stmt.recordBatchQueryId();
    } finally {
      stmt.finalizeBatch(pending);
    }
    if (pending != null) {
      throw pending;
    }
    return result;
  }

  private long serializeBatchAndExecute(SnowflakePreparedStatementImpl stmt, String sql) {
    return executeWithBindings(stmt, sql, useStageBinding(stmt));
  }

  /**
   * Recurses at most once: a stage-binding-disabled failure on the stage-bound path retries with
   * inline JSON bindings, whose own failure is rethrown rather than retried again.
   */
  private long executeWithBindings(
      SnowflakePreparedStatementImpl stmt, String sql, boolean useStage) {
    PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        useStage
            ? PreparedStatementCsvBindings.serialize(snapshot(), rowCount)
            : PreparedStatementBindingSerializer.serialize(snapshot());
    try {
      return executeWithBindings(stmt, sql, nativeBindings);
    } catch (CoreException e) {
      if (useStage && e.isStageBindingDisabled()) {
        return executeWithBindings(stmt, sql, false);
      }
      throw e;
    }
  }

  private CoreException executeRows(
      SnowflakePreparedStatementImpl stmt, String sql, long[] updateCounts) {
    CoreException firstFailure = null;
    for (int row = 0; row < rowCount; row++) {
      CoreException rowFailure = executeRow(stmt, sql, row, updateCounts);
      if (rowFailure != null && firstFailure == null) {
        firstFailure = rowFailure;
      }
    }
    return firstFailure;
  }

  private CoreException executeRow(
      SnowflakePreparedStatementImpl stmt, String sql, int row, long[] updateCounts) {
    try {
      PreparedStatementBindingSerializer.NativeBindings nativeBindings =
          PreparedStatementBindingSerializer.serialize(snapshotRow(row));
      long updateCount = executeWithBindings(stmt, sql, nativeBindings);
      updateCounts[row] =
          updateCount == StatementTypeClassifier.NO_UPDATE_COUNT
              ? Statement.SUCCESS_NO_INFO
              : updateCount;
      return null;
    } catch (CoreException e) {
      updateCounts[row] = Statement.EXECUTE_FAILED;
      return e;
    } finally {
      stmt.recordBatchQueryId();
    }
  }

  /**
   * Manual try/finally rather than try-with-resources: a close-throws-after-RPC-success would
   * otherwise be caught by the outer catch and falsely mark the batch as failed.
   */
  private long executeWithBindings(
      SnowflakePreparedStatementImpl stmt,
      String sql,
      PreparedStatementBindingSerializer.NativeBindings nativeBindings) {
    try {
      return stmt.executeLargeUpdateWithBindings(sql, nativeBindings);
    } finally {
      try {
        nativeBindings.close();
      } catch (RuntimeException closeEx) {
        logger.warn("Failed to close native binding buffer after RPC", closeEx);
      }
    }
  }

  /**
   * Stage-bind decision, mirroring legacy {@code SFStatement}: {@code 0 < threshold && threshold <=
   * cells}, where {@code cells = rows × columns}. Called only from the array-bind path; non-INSERT
   * batches execute one row at a time with JSON bindings.
   */
  private boolean useStageBinding(SnowflakePreparedStatementImpl stmt) {
    int threshold = stmt.stageArrayBindingThreshold();
    long cells = (long) rowCount * columns.size();
    return threshold > 0 && cells >= threshold && stmt.arrayBindSupported();
  }

  private static long[] expandUpdateCounts(long aggregate, int batchSize) {
    long perRow = aggregate == batchSize ? 1L : Statement.SUCCESS_NO_INFO;
    long[] result = new long[batchSize];
    Arrays.fill(result, perRow);
    return result;
  }

  private static int[] allFailed(int batchSize) {
    int[] failed = new int[batchSize];
    Arrays.fill(failed, Statement.EXECUTE_FAILED);
    return failed;
  }

  private static int[] toIntUpdateCounts(long[] updateCounts) {
    int[] result = new int[updateCounts.length];
    for (int i = 0; i < updateCounts.length; i++) {
      result[i] = SnowflakeStatementImpl.toBatchInt(updateCounts[i]);
    }
    return result;
  }

  private Map<Integer, ParameterValue> snapshotRow(int row) {
    Map<Integer, ParameterValue> values = new HashMap<>();
    for (Map.Entry<Integer, ParameterValue> entry : columns.entrySet()) {
      ParameterValue column = entry.getValue();
      @SuppressWarnings("unchecked")
      List<String> columnValues = (List<String>) column.value();
      values.put(entry.getKey(), new ParameterValue(column.bindType(), columnValues.get(row)));
    }
    return values;
  }

  private void commit(int parameterIndex, Map<Integer, ParameterValue> currentValues) {
    ParameterValue parameterValue = currentValues.get(parameterIndex);
    SnowflakeType newType = parameterValue.bindType();
    String stringValue = (String) parameterValue.value();
    ParameterValue existing = columns.get(parameterIndex);
    if (existing == null) {
      List<String> values = new ArrayList<>();
      values.add(stringValue);
      columns.put(parameterIndex, new ParameterValue(newType, values));
      return;
    }
    @SuppressWarnings("unchecked")
    List<String> values = (List<String>) existing.value();
    SnowflakeType prevType = existing.bindType();
    // Promote ANY (or all-null column) → real type on first non-null. Safe — no existing
    // data is reinterpreted.
    if (stringValue != null
        && prevType != newType
        && (prevType == SnowflakeType.ANY || allNullsSoFar(values))) {
      columns.put(parameterIndex, new ParameterValue(newType, values));
    }
    values.add(stringValue);
  }

  private static boolean allNullsSoFar(List<String> values) {
    for (String v : values) {
      if (v != null) {
        return false;
      }
    }
    return true;
  }
}
