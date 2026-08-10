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
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.util.DelegatingWrapper;

@JdbcBoundary
public class SnowflakePooledConnection implements PooledConnection {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakePooledConnection.class);

  private volatile Connection physicalConnection;
  private final Set<ConnectionEventListener> eventListeners;
  private LogicalConnection currentHandle;

  // The PooledConnection handed to the application is the decorator that wraps this instance, not
  // this raw impl. ConnectionEvent.getSource() must therefore report the decorator so listeners can
  // match the event source against the object they registered on (JDBC contract). Defaults to this
  // for direct/unwrapped use; the datasource points it at the decorator via setEventSource().
  private volatile PooledConnection eventSource = this;

  public SnowflakePooledConnection(Connection physicalConnection) {
    this.physicalConnection = physicalConnection;
    logger.debug(
        "Creating new pooled connection with session id: {}", safeGetSessionID(physicalConnection));
    this.eventListeners = new CopyOnWriteArraySet<>();
  }

  @Override
  public Connection getConnection() {
    Connection currentPhysicalConnection = getPhysicalConnection();
    SnowflakeConnectionImpl sfConnection =
        DelegatingWrapper.unwrapUnchecked(currentPhysicalConnection, SnowflakeConnectionImpl.class);
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
    return new DecoratedLogicalConnection(newHandle, sfConnection.getTelemetry());
  }

  /**
   * Returns the live physical connection backing this pooled connection. Throws {@code
   * CONNECTION_CLOSED} when the pooled connection has been closed (physical reference cleared) or
   * the physical connection is already dead (e.g. after a logical {@code abort()}). Reading the
   * volatile field into a local snapshot keeps the null/closed check and the return value
   * consistent even if another thread closes the pooled connection concurrently.
   */
  Connection getPhysicalConnection() {
    Connection currentPhysicalConnection = physicalConnection;
    try {
      if (currentPhysicalConnection == null || currentPhysicalConnection.isClosed()) {
        throw new SFSQLException(CONNECTION_CLOSED, "Connection is closed");
      }
    } catch (SQLException e) {
      throw new SFSQLException("Failed to check whether the physical connection is closed", e);
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
    ConnectionEvent event = new ConnectionEvent(eventSource);
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
    ConnectionEvent event = new ConnectionEvent(eventSource, e);
    for (ConnectionEventListener connectionEventListener : eventListeners) {
      try {
        connectionEventListener.connectionErrorOccurred(event);
      } catch (RuntimeException listenerError) {
        logger.warn("Connection error event listener threw an exception", listenerError);
      }
    }
  }

  /**
   * Sets the source reported by {@link ConnectionEvent#getSource()} to the decorator that wraps
   * this pooled connection. The application never sees this raw impl, so listeners must observe the
   * decorator as the event source.
   */
  void setEventSource(PooledConnection eventSource) {
    this.eventSource = eventSource;
  }

  @Override
  public void addConnectionEventListener(ConnectionEventListener eventListener) {
    this.eventListeners.add(eventListener);
  }

  @Override
  public void close() {
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
        logger.debug(
            "Closing pooled connection with session id: {}", safeGetSessionID(connectionToClose));
        try {
          connectionToClose.close();
        } catch (SQLException e) {
          throw new SFSQLException("Failed to close the pooled physical connection", e);
        }
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

  private static String safeGetSessionID(Connection physicalConnection) {
    return safeGetSessionID(
        DelegatingWrapper.unwrapUnchecked(physicalConnection, SnowflakeConnectionImpl.class));
  }

  /**
   * Session id is only used for debug logging, so a lookup failure must never break pooling.
   * getSessionID() is not implemented yet and throws an unchecked exception, so both checked and
   * runtime failures are tolerated here.
   */
  private static String safeGetSessionID(SnowflakeConnectionImpl sfConnection) {
    try {
      return sfConnection.getSessionID();
    } catch (DriverRuntimeException e) {
      // getSessionID() is a debug-logging convenience, so its failures must be swallowed, not break
      // pooling. It surfaces only the unchecked driver carriers - CoreException when it is
      // unimplemented, CONNECTION_CLOSED (SFSQLException) when probed on an already-aborted/closed
      // connection during close() - so catch that whole family here.
      logger.debug("Could not resolve session id for pooled connection logging", e);
      return "unknown";
    }
  }
}
