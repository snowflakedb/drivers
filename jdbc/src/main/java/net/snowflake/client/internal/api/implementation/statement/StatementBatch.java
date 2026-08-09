package net.snowflake.client.internal.api.implementation.statement;

import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;

/**
 * Per-statement batch of SQL strings. {@link #executeAll} runs each entry sequentially per JDBC §14
 * (continue-on-error) and surfaces the first failure as a {@link SFBatchUpdateException}.
 */
final class StatementBatch {
  private final List<String> entries = new ArrayList<>();

  void add(String sql) {
    entries.add(sql);
  }

  void clear() {
    entries.clear();
  }

  int size() {
    return entries.size();
  }

  int[] executeAll(SnowflakeStatementImpl stmt) {
    int[] updateCounts = new int[entries.size()];
    stmt.clearBatchQueryIds();
    SFBatchUpdateException pending = null;
    try {
      CoreException firstFailure = runEntries(stmt, updateCounts);
      if (firstFailure != null) {
        pending = SnowflakeStatementImpl.buildBatchFailureException(firstFailure, updateCounts);
      }
    } finally {
      stmt.finalizeBatch(pending);
    }
    if (pending != null) {
      throw pending;
    }
    return updateCounts;
  }

  /** Returns the first per-row failure, or {@code null} if every entry succeeded. */
  private CoreException runEntries(SnowflakeStatementImpl stmt, int[] updateCounts) {
    CoreException firstFailure = null;
    for (int i = 0; i < entries.size(); i++) {
      CoreException rowFailure = runOne(stmt, entries.get(i), updateCounts, i);
      if (rowFailure != null && firstFailure == null) {
        firstFailure = rowFailure;
      }
    }
    return firstFailure;
  }

  /** Returns {@code null} on success, or the failure exception on per-row error. */
  private static CoreException runOne(
      SnowflakeStatementImpl stmt, String sql, int[] updateCounts, int row) {
    try {
      long count = stmt.executeLargeUpdateWithBindings(sql, null);
      updateCounts[row] = SnowflakeStatementImpl.toBatchInt(count);
      return null;
    } catch (CoreException e) {
      updateCounts[row] = Statement.EXECUTE_FAILED;
      return e;
    } finally {
      // Always append, even on failure (queryId is null when applySingleResult never ran), so
      // updateCounts and batchQueryIds stay positionally aligned.
      stmt.recordBatchQueryId();
    }
  }
}
