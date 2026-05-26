package net.snowflake.client.internal.api.implementation.statement;

import java.sql.BatchUpdateException;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
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

  /**
   * Append a row built from the per-row {@code currentValues} map. Two-pass: every column is
   * validated before any is mutated so a type-mismatch on a later column doesn't leave earlier
   * columns at a different length.
   */
  void addRow(SqlPlaceholderMetadata meta, Map<Integer, ParameterValue> currentValues)
      throws SQLException {
    if (meta.hasMixedPlaceholderStyles()) {
      throw new SnowflakeSQLException(
          "Mixed positional and numeric placeholders are not supported");
    }
    for (int parameterIndex : meta.referencedParameterIndexes()) {
      validate(parameterIndex, currentValues);
    }
    for (int parameterIndex : meta.referencedParameterIndexes()) {
      commit(parameterIndex, currentValues);
    }
  }

  void clear() {
    columns.clear();
  }

  /** Number of accumulated rows; derived from any column's list size (every column same length). */
  int size() {
    if (columns.isEmpty()) {
      return 0;
    }
    return ((List<?>) columns.values().iterator().next().value()).size();
  }

  boolean isEmpty() {
    return size() == 0;
  }

  private Map<Integer, ParameterValue> snapshot() {
    return new HashMap<>(columns);
  }

  /**
   * Single-roundtrip array-bind execution. Returns one entry per row: per-row {@code 1} when the
   * server's aggregate count equals {@code batchSize} (SNOW-14034), else {@link
   * Statement#SUCCESS_NO_INFO}. On failure throws {@link BatchUpdateException} with all entries set
   * to {@link Statement#EXECUTE_FAILED}.
   */
  long[] executeAll(SnowflakePreparedStatementImpl stmt, String sql, SqlPlaceholderMetadata meta)
      throws SQLException {
    final int batchSize = size();
    stmt.clearBatchQueryIds();
    if (batchSize == 0) {
      stmt.finalizeBatch(null);
      return new long[0];
    }
    long[] result = new long[0];
    BatchUpdateException pending = null;
    try {
      long updateCount = runOnce(stmt, sql, meta);
      result = expandUpdateCounts(updateCount, batchSize);
      stmt.recordBatchQueryId();
    } catch (SQLException e) {
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

  /**
   * Manual try/finally rather than try-with-resources: a close-throws-after-RPC-success would
   * otherwise be caught by the outer catch(SQLException) and falsely mark the batch as failed.
   */
  private long runOnce(SnowflakePreparedStatementImpl stmt, String sql, SqlPlaceholderMetadata meta)
      throws SQLException {
    PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(meta, snapshot());
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

  private void validate(int parameterIndex, Map<Integer, ParameterValue> currentValues)
      throws SQLException {
    ParameterValue parameterValue = currentValues.get(parameterIndex);
    if (parameterValue == null) {
      throw new SnowflakeSQLException("Missing value for parameter index: " + parameterIndex);
    }
    ParameterValue existing = columns.get(parameterIndex);
    if (existing == null) {
      return;
    }
    String stringValue = (String) parameterValue.value();
    if (stringValue == null) {
      return;
    }
    String prevType = existing.bindType();
    String newType = parameterValue.bindType();
    if ("ANY".equalsIgnoreCase(prevType) || prevType.equalsIgnoreCase(newType)) {
      return;
    }
    @SuppressWarnings("unchecked")
    List<String> values = (List<String>) existing.value();
    if (allNullsSoFar(values)) {
      return;
    }
    int prevRow = values.size();
    throw new SnowflakeSQLException(
        ErrorCode.ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED,
        "Array binding does not support mixed types: parameter "
            + parameterIndex
            + " was bound with type "
            + prevType
            + " in row "
            + prevRow
            + " and type "
            + newType
            + " in row "
            + (prevRow + 1));
  }

  private void commit(int parameterIndex, Map<Integer, ParameterValue> currentValues) {
    ParameterValue parameterValue = currentValues.get(parameterIndex);
    String newType = parameterValue.bindType();
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
    String prevType = existing.bindType();
    // Promote ANY (or all-null column) → real type on first non-null. Safe — no existing
    // data is reinterpreted.
    if (stringValue != null
        && !prevType.equalsIgnoreCase(newType)
        && ("ANY".equalsIgnoreCase(prevType) || allNullsSoFar(values))) {
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
