package net.snowflake.client.internal.api.implementation.pooling;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_CLOSED;

import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.ClientInfoStatus;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.NClob;
import java.sql.PreparedStatement;
import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Savepoint;
import java.sql.Statement;
import java.sql.Struct;
import java.util.HashMap;
import java.util.Map;
import java.util.Properties;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.api.exception.SnowflakeSQLException;

class LogicalConnection implements Connection {

  private final Connection physicalConnection;
  private final SnowflakePooledConnection pooledConnection;
  private final AtomicBoolean closed = new AtomicBoolean();

  LogicalConnection(SnowflakePooledConnection pooledConnection) throws SQLException {
    this.physicalConnection = pooledConnection.getPhysicalConnection();
    this.pooledConnection = pooledConnection;
    // getPhysicalConnection() already rejects a null/closed physical at snapshot time, but the
    // physical connection can be closed in the narrow window between that snapshot and the end of
    // construction. Reject a handle backed by an already-closed physical session with
    // CONNECTION_CLOSED (consistent with BD#27) rather than returning one that would let the pool
    // recycle a dead connection on its later close().
    if (physicalConnection.isClosed()) {
      throw new SnowflakeSQLException(CONNECTION_CLOSED, "Connection is closed");
    }
  }

  /**
   * Delegates a void operation to the physical connection, firing a connection error event for any
   * {@link SQLException} the delegate throws except the exempt types listed below.
   *
   * <p>{@link SQLFeatureNotSupportedException} and {@link SQLClientInfoException} are rethrown
   * without firing an error event: an unimplemented JDBC feature or a rejected client-info property
   * is a caller/property mistake, not a sign the pooled physical connection is broken, and pool
   * managers typically evict a connection that signals {@code connectionErrorOccurred}.
   */
  private void runOnPhysical(SqlRunnable action) throws SQLException {
    try {
      action.run();
    } catch (SQLFeatureNotSupportedException | SQLClientInfoException e) {
      throw e;
    } catch (SQLException e) {
      fireConnectionErrorEventIfOpen(e);
      throw e;
    }
  }

  /**
   * Delegates a value-returning operation to the physical connection. See {@link #runOnPhysical}.
   */
  private <T> T callOnPhysical(SqlCallable<T> action) throws SQLException {
    try {
      return action.call();
    } catch (SQLFeatureNotSupportedException | SQLClientInfoException e) {
      throw e;
    } catch (SQLException e) {
      fireConnectionErrorEventIfOpen(e);
      throw e;
    }
  }

  /**
   * Fires {@code connectionErrorOccurred} only while this handle is still open. If a concurrent
   * {@link #close()} has already claimed the handle and fired {@code connectionClosed} (telling the
   * pool to recycle the connection), suppressing the error event avoids delivering conflicting
   * close/error signals for the same checkout.
   */
  private void fireConnectionErrorEventIfOpen(SQLException e) {
    if (!closed.get()) {
      pooledConnection.fireConnectionErrorEvent(e);
    }
  }

  @Override
  public Statement createStatement() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::createStatement);
  }

  @Override
  public PreparedStatement prepareStatement(String sql) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.prepareStatement(sql));
  }

  @Override
  public CallableStatement prepareCall(String sql) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.prepareCall(sql));
  }

  @Override
  public String nativeSQL(String sql) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.nativeSQL(sql));
  }

  @Override
  public void setAutoCommit(boolean autoCommit) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setAutoCommit(autoCommit));
  }

  @Override
  public boolean getAutoCommit() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getAutoCommit);
  }

  @Override
  public void commit() throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(physicalConnection::commit);
  }

  @Override
  public void rollback() throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(physicalConnection::rollback);
  }

  /** Logical connection close does not close the physical connection; it only fires events. */
  @Override
  public void close() throws SQLException {
    // compareAndSet guarantees the close event fires exactly once, even under concurrent close().
    if (!closed.compareAndSet(false, true)) {
      return;
    }
    pooledConnection.fireConnectionCloseEvent();
  }

  /**
   * Silently invalidates this handle without firing a {@code connectionClosed} event. Used by
   * {@link SnowflakePooledConnection#getConnection()} when a new logical handle is borrowed while a
   * prior one is still open: the {@code javax.sql.PooledConnection} contract requires the
   * previously returned handle to become unusable, but this is internal reclamation rather than an
   * application close, so no pool lifecycle event must be delivered. Subsequent operations on the
   * invalidated handle then fail with {@code CONNECTION_CLOSED}.
   */
  void invalidate() {
    closed.set(true);
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed.get();
  }

  @Override
  public DatabaseMetaData getMetaData() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getMetaData);
  }

  @Override
  public void setReadOnly(boolean readOnly) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setReadOnly(readOnly));
  }

  @Override
  public boolean isReadOnly() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::isReadOnly);
  }

  @Override
  public void setCatalog(String catalog) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setCatalog(catalog));
  }

  @Override
  public String getCatalog() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getCatalog);
  }

  @Override
  public void setTransactionIsolation(int level) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setTransactionIsolation(level));
  }

  @Override
  public int getTransactionIsolation() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getTransactionIsolation);
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getWarnings);
  }

  @Override
  public void clearWarnings() throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(physicalConnection::clearWarnings);
  }

  @Override
  public Statement createStatement(int resultSetType, int resultSetConcurrency)
      throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () -> physicalConnection.createStatement(resultSetType, resultSetConcurrency));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () -> physicalConnection.prepareStatement(sql, resultSetType, resultSetConcurrency));
  }

  @Override
  public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () -> physicalConnection.prepareCall(sql, resultSetType, resultSetConcurrency));
  }

  @Override
  public Map<String, Class<?>> getTypeMap() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getTypeMap);
  }

  @Override
  public void setTypeMap(Map<String, Class<?>> map) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setTypeMap(map));
  }

  @Override
  public void setHoldability(int holdability) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setHoldability(holdability));
  }

  @Override
  public int getHoldability() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getHoldability);
  }

  @Override
  public Savepoint setSavepoint() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::setSavepoint);
  }

  @Override
  public Savepoint setSavepoint(String name) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.setSavepoint(name));
  }

  @Override
  public void rollback(Savepoint savepoint) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.rollback(savepoint));
  }

  @Override
  public void releaseSavepoint(Savepoint savepoint) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.releaseSavepoint(savepoint));
  }

  @Override
  public Statement createStatement(
      int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () ->
            physicalConnection.createStatement(
                resultSetType, resultSetConcurrency, resultSetHoldability));
  }

  @Override
  public PreparedStatement prepareStatement(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () ->
            physicalConnection.prepareStatement(
                sql, resultSetType, resultSetConcurrency, resultSetHoldability));
  }

  @Override
  public CallableStatement prepareCall(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(
        () ->
            physicalConnection.prepareCall(
                sql, resultSetType, resultSetConcurrency, resultSetHoldability));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int autoGeneratedKeys) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.prepareStatement(sql, autoGeneratedKeys));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.prepareStatement(sql, columnIndexes));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.prepareStatement(sql, columnNames));
  }

  @Override
  public Clob createClob() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::createClob);
  }

  @Override
  public Blob createBlob() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::createBlob);
  }

  @Override
  public NClob createNClob() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::createNClob);
  }

  @Override
  public SQLXML createSQLXML() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::createSQLXML);
  }

  /**
   * A closed logical handle reports {@code false} without touching the physical connection or
   * signalling the pool (an already-returned handle is simply invalid).
   *
   * <p>When the handle is open the call is delegated through {@link #callOnPhysical}, so any {@link
   * SQLException} the physical connection <em>throws</em> (including the negative-timeout argument
   * error mandated by {@link Connection#isValid(int)}) fires {@code connectionErrorOccurred},
   * matching the "failing delegate operation fires a connection error event" contract asserted by
   * the {@code shouldFireConnectionErrorEventWhenPhysicalConnectionDelegatesThrow} scenario. Note
   * this only covers thrown exceptions: a physical {@code isValid()} that simply returns {@code
   * false} on a heartbeat/liveness failure (the normal Snowflake behavior) does not throw and so
   * does not fire {@code connectionErrorOccurred} (see BD#34).
   */
  @Override
  public boolean isValid(int timeout) throws SQLException {
    if (closed.get()) {
      return false;
    }
    return callOnPhysical(() -> physicalConnection.isValid(timeout));
  }

  /**
   * Delegates directly to the physical connection (not through {@link #runOnPhysical}, whose
   * checked signature is wider than {@code throws SQLClientInfoException}). A {@link
   * SQLClientInfoException} reports that a client-info property could not be set (for Snowflake, an
   * unknown/unsupported property name) — a caller/property error, not a sign that the physical
   * connection is broken — so it propagates without firing {@code connectionErrorOccurred},
   * matching the {@link #getClientInfo} path and avoiding eviction of a healthy pooled connection.
   */
  @Override
  public void setClientInfo(String name, String value) throws SQLClientInfoException {
    if (closed.get()) {
      Map<String, ClientInfoStatus> failedProperties = new HashMap<>();
      failedProperties.put(name, ClientInfoStatus.REASON_UNKNOWN_PROPERTY);
      throw connectionClosedClientInfoException(failedProperties);
    }
    physicalConnection.setClientInfo(name, value);
  }

  @Override
  public void setClientInfo(Properties properties) throws SQLClientInfoException {
    if (closed.get()) {
      Map<String, ClientInfoStatus> failedProperties = new HashMap<>();
      if (properties != null) {
        for (String name : properties.stringPropertyNames()) {
          failedProperties.put(name, ClientInfoStatus.REASON_UNKNOWN_PROPERTY);
        }
      }
      throw connectionClosedClientInfoException(failedProperties);
    }
    physicalConnection.setClientInfo(properties);
  }

  /**
   * {@link #setClientInfo} can only throw {@link SQLClientInfoException}, so a closed handle
   * reports the same {@code CONNECTION_CLOSED} SQLState/vendor code as the other closed-state
   * guards rather than a bare, code-less exception. The properties that could not be set are
   * reported in {@code failedProperties} as required by the JDBC contract so pool/validation
   * frameworks can inspect them.
   */
  private SQLClientInfoException connectionClosedClientInfoException(
      Map<String, ClientInfoStatus> failedProperties) {
    return new SQLClientInfoException(
        "Connection is closed",
        CONNECTION_CLOSED.getSqlState(),
        CONNECTION_CLOSED.getMessageCode(),
        failedProperties);
  }

  @Override
  public String getClientInfo(String name) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.getClientInfo(name));
  }

  @Override
  public Properties getClientInfo() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getClientInfo);
  }

  @Override
  public Array createArrayOf(String typeName, Object[] elements) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.createArrayOf(typeName, elements));
  }

  @Override
  public Struct createStruct(String typeName, Object[] attributes) throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(() -> physicalConnection.createStruct(typeName, attributes));
  }

  @Override
  public void setSchema(String schema) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setSchema(schema));
  }

  @Override
  public String getSchema() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getSchema);
  }

  /**
   * Aborts the connection. Unlike {@link #close()}, abort forcibly terminates the underlying
   * physical connection. On a successful abort the logical handle is marked closed and {@code
   * connectionErrorOccurred} is fired so the pool manager <em>discards</em> the now-dead physical
   * connection. {@code connectionClosed} is deliberately <em>not</em> used here: in the {@code
   * javax.sql} pooling contract {@code connectionClosed} means the logical handle was closed
   * normally and the physical connection is idle and reusable, so firing it after abort would leave
   * a standard pool handing out a dead connection on every subsequent borrow. Only {@code
   * connectionErrorOccurred} instructs the pool to evict. The unsupported and failed cases are
   * described below.
   *
   * <p>The logical handle is claimed via {@code compareAndSet} <em>before</em> touching the
   * physical connection. This closes a race with a concurrent {@link #close()}: if {@code close()}
   * were allowed to win and fire {@code connectionClosed} (returning a still-live physical
   * connection to the pool) while this method then aborted that very connection, the pool would
   * hand out a dead connection. On an unsupported abort the handle stays usable; on a failed abort
   * the physical connection is already dead, so the handle remains closed and an error event is
   * fired, preventing a later {@code close()} from recycling it.
   */
  @Override
  public void abort(Executor executor) throws SQLException {
    if (!closed.compareAndSet(false, true)) {
      return;
    }
    try {
      physicalConnection.abort(executor);
    } catch (SQLFeatureNotSupportedException e) {
      // Abort is unsupported, so the physical connection was left untouched; keep handle usable.
      closed.set(false);
      throw e;
    } catch (SQLException e) {
      // The physical connection is now dead; signal an error so the pool discards it and leave the
      // logical handle closed so a subsequent close() does not recycle the dead connection.
      pooledConnection.fireConnectionErrorEvent(e);
      throw e;
    }
    // Abort succeeded: the physical connection is dead. Fire connectionErrorOccurred (not
    // connectionClosed) so the pool evicts the connection instead of treating it as idle/reusable.
    pooledConnection.fireConnectionErrorEvent(
        new SnowflakeSQLException(CONNECTION_CLOSED, "Connection has been aborted"));
  }

  @Override
  public void setNetworkTimeout(Executor executor, int milliseconds) throws SQLException {
    throwExceptionIfClosed();
    runOnPhysical(() -> physicalConnection.setNetworkTimeout(executor, milliseconds));
  }

  @Override
  public int getNetworkTimeout() throws SQLException {
    throwExceptionIfClosed();
    return callOnPhysical(physicalConnection::getNetworkTimeout);
  }

  /**
   * Delegates directly to the physical connection (not through {@link #callOnPhysical}). A failed
   * type resolution (e.g. unwrapping to an unsupported interface) is a caller mistake, not a sign
   * that the physical connection is broken, so the resulting SQLException must not fire {@code
   * connectionErrorOccurred} and evict a healthy pooled connection - mirroring the {@link
   * SQLFeatureNotSupportedException}/{@link SQLClientInfoException} handling documented in
   * BD#26/BD#32.
   */
  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    throwExceptionIfClosed();
    return physicalConnection.isWrapperFor(iface);
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    throwExceptionIfClosed();
    return physicalConnection.unwrap(iface);
  }

  private void throwExceptionIfClosed() throws SQLException {
    if (closed.get()) {
      throw new SnowflakeSQLException(CONNECTION_CLOSED, "Connection is closed");
    }
  }

  @FunctionalInterface
  private interface SqlRunnable {
    void run() throws SQLException;
  }

  @FunctionalInterface
  private interface SqlCallable<T> {
    T call() throws SQLException;
  }
}
