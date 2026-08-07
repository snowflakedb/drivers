package net.snowflake.client.internal.api.implementation.statement;

import static net.snowflake.client.internal.api.implementation.statement.StatementTypeClassifier.NO_UPDATE_COUNT;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLWarning;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.MultiStatementResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.NotImplementedException;
import net.snowflake.client.internal.util.StringUtil;

@JdbcBoundary
public class SnowflakeStatementImpl implements InternalStatement, DelegatingWrapper {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeStatementImpl.class);

  protected final InternalSnowflakeConnection connection;
  protected final CoreDriverApi coreDriverApi;

  private final AtomicBoolean closed = new AtomicBoolean(false);
  protected int maxRows = 0;
  protected int queryTimeout = 0;
  protected int fetchSize = 0;
  protected StatementHandle statementHandle;
  protected InternalResultSet currentResultSet;
  /**
   * Cached decorated view of {@link #currentResultSet}, so {@code executeQuery()} and a later
   * {@code getResultSet()} hand back the same wrapper (legacy JDBC {@code ==} contract). Nulled by
   * {@link #setCurrentResultSet(InternalResultSet)}.
   */
  private ResultSet currentDecoratedResultSet;

  protected long currentUpdateCount = NO_UPDATE_COUNT;
  protected String queryId;
  protected final Set<InternalResultSet> openResultSets = ConcurrentHashMap.newKeySet();
  private final StatementBatch batch = new StatementBatch();
  /** Per-batch-entry query IDs collected during {@link #executeBatch()}. */
  private final List<String> batchQueryIds = new ArrayList<>();
  /** Non-null only while navigating a multi-statement result set sequence. */
  private MultiStatementState multiState;

  public SnowflakeStatementImpl(
      InternalSnowflakeConnection connection, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.coreDriverApi = coreDriverApi;
    this.statementHandle = coreDriverApi.statementNew(connection.getHandle()).getStmtHandle();
  }

  @Override
  public ResultSet executeQuery(String sql) {
    checkClosed();
    executeQueryWithBindings(sql, null);
    return decoratedCurrentResultSet();
  }

  @Override
  public InternalResultSet executeQueryInternal(String sql) {
    checkClosed();
    return executeQueryWithBindings(sql, null);
  }

  protected InternalResultSet executeQueryWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    applyExecuteQueryResult(response);
    return currentResultSet;
  }

  /** Single mutation point for {@link #currentResultSet}; invalidates the cached decorated view. */
  protected void setCurrentResultSet(InternalResultSet resultSet) {
    this.currentResultSet = resultSet;
    this.currentDecoratedResultSet = null;
  }

  /** Lazily builds and caches the decorated boundary view of {@link #currentResultSet}. */
  protected ResultSet decoratedCurrentResultSet() {
    if (currentResultSet == null) {
      return null;
    }
    if (currentDecoratedResultSet == null) {
      currentDecoratedResultSet =
          Decorators.resultSet(currentResultSet, getConnectionInternal().getTelemetry());
    }
    return currentDecoratedResultSet;
  }

  protected int executeUpdateWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) {
    ensureNoResultSet(executeWithBindings(sql, bindings), "executeUpdate", queryId);
    return getCurrentUpdateCountAsInt();
  }

  protected long executeLargeUpdateWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) {
    ensureNoResultSet(executeWithBindings(sql, bindings), "executeLargeUpdate", queryId);
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
  private static void ensureNoResultSet(
      boolean producedResultSet, String methodName, String queryId) {
    if (producedResultSet) {
      throw new SFSQLException(
              methodName + "() cannot be used for statements that produce a ResultSet")
          .withQueryId(queryId);
    }
  }

  protected boolean executeWithBindings(
      String sql, PreparedStatementBindingSerializer.NativeBindings bindings) {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    return updateExecutionStateAndReturnHasResultSet(response);
  }

  private ExecuteQueryResponse executeStatement(
      String sql, PreparedStatementBindingSerializer.NativeBindings nativeBindings) {
    QueryBindings bindings = nativeBindings != null ? nativeBindings.bindings() : null;

    boolean hasBindings = bindings != null;
    logger.debug("Statement executeWithBindings start: hasBindings={}", hasBindings);
    ParametersRegistry parameters = connection.getParameters();
    if (logger.isInfoEnabled()
        && parameters != null
        && parameters.getBool(Parameter.LOG_QUERY_TEXT)) {
      logger.info("query: [{}]", sql);
    }

    prepareForExecution();
    coreDriverApi.statementSetSqlQuery(statementHandle, sql);
    // PreparedStatement callers must wrap this in try-with-resources on the NativeBindings so
    // the embedded native pointer remains valid across the synchronous RPC (JLS §12.6.1).
    ExecuteQueryResponse response;
    try {
      response = coreDriverApi.statementExecuteQuery(statementHandle, bindings);
    } catch (CoreException e) {
      // Mirror snowflake-jdbc: surface the server-side queryId on a failed execute so callers
      // can correlate the error with a Snowflake history entry.
      captureQueryIdFromException(e);
      throw e;
    }
    logger.debug("statementExecuteQuery succeeded: hasBindings={}", hasBindings);
    return response;
  }

  private void captureQueryIdFromException(CoreException e) {
    // prepareForExecution() already cleared queryId; only overwrite when the server surfaced one.
    if (e != null) {
      String exceptionQueryId = e.getQueryId();
      if (!StringUtil.isNullOrEmpty(exceptionQueryId)) {
        this.queryId = exceptionQueryId;
      }
    }
  }

  private ResultSetResponse fetchResultSetByQueryId(String queryId) {
    return coreDriverApi.connectionGetResultSet(connection.getHandle(), queryId);
  }

  private void applyExecuteQueryResult(ExecuteQueryResponse response) {
    if (response.hasMulti()) {
      applyMultiStatementResult(response.getMulti());
      return;
    }
    applySingleResult(response.getSingle(), true);
  }

  private boolean updateExecutionStateAndReturnHasResultSet(ExecuteQueryResponse response) {
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
  private boolean applySingleResult(ResultSetResponse rsResponse, boolean forceResultSet) {
    ResultSetDescriptor descriptor = rsResponse.getResultDescriptor();
    queryId = descriptor.getQueryId();

    if (StatementTypeClassifier.producesResultSet(descriptor)) {
      setCurrentResultSet(ResultSetFactory.create(coreDriverApi, this, queryId, rsResponse));
      currentUpdateCount = NO_UPDATE_COUNT;
      return true;
    }

    if (forceResultSet) {
      // DML/DDL that returned via executeQuery() — still surface a ResultSet if the server
      // provides a stream (matches old JDBC driver behavior)
      InternalResultSet maybeResultSet =
          ResultSetFactory.createIfHasStream(coreDriverApi, this, queryId, rsResponse);
      if (maybeResultSet != null) {
        setCurrentResultSet(maybeResultSet);
        currentUpdateCount = NO_UPDATE_COUNT;
        return true;
      }
    }

    setCurrentResultSet(null);
    currentUpdateCount = StatementTypeClassifier.getUpdateCount(descriptor);
    return false;
  }

  private void applyMultiStatementResult(MultiStatementResult multi) {
    multiState = MultiStatementState.from(multi);
    queryId = multiState.getParentQueryId();
    if (multiState.isEmpty()) {
      setCurrentResultSet(null);
      currentUpdateCount = NO_UPDATE_COUNT;
      return;
    }
    advanceToNextChild();
  }

  private void advanceToNextChild() {
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
      setCurrentResultSet(ResultSetFactory.create(coreDriverApi, this, childQueryId, rsResponse));
      currentUpdateCount = NO_UPDATE_COUNT;
    } else {
      setCurrentResultSet(null);
      currentUpdateCount = StatementTypeClassifier.getUpdateCount(descriptor);
    }
  }

  private void resetExecutionState() {
    setCurrentResultSet(null);
    currentUpdateCount = NO_UPDATE_COUNT;
    queryId = null;
    multiState = null;
  }

  private void prepareForExecution() {
    if (currentResultSet != null && !currentResultSet.isClosed()) {
      openResultSets.add(currentResultSet);
    }
    resetExecutionState();
  }

  private void clearExecutionState() {
    closeCurrentResultSet();
    for (InternalResultSet resultSet : openResultSets) {
      closeResultSet(resultSet);
    }
    openResultSets.clear();
    resetExecutionState();
  }

  protected void closeCurrentResultSet() {
    closeResultSet(currentResultSet);
  }

  public void removeClosedResultSet(ResultSet resultSet) {
    openResultSets.remove(resultSet);
  }

  private void closeResultSet(InternalResultSet resultSet) {
    if (resultSet != null && !resultSet.isClosed()) {
      resultSet.close();
    }
  }

  private int getCurrentUpdateCountAsInt() {
    return toJdbcIntUpdateCount(currentUpdateCount);
  }

  private int toJdbcIntUpdateCount(long updateCount) {
    if (updateCount == NO_UPDATE_COUNT) {
      return (int) NO_UPDATE_COUNT;
    }
    try {
      return Math.toIntExact(updateCount);
    } catch (ArithmeticException e) {
      throw new SFSQLException("Update count exceeds JDBC int range", e).withQueryId(queryId);
    }
  }

  @Override
  public int executeUpdate(String sql) {
    checkClosed();
    return executeUpdateWithBindings(sql, null);
  }

  @Override
  public void close() {
    if (!closed.compareAndSet(false, true)) {
      return;
    }
    clearExecutionState();
    try {
      coreDriverApi.statementRelease(statementHandle);
    } catch (CoreException e) {
      logger.debug("Error releasing statement handle", e);
    }
    connection.removeStatement(this);
  }

  @Override
  public int getMaxFieldSize() {
    checkClosed();
    return 0; // No limit in stub implementation
  }

  @Override
  public void setMaxFieldSize(int max) {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public int getMaxRows() {
    checkClosed();
    return maxRows;
  }

  @Override
  public void setMaxRows(int max) {
    checkClosed();
    this.maxRows = max;
  }

  @Override
  public void setEscapeProcessing(boolean enable) {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public int getQueryTimeout() {
    checkClosed();
    return queryTimeout;
  }

  @Override
  public void setQueryTimeout(int seconds) {
    checkClosed();
    this.queryTimeout = seconds;
  }

  @Override
  public void cancel() {
    checkClosed();
    // Stub implementation - no cancellation logic
  }

  @Override
  public SQLWarning getWarnings() {
    checkClosed();
    return null;
  }

  @Override
  public void clearWarnings() {
    checkClosed();
    // Stub implementation - no warnings to clear
  }

  @Override
  public void setCursorName(String name) {
    throw new SFSQLFeatureNotSupportedException("setCursorName not supported");
  }

  @Override
  public boolean execute(String sql) {
    checkClosed();
    return executeWithBindings(sql, null);
  }

  @Override
  public ResultSet getResultSet() {
    checkClosed();
    return decoratedCurrentResultSet();
  }

  @Override
  public int getUpdateCount() {
    checkClosed();
    return getCurrentUpdateCountAsInt();
  }

  @Override
  public boolean getMoreResults() {
    return getMoreResults(Statement.CLOSE_CURRENT_RESULT);
  }

  @Override
  public void setFetchDirection(int direction) {
    checkClosed();
    if (direction != ResultSet.FETCH_FORWARD) {
      throw new SFSQLFeatureNotSupportedException("Only FETCH_FORWARD supported");
    }
  }

  @Override
  public int getFetchDirection() {
    checkClosed();
    return ResultSet.FETCH_FORWARD;
  }

  @Override
  public void setFetchSize(int rows) {
    checkClosed();
    this.fetchSize = rows;
  }

  @Override
  public int getFetchSize() {
    checkClosed();
    return fetchSize;
  }

  @Override
  public int getResultSetConcurrency() {
    checkClosed();
    return ResultSet.CONCUR_READ_ONLY;
  }

  @Override
  public int getResultSetType() {
    checkClosed();
    return ResultSet.TYPE_FORWARD_ONLY;
  }

  @Override
  public void addBatch(String sql) {
    checkClosed();
    if (sql == null) {
      throw new SFSQLException("addBatch requires a non-null SQL string");
    }
    batch.add(sql);
  }

  @Override
  public void clearBatch() {
    checkClosed();
    batch.clear();
    // batchQueryIds is intentionally not cleared here — see SnowflakeStatement#getBatchQueryIDs.
  }

  // TODO: honour CLIENT_CLEAR_BATCH_ONLY_AFTER_SUCCESSFUL_EXECUTION once sf_core surfaces session
  // params; today we always clear in finally.
  @Override
  public int[] executeBatch() {
    checkClosed();
    return batch.executeAll(this);
  }

  protected static SFBatchUpdateException buildBatchFailureException(
      CoreException firstFailure, int[] updateCounts) {
    // Decompose the DriverException the same way SnowflakeSQLException does (null/0 when absent,
    // e.g. transport failures) so the BatchUpdateException keeps the core's SQLState / vendor code.
    DriverException error = firstFailure.getError();
    String sqlState = (error != null && error.hasSqlState()) ? error.getSqlState() : null;
    int vendorCode = (error != null && error.hasVendorCode()) ? error.getVendorCode() : 0;
    return new SFBatchUpdateException(
        firstFailure.getLocalizedMessage(), sqlState, vendorCode, updateCounts, firstFailure);
  }

  /**
   * Shared cleanup for executeBatch paths: clear the batch and reset current-result state. If a BUE
   * is pending from the catch path, attach cleanup failures as suppressed; on the success path, log
   * + swallow so cleanup errors don't mask successful update counts.
   */
  protected void finalizeBatch(SFBatchUpdateException pending) {
    try {
      clearBatch();
    } catch (Exception cleanupEx) {
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
  public Connection getConnection() {
    checkClosed();
    return Decorators.connection(connection, connection.getTelemetry());
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
  public boolean getMoreResults(int current) {
    checkClosed();
    if (current == Statement.CLOSE_CURRENT_RESULT || current == Statement.CLOSE_ALL_RESULTS) {
      closeCurrentResultSet();
    }
    if (current == Statement.CLOSE_ALL_RESULTS) {
      for (InternalResultSet resultSet : openResultSets) {
        closeResultSet(resultSet);
      }
      openResultSets.clear();
    }
    if (current == Statement.KEEP_CURRENT_RESULT && currentResultSet != null) {
      openResultSets.add(currentResultSet);
    }

    if (multiState == null || !multiState.hasMore()) {
      setCurrentResultSet(null);
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
  public ResultSet getGeneratedKeys() {
    throw new SFSQLFeatureNotSupportedException("getGeneratedKeys not supported");
  }

  @Override
  public int executeUpdate(String sql, int autoGeneratedKeys) {
    return executeUpdate(sql);
  }

  @Override
  public int executeUpdate(String sql, int[] columnIndexes) {
    return executeUpdate(sql);
  }

  @Override
  public int executeUpdate(String sql, String[] columnNames) {
    return executeUpdate(sql);
  }

  @Override
  public boolean execute(String sql, int autoGeneratedKeys) {
    return execute(sql);
  }

  @Override
  public boolean execute(String sql, int[] columnIndexes) {
    return execute(sql);
  }

  @Override
  public boolean execute(String sql, String[] columnNames) {
    return execute(sql);
  }

  @Override
  public int getResultSetHoldability() {
    checkClosed();
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public boolean isClosed() {
    return closed.get();
  }

  @Override
  public void setPoolable(boolean poolable) {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public boolean isPoolable() {
    checkClosed();
    return false;
  }

  @Override
  public void closeOnCompletion() {
    checkClosed();
    // Stub implementation - ignore
  }

  @Override
  public boolean isCloseOnCompletion() {
    checkClosed();
    return false;
  }

  protected void checkClosed() {
    if (isClosed()) {
      throw new SFSQLException("Statement is closed");
    }
    if (connection.isClosed()) {
      throw new SFSQLException("Connection is closed");
    }
  }

  @Override
  public String getQueryID() {
    checkClosed();
    return queryId;
  }

  @Override
  public List<String> getBatchQueryIDs() {
    checkClosed();
    return Collections.unmodifiableList(new ArrayList<>(batchQueryIds));
  }

  @Override
  public void setParameter(String name, Object value) {
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
    // A not-yet-implemented gap (legacy snowflake-jdbc stores the batch id), not a by-design
    // unsupported feature — hence NotImplementedException, not SFSQLFeatureNotSupportedException.
    throw new NotImplementedException("setBatchID not supported");
  }

  @Override
  public ResultSet executeAsyncQuery(String sql) {
    checkClosed();
    return executeAsyncQueryWithBindings(sql, null);
  }

  protected ResultSet executeAsyncQueryWithBindings(String sql, QueryBindings bindings) {
    prepareForExecution();
    coreDriverApi.statementSetSqlQuery(statementHandle, sql);
    StatementExecuteAsyncResponse response =
        coreDriverApi.statementExecuteAsync(statementHandle, bindings);
    String asyncQueryId = response.getQueryId();
    queryId = asyncQueryId;
    InternalResultSet asyncResultSet =
        ResultSetFactory.createAsync(asyncQueryId, connection, this, false);
    setCurrentResultSet(asyncResultSet);
    return decoratedCurrentResultSet();
  }

  @Override
  public void setAsyncQueryTimeout(int seconds) {
    throw new SFSQLFeatureNotSupportedException("setAsyncQueryTimeout not supported");
  }
}
