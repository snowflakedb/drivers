package net.snowflake.client.internal.api.implementation.pooling;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.HashSet;
import java.util.Set;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import javax.sql.StatementEventListener;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

public class SnowflakePooledConnection implements PooledConnection {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakePooledConnection.class);

  private Connection physicalConnection;
  private final Set<ConnectionEventListener> eventListeners;

  public SnowflakePooledConnection(Connection physicalConnection) throws SQLException {
    this.physicalConnection = physicalConnection;
    SnowflakeConnectionImpl sfConnection = physicalConnection.unwrap(SnowflakeConnectionImpl.class);
    logger.debug(
        "Creating new pooled connection with session id: {}", safeGetSessionID(sfConnection));
    this.eventListeners = new HashSet<>();
  }

  @Override
  public Connection getConnection() throws SQLException {
    SnowflakeConnectionImpl sfConnection = physicalConnection.unwrap(SnowflakeConnectionImpl.class);
    logger.debug(
        "Creating new Logical Connection based on pooled connection with session id: {}",
        safeGetSessionID(sfConnection));
    return new LogicalConnection(this);
  }

  Connection getPhysicalConnection() {
    return physicalConnection;
  }

  void fireConnectionCloseEvent() {
    for (ConnectionEventListener connectionEventListener : eventListeners) {
      connectionEventListener.connectionClosed(new ConnectionEvent(this));
    }
  }

  void fireConnectionErrorEvent(SQLException e) {
    for (ConnectionEventListener connectionEventListener : eventListeners) {
      connectionEventListener.connectionErrorOccurred(new ConnectionEvent(this, e));
    }
  }

  @Override
  public void addConnectionEventListener(ConnectionEventListener eventListener) {
    this.eventListeners.add(eventListener);
  }

  @Override
  public void close() throws SQLException {
    if (this.physicalConnection != null) {
      SnowflakeConnectionImpl sfConnection =
          physicalConnection.unwrap(SnowflakeConnectionImpl.class);
      logger.debug("Closing pooled connection with session id: {}", safeGetSessionID(sfConnection));
      this.physicalConnection.close();
      this.physicalConnection = null;
    }
    eventListeners.clear();
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

  // TODO not fully implemented yet
  private static String safeGetSessionID(SnowflakeConnectionImpl sfConnection) {
    try {
      return sfConnection.getSessionID();
    } catch (Exception e) {
      return "Not implemented";
    }
  }
}
