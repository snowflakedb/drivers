package net.snowflake.client.internal.api.implementation.pooling;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_CLOSED;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Set;
import java.util.concurrent.CopyOnWriteArraySet;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import javax.sql.StatementEventListener;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

public class SnowflakePooledConnection implements PooledConnection {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakePooledConnection.class);

  private volatile Connection physicalConnection;
  private final Set<ConnectionEventListener> eventListeners;
  private LogicalConnection currentHandle;

  public SnowflakePooledConnection(Connection physicalConnection) throws SQLException {
    this.physicalConnection = physicalConnection;
    SnowflakeConnectionImpl sfConnection = physicalConnection.unwrap(SnowflakeConnectionImpl.class);
    logger.debug(
        "Creating new pooled connection with session id: {}", safeGetSessionID(sfConnection));
    this.eventListeners = new CopyOnWriteArraySet<>();
  }

  @Override
  public Connection getConnection() throws SQLException {
    Connection currentPhysicalConnection = getPhysicalConnection();
    SnowflakeConnectionImpl sfConnection =
        currentPhysicalConnection.unwrap(SnowflakeConnectionImpl.class);
    logger.debug(
        "Creating new Logical Connection based on pooled connection with session id: {}",
        safeGetSessionID(sfConnection));
    LogicalConnection newHandle = new LogicalConnection(this);
    // The javax.sql.PooledConnection contract allows at most one active logical handle per pooled
    // connection: borrowing a new handle must invalidate any previously returned, still-open one so
    // two handles can never drive the same physical session. Invalidate silently (no
    // connectionClosed event) because this is internal reclamation, not an application close.
    synchronized (this) {
      LogicalConnection previousHandle = this.currentHandle;
      if (previousHandle != null) {
        previousHandle.invalidate();
      }
      this.currentHandle = newHandle;
    }
    logger.debug("getConnection: returning new logical connection for pooled connection");
    return newHandle;
  }

  /**
   * Returns the live physical connection backing this pooled connection. Throws {@code
   * CONNECTION_CLOSED} when the pooled connection has been closed (physical reference cleared) or
   * the physical connection is already dead (e.g. after a logical {@code abort()}). Reading the
   * volatile field into a local snapshot keeps the null/closed check and the return value
   * consistent even if another thread closes the pooled connection concurrently.
   */
  Connection getPhysicalConnection() throws SQLException {
    Connection currentPhysicalConnection = physicalConnection;
    if (currentPhysicalConnection == null || currentPhysicalConnection.isClosed()) {
      throw new SnowflakeSQLException(CONNECTION_CLOSED, "Connection is closed");
    }
    return currentPhysicalConnection;
  }

  void fireConnectionCloseEvent() {
    // Once close() has claimed the pooled connection (physical reference cleared) no further
    // lifecycle events must reach listeners: a late connectionClosed would tell the pool manager a
    // being-destroyed connection is idle/reusable and corrupt its bookkeeping.
    if (physicalConnection == null) {
      return;
    }
    ConnectionEvent event = new ConnectionEvent(this);
    for (ConnectionEventListener connectionEventListener : eventListeners) {
      try {
        connectionEventListener.connectionClosed(event);
      } catch (RuntimeException listenerError) {
        logger.warn("Connection close event listener threw an exception", listenerError);
      }
    }
  }

  void fireConnectionErrorEvent(SQLException e) {
    // See fireConnectionCloseEvent: suppress events delivered after the pooled connection is
    // closed.
    if (physicalConnection == null) {
      return;
    }
    ConnectionEvent event = new ConnectionEvent(this, e);
    for (ConnectionEventListener connectionEventListener : eventListeners) {
      try {
        connectionEventListener.connectionErrorOccurred(event);
      } catch (RuntimeException listenerError) {
        logger.warn("Connection error event listener threw an exception", listenerError);
      }
    }
  }

  @Override
  public void addConnectionEventListener(ConnectionEventListener eventListener) {
    this.eventListeners.add(eventListener);
  }

  @Override
  public void close() throws SQLException {
    logger.debug("close: closing pooled connection");
    // Atomically claim the physical connection so concurrent close() calls cannot double-close it
    // or NPE on a reference the other thread already cleared.
    Connection connectionToClose;
    synchronized (this) {
      connectionToClose = this.physicalConnection;
      this.physicalConnection = null;
      this.currentHandle = null;
    }
    // Clear listeners even if the physical close throws, so a failed close cannot leave listeners
    // registered on a pooled connection whose physical reference is already gone.
    try {
      if (connectionToClose != null) {
        SnowflakeConnectionImpl sfConnection =
            connectionToClose.unwrap(SnowflakeConnectionImpl.class);
        logger.debug(
            "Closing pooled connection with session id: {}", safeGetSessionID(sfConnection));
        connectionToClose.close();
      }
    } finally {
      eventListeners.clear();
    }
    logger.debug("close: pooled connection closed");
  }

  @Override
  public void removeConnectionEventListener(ConnectionEventListener eventListener) {
    this.eventListeners.remove(eventListener);
  }

  @Override
  public void addStatementEventListener(StatementEventListener eventListener) {
    // not supported
  }

  @Override
  public void removeStatementEventListener(StatementEventListener eventListener) {
    // not supported
  }

  /**
   * Session id is only used for debug logging, so a lookup failure must never break pooling.
   * getSessionID() is not implemented yet and throws an unchecked exception, so both checked and
   * runtime failures are tolerated here.
   */
  private static String safeGetSessionID(SnowflakeConnectionImpl sfConnection) {
    try {
      return sfConnection.getSessionID();
    } catch (SQLException | RuntimeException e) {
      logger.debug("Could not resolve session id for pooled connection logging", e);
      return "unknown";
    }
  }
}
