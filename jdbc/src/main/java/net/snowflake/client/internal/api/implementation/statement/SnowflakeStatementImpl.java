package net.snowflake.client.internal.api.implementation.statement;

import static net.snowflake.client.internal.api.implementation.statement.StatementTypeClassifier.NO_UPDATE_COUNT;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.Statement;
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
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;

public class SnowflakeStatementImpl implements Statement, SnowflakeStatement {
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
    return executeQueryWithBindings(sql, null);
  }

  protected ResultSet executeQueryWithBindings(String sql, QueryBindings bindings)
      throws SQLException {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    applyExecuteQueryResult(response);
    return currentResultSet;
  }

  protected int executeUpdateWithBindings(String sql, QueryBindings bindings) throws SQLException {
    boolean producedResultSet = executeWithBindings(sql, bindings);
    if (producedResultSet) {
      throw new SnowflakeSQLException(
          "executeUpdate() cannot be used for statements that produce a ResultSet");
    }
    return getCurrentUpdateCountAsInt();
  }

  protected boolean executeWithBindings(String sql, QueryBindings bindings) throws SQLException {
    checkClosed();
    ExecuteQueryResponse response = executeStatement(sql, bindings);
    return updateExecutionStateAndReturnHasResultSet(response);
  }

  private ExecuteQueryResponse executeStatement(String sql, QueryBindings bindings)
      throws SQLException {
    boolean hasBindings = bindings != null;
    logger.debug("Statement executeWithBindings start: sql={}, hasBindings={}", sql, hasBindings);
    prepareForExecution();
    coreDriverApi.statementSetSqlQuery(statementHandle, sql);
    ExecuteQueryResponse response = coreDriverApi.statementExecuteQuery(statementHandle, bindings);
    logger.debug("statementExecuteQuery succeeded: hasBindings={}", hasBindings);
    return response;
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
      currentResultSet =
          ResultSetFactory.create(coreDriverApi, this, rsResponse.getResultSetHandle());
      currentUpdateCount = NO_UPDATE_COUNT;
      return true;
    }

    if (forceResultSet) {
      // DML/DDL that returned via executeQuery() — still surface a ResultSet if the server
      // provides a stream (matches old JDBC driver behavior)
      InternalResultSet maybeResultSet =
          ResultSetFactory.createIfHasStream(coreDriverApi, this, rsResponse.getResultSetHandle());
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
      currentResultSet =
          ResultSetFactory.create(coreDriverApi, this, rsResponse.getResultSetHandle());
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
    throw new SQLFeatureNotSupportedException("addBatch not supported");
  }

  @Override
  public void clearBatch() throws SQLException {
    throw new SQLFeatureNotSupportedException("clearBatch not supported");
  }

  @Override
  public int[] executeBatch() throws SQLException {
    throw new SQLFeatureNotSupportedException("executeBatch not supported");
  }

  @Override
  public Connection getConnection() throws SQLException {
    checkClosed();
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

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    if (iface.isAssignableFrom(getClass())) {
      return iface.cast(this);
    }
    throw new SQLException("Cannot unwrap to " + iface.getName());
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    return iface.isAssignableFrom(getClass());
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
    throw new SQLFeatureNotSupportedException("getBatchQueryIDs not supported");
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
    throw new SQLFeatureNotSupportedException("executeAsyncQuery not supported");
  }

  @Override
  public void setAsyncQueryTimeout(int seconds) throws SQLException {
    throw new SQLFeatureNotSupportedException("setAsyncQueryTimeout not supported");
  }
}
