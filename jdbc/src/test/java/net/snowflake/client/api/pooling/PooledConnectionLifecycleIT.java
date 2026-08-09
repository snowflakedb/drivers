package net.snowflake.client.api.pooling;

import static net.snowflake.jdbc.utils.PoolingTestCompat.assertGetConnectionAfterPooledCloseFails;
import static net.snowflake.jdbc.utils.TestParameters.has;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import org.junit.jupiter.api.Test;

public class PooledConnectionLifecycleIT extends PoolingTestBase {

  @Test
  public void shouldPooledConnectionBasicLifecycle() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertFalse(logicalConnection.isClosed());

      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }

    // Physical connection should still be alive: prove by getting a new logical connection
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      try (Statement stmt = logicalConnection2.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }

    pooledConnection.close();
  }

  @Test
  public void shouldConnectionCloseEventFires() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement stmt = logicalConnection.createStatement()) {
        stmt.execute("SELECT 1");
      }
    }

    assertEquals(1, listener.closedEvents.size());
    ConnectionEvent closedEvent = listener.closedEvents.get(0);
    assertNull(closedEvent.getSQLException());
    assertInstanceOf(PooledConnection.class, closedEvent.getSource());
    assertSame(pooledConnection, closedEvent.getSource());

    // Physical connection should still be alive after logical close
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      try (Statement stmt = logicalConnection2.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }

    pooledConnection.removeConnectionEventListener(listener);
    pooledConnection.close();
  }

  @Test
  public void shouldConnectionErrorEventFires() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      SnowflakeConnectionImpl physicalConnection =
          logicalConnection.unwrap(SnowflakeConnectionImpl.class);
      SQLException error =
          assertThrows(
              SQLException.class,
              () -> logicalConnection.setCatalog("nonexistent_database_xyz_9999"));
      assertEquals(1, listener.errorEvents.size());
      ConnectionEvent errorEvent = listener.errorEvents.get(0);
      assertInstanceOf(PooledConnection.class, errorEvent.getSource());
      assertSame(pooledConnection, errorEvent.getSource());
      assertNotNull(errorEvent.getSQLException());
      assertEquals(error.getErrorCode(), errorEvent.getSQLException().getErrorCode());

      // A connectionErrorOccurred for a semantic failure (invalid catalog) must not kill the
      // physical connection: the same logical handle can still run queries afterwards.
      assertFalse(physicalConnection.isClosed());
      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }

    pooledConnection.close();
  }

  @Test
  public void shouldGetPooledConnectionWithUserAndPassword() throws Exception {
    // The (user, password) overload is password-specific; skip in JWT-only environments where
    // SNOWFLAKE_TEST_PASSWORD is not configured.
    assumeTrue(
        has("SNOWFLAKE_TEST_PASSWORD"),
        "Skipping credential-overload test: SNOWFLAKE_TEST_PASSWORD not configured");
    SnowflakeConnectionPoolDataSource poolDataSource = createPasswordConfiguredPoolDataSource();

    PooledConnection pooledConnection =
        trackPooledConnection(poolDataSource.getPooledConnection(getUser(), getPassword()));
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        // Assert the query actually returns a result so the test fails if the credential overload
        // yields an unusable connection rather than silently passing on a no-throw execute().
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldGetConnectionAfterPooledConnectionCloseThrowsConnectionClosed()
      throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());

    // Borrow once to confirm the pooled connection is usable, then close the pooled connection.
    pooledConnection.getConnection().close();
    pooledConnection.close();

    // After the pooled connection is closed, borrowing again must fail with CONNECTION_CLOSED
    // (universal driver) or NullPointerException (legacy driver; BD#27).
    assertGetConnectionAfterPooledCloseFails(pooledConnection);
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
