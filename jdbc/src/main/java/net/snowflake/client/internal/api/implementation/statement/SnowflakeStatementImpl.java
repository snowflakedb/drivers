package net.snowflake.client.internal.api.implementation.statement;

import static net.snowflake.client.internal.api.implementation.statement.StatementTypeClassifier.NO_UPDATE_COUNT;

import java.sql.BatchUpdateException;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.MultiStatementResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.StringUtil;

public class SnowflakeStatementImpl implements Statement, SnowflakeStatement, DelegatingWrapper {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeStatementImpl.class);

  protected final InternalSnowflakeConnection connection;
  protected final CoreDriverApi coreDriverApi;

  private final AtomicBoolean closed = new AtomicBoolean(false);
  protected int maxRows = 0;
  protected int queryTimeout = 0;
  protected int fetchSize = 0;
  protected StatementHandle statementHandle;
  protected ResultSet currentResultSet;
  protected long currentUpdateCount = NO_UPDATE_COUNT;
  protected String queryId;
  protected final Set<ResultSet> openResultSets = ConcurrentHashMap.newKeySet();
  private final StatementBatch batch = new StatementBatch();
  /** Per-batch-entry query IDs collected during {@link #executeBatch()}. */
  private final List<String> batchQueryIds = new ArrayList<>();
  /** Non-null only while navigating a multi-statement result set sequence. */
  private MultiStatementState multiState;

  public SnowflakeStatementImpl(
      InternalSnowflakeConnection connection, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.coreDriverApi = coreDriverApi;
    try {
      this.statementHandle = coreDriverApi.statementNew(connection.getHandle()).getStmtHandle();
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  @Override
  public ResultSet executeQuery(String sql) throws SQLException {
    checkClosed();
    return executeQueryWithBindings(sql, (PreparedStatementBindingSerializer.NativeBindings) null);
  }

  protected ResultSet executeQueryWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) throws SQLException {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    applyExecuteQueryResult(response);
    return currentResultSet;
  }

  protected int executeUpdateWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) throws SQLException {
    ensureNoResultSet(executeWithBindings(sql, bindings), "executeUpdate");
    return getCurrentUpdateCountAsInt();
  }

  protected long executeLargeUpdateWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) throws SQLException {
    ensureNoResultSet(executeWithBindings(sql, bindings), "executeLargeUpdate");
    return currentUpdateCount;
  }

  /**
   * Enforce the JDBC contract for {@code executeUpdate}/{@code executeLargeUpdate}: the spec
   * requires throwing when the SQL produced a {@link ResultSet}. The query has already been
   * executed at this point, but we cannot detect ResultSet-vs-update-count up front — Snowflake's
   * SQL surface (dynamic SQL, multi-statements) means the server's response is the only authority.
   * snowflake-jdbc throws here too (see {@code executeUpdateInternal} → {@code
   * UNSUPPORTED_STATEMENT_TYPE_IN_EXECUTION_API}); silently swallowing would let {@code
   * executeUpdate("SELECT …")} return 0 and surprise callers.
   */
  private static void ensureNoResultSet(boolean producedResultSet, String methodName)
      throws SQLException {
    if (producedResultSet) {
      throw new SnowflakeSQLException(
          methodName + "() cannot be used for statements that produce a ResultSet");
    }
  }

  protected boolean executeWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) throws SQLException {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    return updateExecutionStateAndReturnHasResultSet(response);
  }

  private ExecuteQueryResponse executeStatement(
      String sql, PreparedStatementBindingSerializer.NativeBindings nativeBindings)
      throws SQLException {
    QueryBindings bindings = nativeBindings != null ? nativeBindings.bindings() : null;
    boolean hasBindings = bindings != null;
    logger.debug("Statement executeWithBindings start: sql={}, hasBindings={}", sql, hasBindings);
    prepareForExecution();
    coreDriverApi.statementSetSqlQuery(statementHandle, sql);
    // PreparedStatement callers must wrap this in try-with-resources on the NativeBindings so
    // the embedded native pointer remains valid across the synchronous RPC (JLS §12.6.1).
    ExecuteQueryResponse response;
    try {
      response = coreDriverApi.statementExecuteQuery(statementHandle, bindings);
    } catch (SQLException e) {
      // Mirror snowflake-jdbc: surface the server-side queryId on a failed execute so callers
      // can correlate the error with a Snowflake history entry.
      captureQueryIdFromException(e);
      throw e;
    }
    logger.debug("statementExecuteQuery succeeded: hasBindings={}", hasBindings);
    return response;
  }

  private void captureQueryIdFromException(SQLException e) {
    // prepareForExecution() already cleared queryId; only overwrite when the server surfaced one.
    if (e instanceof SnowflakeSQLException) {
      String exceptionQueryId = ((SnowflakeSQLException) e).getQueryId();
      if (!StringUtil.isNullOrEmpty(exceptionQueryId)) {
        this.queryId = exceptionQueryId;
      }
    }
  }

  private ResultSetResponse fetchResultSetByQueryId(String queryId) throws SQLException {
    return coreDriverApi.connectionGetResultSet(connection.getHandle(), queryId);
  }

  private void applyExecuteQueryResult(ExecuteQueryResponse response) throws SQLException {
    if (response.hasMulti()) {
      applyMultiStatementResult(response.getMulti());
      return;
    }
    applySingleResult(response.getSingle(), true);
  }

  private boolean updateExecutionStateAndReturnHasResultSet(ExecuteQueryResponse response)
      throws SQLException {
    if (response.hasMulti()) {
      applyMultiStatementResult(response.getMulti());
      return currentResultSet != null;
    }
    return applySingleResult(response.getSingle(), false);
  }

  /**
   * Apply a single-statement result: set queryId, fetch stream, update
   * currentResultSet/updateCount.
   *
   * @param forceResultSet when true (executeQuery path), surface a ResultSet even for DML/DDL if
   *     the server provides a non-empty stream.
   * @return true if the result produced a ResultSet.
   */
  private boolean applySingleResult(ResultSetResponse rsResponse, boolean forceResultSet)
      throws SQLException {
    ResultSetDescriptor descriptor = rsResponse.getResultDescriptor();
    queryId = descriptor.getQueryId();

    if (StatementTypeClassifier.producesResultSet(descriptor)) {
      currentResultSet = ResultSetFactory.create(coreDriverApi, this, queryId, rsResponse);
      currentUpdateCount = NO_UPDATE_COUNT;
      return true;
    }

    if (forceResultSet) {
      // DML/DDL that returned via executeQuery() — still surface a ResultSet if the server
      // provides a stream (matches old JDBC driver behavior)
      InternalResultSet maybeResultSet =
          ResultSetFactory.createIfHasStream(coreDriverApi, this, queryId, rsResponse);
      if (maybeResultSet != null) {
        currentResultSet = maybeResultSet;
        currentUpdateCount = NO_UPDATE_COUNT;
        return true;
      }
    }

    currentResultSet = null;
    currentUpdateCount = StatementTypeClassifier.getUpdateCount(descriptor);
    return false;
  }

  private void applyMultiStatementResult(MultiStatementResult multi) throws SQLException {
    multiState = MultiStatementState.from(multi);
    queryId = multiState.getParentQueryId();
    if (multiState.isEmpty()) {
      currentResultSet = null;
      currentUpdateCount = NO_UPDATE_COUNT;
      return;
    }
    advanceToNextChild();
  }

  private void advanceToNextChild() throws SQLException {
    String childQueryId = multiState.advance();
    int index = multiState.currentIndex();

    ResultSetResponse rsResponse = fetchResultSetByQueryId(childQueryId);
    ResultSetDescriptor descriptor = rsResponse.getResultDescriptor();

    boolean producesResultSet;
    if (multiState.hasStatementTypeFor(index)) {
      producesResultSet = multiState.producesResultSet(index);
    } else {
      producesResultSet = StatementTypeClassifier.producesResultSet(descriptor);
    }

    if (producesResultSet) {
      currentResultSet = ResultSetFactory.create(coreDriverApi, this, childQueryId, rsResponse);
      currentUpdateCount = NO_UPDATE_COUNT;
    } else {
      currentResultSet = null;
      currentUpdateCount = StatementTypeClassifier.getUpdateCount(descriptor);
    }
  }

  private void resetExecutionState() {
    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
    queryId = null;
    multiState = null;
  }

  private void prepareForExecution() throws SQLException {
    if (currentResultSet != null && !currentResultSet.isClosed()) {
      openResultSets.add(currentResultSet);
    }
    resetExecutionState();
  }

  private void clearExecutionState() throws SQLException {
    closeCurrentResultSet();
    for (ResultSet resultSet : openResultSets) {
      closeResultSet(resultSet);
    }
    openResultSets.clear();
    resetExecutionState();
  }

  protected void closeCurrentResultSet() throws SQLException {
    closeResultSet(currentResultSet);
  }

  public void removeClosedResultSet(ResultSet resultSet) {
    openResultSets.remove(resultSet);
  }

  private void closeResultSet(ResultSet resultSet) throws SQLException {
    if (resultSet != null && !resultSet.isClosed()) {
      resultSet.close();
    }
  }

  private int getCurrentUpdateCountAsInt() throws SQLException {
    return toJdbcIntUpdateCount(currentUpdateCount);
  }

  private int toJdbcIntUpdateCount(long updateCount) throws SQLException {
    if (updateCount == NO_UPDATE_COUNT) {
      return (int) NO_UPDATE_COUNT;
    }
    try {
      return Math.toIntExact(updateCount);
    } catch (ArithmeticException e) {
      throw new SnowflakeSQLException("Update count exceeds JDBC int range", e);
    }
  }

  @Override
  public int executeUpdate(String sql) throws SQLException {
    checkClosed();
    return executeUpdateWithBindings(sql, null);
  }

  @Override
  public void close() throws SQLException {
    if (!closed.compareAndSet(false, true)) {
      return;
    }
    clearExecutionState();
    try {
      coreDriverApi.statementRelease(statementHandle);
    } catch (SQLException e) {
      logger.debug("Error releasing statement handle", e);
    }
    connection.removeStatement(this);
  }

  @Override
  public int getMaxFieldSize() throws SQLException {
    checkClosed();
    return 0; // No limit in stub implementation
  }

  @Override
  public void setMaxFieldSize(int max) throws SQLException {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public int getMaxRows() throws SQLException {
    checkClosed();
    return maxRows;
  }

  @Override
  public void setMaxRows(int max) throws SQLException {
    checkClosed();
    this.maxRows = max;
  }

  @Override
  public void setEscapeProcessing(boolean enable) throws SQLException {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public int getQueryTimeout() throws SQLException {
    checkClosed();
    return queryTimeout;
  }

  @Override
  public void setQueryTimeout(int seconds) throws SQLException {
    checkClosed();
    this.queryTimeout = seconds;
  }

  @Override
  public void cancel() throws SQLException {
    checkClosed();
    // Stub implementation - no cancellation logic
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    checkClosed();
    return null;
  }

  @Override
  public void clearWarnings() throws SQLException {
    checkClosed();
    // Stub implementation - no warnings to clear
  }

  @Override
  public void setCursorName(String name) throws SQLException {
    throw new SQLFeatureNotSupportedException("setCursorName not supported");
  }

  @Override
  public boolean execute(String sql) throws SQLException {
    checkClosed();
    return executeWithBindings(sql, null);
  }

  @Override
  public ResultSet getResultSet() throws SQLException {
    checkClosed();
    return currentResultSet;
  }

  @Override
  public int getUpdateCount() throws SQLException {
    checkClosed();
    return getCurrentUpdateCountAsInt();
  }

  @Override
  public boolean getMoreResults() throws SQLException {
    return getMoreResults(Statement.CLOSE_CURRENT_RESULT);
  }

  @Override
  public void setFetchDirection(int direction) throws SQLException {
    checkClosed();
    if (direction != ResultSet.FETCH_FORWARD) {
      throw new SQLFeatureNotSupportedException("Only FETCH_FORWARD supported");
    }
  }

  @Override
  public int getFetchDirection() throws SQLException {
    checkClosed();
    return ResultSet.FETCH_FORWARD;
  }

  @Override
  public void setFetchSize(int rows) throws SQLException {
    checkClosed();
    this.fetchSize = rows;
  }

  @Override
  public int getFetchSize() throws SQLException {
    checkClosed();
    return fetchSize;
  }

  @Override
  public int getResultSetConcurrency() throws SQLException {
    checkClosed();
    return ResultSet.CONCUR_READ_ONLY;
  }

  @Override
  public int getResultSetType() throws SQLException {
    checkClosed();
    return ResultSet.TYPE_FORWARD_ONLY;
  }

  @Override
  public void addBatch(String sql) throws SQLException {
    checkClosed();
    if (sql == null) {
      throw new SnowflakeSQLException("addBatch requires a non-null SQL string");
    }
    batch.add(sql);
  }

  @Override
  public void clearBatch() throws SQLException {
    checkClosed();
    batch.clear();
    // batchQueryIds is intentionally not cleared here — see SnowflakeStatement#getBatchQueryIDs.
  }

  // TODO: honour CLIENT_CLEAR_BATCH_ONLY_AFTER_SUCCESSFUL_EXECUTION once sf_core surfaces session
  // params; today we always clear in finally.
  @Override
  public int[] executeBatch() throws SQLException {
    checkClosed();
    return batch.executeAll(this);
  }

  protected static BatchUpdateException buildBatchFailureException(
      SQLException firstFailure, int[] updateCounts) {
    return new BatchUpdateException(
        firstFailure.getLocalizedMessage(),
        firstFailure.getSQLState(),
        firstFailure.getErrorCode(),
        updateCounts,
        firstFailure);
  }

  /**
   * Shared cleanup for executeBatch paths: clear the batch and reset current-result state. If a BUE
   * is pending from the catch path, attach cleanup failures as suppressed; on the success path, log
   * + swallow so cleanup errors don't mask successful update counts.
   */
  protected void finalizeBatch(BatchUpdateException pending) {
    try {
      clearBatch();
    } catch (SQLException cleanupEx) {
      if (pending != null) {
        pending.addSuppressed(cleanupEx);
      } else {
        logger.warn("clearBatch failed after successful executeBatch", cleanupEx);
      }
    }
    resetCurrentResultState();
  }

  /**
   * Map a long update count to a JDBC batch int. {@code NO_UPDATE_COUNT} and out-of-int values
   * collapse to {@link Statement#SUCCESS_NO_INFO}; callers wanting full fidelity should use {@link
   * Statement#executeLargeBatch()}.
   */
  protected static int toBatchInt(long value) {
    if (value == NO_UPDATE_COUNT || value > Integer.MAX_VALUE || value < Integer.MIN_VALUE) {
      return Statement.SUCCESS_NO_INFO;
    }
    return (int) value;
  }

  /** Both {@code null} and the empty string (proto3 default) collapse to {@code null}. */
  protected static String normalizeQueryId(String queryId) {
    return StringUtil.isNullOrEmpty(queryId) ? null : queryId;
  }

  protected void recordBatchQueryId() {
    batchQueryIds.add(normalizeQueryId(queryId));
  }

  protected void clearBatchQueryIds() {
    batchQueryIds.clear();
  }

  /**
   * Per JDBC spec, after {@code executeBatch()} {@link Statement#getResultSet()} returns null and
   * {@link Statement#getUpdateCount()} returns -1.
   */
  protected void resetCurrentResultState() {
    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
  }

  @Override
  public Connection getConnection() throws SQLException {
    checkClosed();
    return connection;
  }

  /**
   * Returns the owning connection without a closed-state check. Used during ResultSet construction,
   * which must succeed even if the statement was concurrently closed after {@code execute()}
   * returned (parity with legacy snowflake-jdbc, which does not re-validate the statement while
   * assembling the result set). Callers must not use this to bypass {@link #checkClosed()} on the
   * public API path.
   */
  public InternalSnowflakeConnection getConnectionInternal() {
    return connection;
  }

  public Set<ResultSet> getOpenResultSets() {
    return Collections.unmodifiableSet(openResultSets);
  }

  @Override
  public boolean getMoreResults(int current) throws SQLException {
    checkClosed();
    if (current == Statement.CLOSE_CURRENT_RESULT || current == Statement.CLOSE_ALL_RESULTS) {
      closeCurrentResultSet();
    }
    if (current == Statement.CLOSE_ALL_RESULTS) {
      for (ResultSet resultSet : openResultSets) {
        closeResultSet(resultSet);
      }
      openResultSets.clear();
    }
    if (current == Statement.KEEP_CURRENT_RESULT && currentResultSet != null) {
      openResultSets.add(currentResultSet);
    }

    if (multiState == null || !multiState.hasMore()) {
      currentResultSet = null;
      currentUpdateCount = NO_UPDATE_COUNT;
      return false;
    }

    advanceToNextChild();

    if (currentResultSet != null) {
      return true;
    }
    return multiState.hasMore();
  }

  @Override
  public ResultSet getGeneratedKeys() throws SQLException {
    throw new SQLFeatureNotSupportedException("getGeneratedKeys not supported");
  }

  @Override
  public int executeUpdate(String sql, int autoGeneratedKeys) throws SQLException {
    return executeUpdate(sql);
  }

  @Override
  public int executeUpdate(String sql, int[] columnIndexes) throws SQLException {
    return executeUpdate(sql);
  }

  @Override
  public int executeUpdate(String sql, String[] columnNames) throws SQLException {
    return executeUpdate(sql);
  }

  @Override
  public boolean execute(String sql, int autoGeneratedKeys) throws SQLException {
    return execute(sql);
  }

  @Override
  public boolean execute(String sql, int[] columnIndexes) throws SQLException {
    return execute(sql);
  }

  @Override
  public boolean execute(String sql, String[] columnNames) throws SQLException {
    return execute(sql);
  }

  @Override
  public int getResultSetHoldability() throws SQLException {
    checkClosed();
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed.get();
  }

  @Override
  public void setPoolable(boolean poolable) throws SQLException {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public boolean isPoolable() throws SQLException {
    checkClosed();
    return false;
  }

  @Override
  public void closeOnCompletion() throws SQLException {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public boolean isCloseOnCompletion() throws SQLException {
    checkClosed();
    return false;
  }

  protected void checkClosed() throws SQLException {
    if (isClosed()) {
      throw new SQLException("Statement is closed");
    }
    if (connection.isClosed()) {
      throw new SQLException("Connection is closed");
    }
  }

  @Override
  public String getQueryID() throws SQLException {
    checkClosed();
    return queryId;
  }

  @Override
  public List<String> getBatchQueryIDs() throws SQLException {
    checkClosed();
    return Collections.unmodifiableList(new ArrayList<>(batchQueryIds));
  }

  @Override
  public void setParameter(String name, Object value) throws SQLException {
    checkClosed();
    ConfigSetting.Builder settingBuilder = ConfigSetting.newBuilder();
    if (value instanceof Number) {
      settingBuilder.setIntValue(((Number) value).longValue());
    } else {
      settingBuilder.setStringValue(String.valueOf(value));
    }
    coreDriverApi.statementSetOptions(
        statementHandle, Collections.singletonMap(name, settingBuilder.build()));
  }

  @Override
  public void setBatchID(String batchID) {
    throw new RuntimeException("setBatchID not supported"); // no throws SQLException
  }

  @Override
  public ResultSet executeAsyncQuery(String sql) throws SQLException {
    checkClosed();
    return executeAsyncQueryWithBindings(sql, null);
  }

  protected ResultSet executeAsyncQueryWithBindings(String sql, QueryBindings bindings)
      throws SQLException {
    prepareForExecution();
    coreDriverApi.statementSetSqlQuery(statementHandle, sql);
    StatementExecuteAsyncResponse response =
        coreDriverApi.statementExecuteAsync(statementHandle, bindings);
    String asyncQueryId = response.getQueryId();
    queryId = asyncQueryId;
    ResultSet asyncResultSet = ResultSetFactory.createAsync(asyncQueryId, connection, this, false);
    currentResultSet = asyncResultSet;
    return asyncResultSet;
  }

  @Override
  public void setAsyncQueryTimeout(int seconds) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsyncQueryTimeout not supported");
  }
}
