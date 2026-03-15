package net.snowflake.client.api.pooling;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import net.snowflake.client.internal.api.implementation.pooling.SnowflakePooledConnection;
import org.junit.jupiter.api.Test;

public class PooledConnectionLifecycleIT extends PoolingTestBase {

  @Test
  public void testPooledConnectionBasicLifecycle() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    assertNotNull(pooledConnection);

    Connection logicalConnection = pooledConnection.getConnection();
    assertNotNull(logicalConnection);
    assertFalse(logicalConnection.isClosed());

    try (Statement stmt = logicalConnection.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1")) {
      assertTrue(rs.next());
      assertEquals(1, rs.getInt(1));
    }

    logicalConnection.close();
    assertTrue(logicalConnection.isClosed());

    // Physical connection should still be alive: prove by getting a new logical connection
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      try (Statement stmt = logicalConnection2.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
      }
    }

    pooledConnection.close();
  }

  @Test
  public void testConnectionCloseEventFires() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      logicalConnection.createStatement().execute("SELECT 1");
    }

    assertEquals(1, listener.closedEvents.size());
    ConnectionEvent closedEvent = listener.closedEvents.get(0);
    assertNull(closedEvent.getSQLException());
    assertInstanceOf(SnowflakePooledConnection.class, closedEvent.getSource());
    assertSame(pooledConnection, closedEvent.getSource());

    // Physical connection should still be alive after logical close
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      try (Statement stmt = logicalConnection2.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
      }
    }

    pooledConnection.removeConnectionEventListener(listener);
    pooledConnection.close();
  }

  @Test
  public void testConnectionErrorEventFires() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try {
        logicalConnection.setCatalog("nonexistent_database_xyz_9999");
      } catch (SQLException e) {
        assertEquals(1, listener.errorEvents.size());
        ConnectionEvent errorEvent = listener.errorEvents.get(0);
        assertInstanceOf(SnowflakePooledConnection.class, errorEvent.getSource());
        assertSame(pooledConnection, errorEvent.getSource());
        assertEquals(e.getErrorCode(), errorEvent.getSQLException().getErrorCode());
      }
    }

    pooledConnection.close();
  }

  @Test
  public void testGetPooledConnectionWithUserAndPassword() throws Exception {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();

    PooledConnection pooledConnection =
        poolDataSource.getPooledConnection(getUser(), getPassword());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      logicalConnection.createStatement().execute("SELECT 1");
    }
    pooledConnection.close();
  }

  static class TestConnectionListener implements ConnectionEventListener {
    final List<ConnectionEvent> closedEvents = new ArrayList<>();
    final List<ConnectionEvent> errorEvents = new ArrayList<>();

    @Override
    public void connectionClosed(ConnectionEvent event) {
      closedEvents.add(event);
    }

    @Override
    public void connectionErrorOccurred(ConnectionEvent event) {
      errorEvents.add(event);
    }
  }
}
