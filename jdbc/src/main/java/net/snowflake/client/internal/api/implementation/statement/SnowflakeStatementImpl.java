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
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteQueryRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetSqlQueryRequest;

public class SnowflakeStatementImpl implements Statement, SnowflakeStatement {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeStatementImpl.class);

  protected final SnowflakeConnectionImpl connection;
  protected boolean closed = false;
  protected int maxRows = 0;
  protected int queryTimeout = 0;
  protected int fetchSize = 0;
  protected StatementHandle statementHandle;
  protected ResultSet currentResultSet;
  protected long currentUpdateCount = NO_UPDATE_COUNT;
  protected String queryId;
  protected final Set<ResultSet> openResultSets = ConcurrentHashMap.newKeySet();

  public SnowflakeStatementImpl(SnowflakeConnectionImpl connection) {
    this.connection = connection;
    StatementNewRequest statementNewRequest =
        StatementNewRequest.newBuilder().setConnHandle(connection.connectionHandle).build();
    try {
      this.statementHandle =
          ProtobufApis.databaseDriverV1.statementNew(statementNewRequest).getStmtHandle();
    } catch (DatabaseDriverService.ServiceException e) {
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
    ExecuteResult executeResult = executeStatement(sql, bindings);
    applyExecuteQueryResult(executeResult);
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
    ExecuteResult executeResult = executeStatement(sql, bindings);
    return updateExecutionStateAndReturnHasResultSet(executeResult);
  }

  private ExecuteResult executeStatement(String sql, QueryBindings bindings) throws SQLException {
    boolean hasBindings = bindings != null;
    logger.debug("Statement executeWithBindings start: sql={}, hasBindings={}", sql, hasBindings);
    prepareForExecution();
    StatementSetSqlQueryRequest statementSetSqlQueryRequest =
        StatementSetSqlQueryRequest.newBuilder()
            .setStmtHandle(statementHandle)
            .setQuery(sql)
            .build();
    try {
      ProtobufApis.databaseDriverV1.statementSetSqlQuery(statementSetSqlQueryRequest);
    } catch (DatabaseDriverService.ServiceException e) {
      logger.warn("statementSetSqlQuery failed: sql={}, hasBindings={}", sql, hasBindings, e);
      throw new SnowflakeSQLException("Failed to set SQL query on statement", e);
    }

    StatementExecuteQueryRequest.Builder executeQueryRequestBuilder =
        StatementExecuteQueryRequest.newBuilder().setStmtHandle(statementHandle);
    if (bindings != null) {
      executeQueryRequestBuilder.setBindings(bindings);
    }
    StatementExecuteQueryRequest executeQueryRequest = executeQueryRequestBuilder.build();
    logger.debug(
        "statementExecuteQuery request prepared: hasBindings={}, requestBytes={}",
        hasBindings,
        executeQueryRequest.getSerializedSize());
    try {
      StatementExecuteQueryResponse result =
          ProtobufApis.databaseDriverV1.statementExecuteQuery(executeQueryRequest);
      ExecuteResult executeResult = result.getResult();
      logger.debug(
          "statementExecuteQuery succeeded: hasBindings={}, queryId={}",
          hasBindings,
          executeResult.getQueryId());
      return executeResult;
    } catch (DatabaseDriverService.ServiceException e) {
      logger.warn("statementExecuteQuery failed: hasBindings={}", hasBindings, e);
      throw new SnowflakeSQLException("Failed to execute statement query", e);
    }
  }

  private void applyExecuteQueryResult(ExecuteResult executeResult) throws SQLException {
    queryId = executeResult.getQueryId();
    // Match the old JDBC driver: executeQuery() historically accepted single-statement
    // DML/DDL and surfaced a ResultSet wrapper when the server returned a stream
    if (StatementTypeClassifier.producesResultSet(executeResult) || executeResult.hasStream()) {
      applyResultSetExecutionResult(executeResult);
      return;
    }

    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
  }

  private boolean updateExecutionStateAndReturnHasResultSet(ExecuteResult executeResult)
      throws SQLException {
    queryId = executeResult.getQueryId();
    if (StatementTypeClassifier.producesResultSet(executeResult)) {
      applyResultSetExecutionResult(executeResult);
      return true;
    }

    currentResultSet = null;
    currentUpdateCount = StatementTypeClassifier.getUpdateCount(executeResult);
    return false;
  }

  private void applyResultSetExecutionResult(ExecuteResult executeResult) throws SQLException {
    currentResultSet = new SnowflakeResultSetImpl(this, executeResult);
    currentUpdateCount = NO_UPDATE_COUNT;
  }

  private void prepareForExecution() throws SQLException {
    if (currentResultSet != null && !currentResultSet.isClosed()) {
      openResultSets.add(currentResultSet);
    }
    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
    queryId = null;
  }

  private void clearExecutionState() throws SQLException {
    closeCurrentResultSet();
    for (ResultSet resultSet : openResultSets) {
      closeResultSet(resultSet);
    }
    openResultSets.clear();
    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
    queryId = null;
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
    if (closed) {
      return;
    }
    clearExecutionState();
    closed = true;
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
    // TODO: Implement multi-statement result traversal once JDBC can iterate child results beyond
    // the first ExecuteResult returned for a statement execution.
    currentResultSet = null;
    currentUpdateCount = NO_UPDATE_COUNT;
    return false;
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
    return closed;
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
    if (closed) {
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
    throw new SQLFeatureNotSupportedException("setParameter not supported");
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
