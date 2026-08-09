package net.snowflake.client.internal.api.implementation.pooling;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_CLOSED;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import org.junit.jupiter.api.Test;

public class SnowflakePooledConnectionTest {

  // SnowflakePooledConnection is a @JdbcBoundary: getConnection()'s public contract — translating
  // the runtime CONNECTION_CLOSED carrier into the checked SnowflakeSQLException the JDBC API
  // promises — lives in its generated decorator, so exception-contract assertions must go through
  // it rather than the raw impl.
  private static PooledConnection decorated(SnowflakePooledConnection raw) {
    return new DecoratedSnowflakePooledConnection(raw, Telemetry.NOOP);
  }

  @Test
  public void shouldGetConnectionAfterCloseThrowsConnectionClosed() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    PooledConnection pooledConnection =
        decorated(new SnowflakePooledConnection(physicalConnection));
    pooledConnection.close();

    // After the pooled connection is closed the physical connection is released; borrowing a
    // logical connection must fail cleanly with a SQLException rather than a NullPointerException.
    SnowflakeSQLException ex =
        assertThrows(SnowflakeSQLException.class, pooledConnection::getConnection);
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    verify(physicalConnection).close();
  }

  @Test
  public void shouldEventsAreSuppressedAfterPooledConnectionIsClosed() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener listener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(listener);

    // Positive control: while the pooled connection is open, events do reach listeners.
    pooledConnection.fireConnectionCloseEvent();
    verify(listener).connectionClosed(org.mockito.ArgumentMatchers.any());

    pooledConnection.close();

    // Register a listener AFTER close so the suppression cannot be attributed to close() having
    // cleared the listener set; this isolates the physicalConnection==null guard. Once the pooled
    // connection is closed, a late event from an in-flight logical handle must not reach listeners:
    // a stray connectionClosed would tell the pool a destroyed connection is idle/reusable.
    ConnectionEventListener lateListener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(lateListener);

    pooledConnection.fireConnectionCloseEvent();
    pooledConnection.fireConnectionErrorEvent(new SQLException("boom"));

    verify(lateListener, never()).connectionClosed(org.mockito.ArgumentMatchers.any());
    verify(lateListener, never()).connectionErrorOccurred(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldCloseEventListenerExceptionDoesNotPropagate() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener throwingListener = mock(ConnectionEventListener.class);
    org.mockito.Mockito.doThrow(new RuntimeException("listener boom"))
        .when(throwingListener)
        .connectionClosed(org.mockito.ArgumentMatchers.any());
    ConnectionEventListener healthyListener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(throwingListener);
    pooledConnection.addConnectionEventListener(healthyListener);

    // A misbehaving listener must not break event fan-out to the remaining listeners.
    pooledConnection.fireConnectionCloseEvent();

    // Verify the throwing listener was actually invoked, otherwise the test would pass even if the
    // first listener were silently skipped.
    verify(throwingListener).connectionClosed(org.mockito.ArgumentMatchers.any());
    verify(healthyListener).connectionClosed(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldGetConnectionWhenPhysicalConnectionIsDeadThrowsConnectionClosed()
      throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    PooledConnection pooledConnection =
        decorated(new SnowflakePooledConnection(physicalConnection));
    // Simulate the physical connection being torn down (e.g. by a logical abort) without the
    // pooled connection being explicitly closed.
    when(physicalConnection.isClosed()).thenReturn(true);

    SnowflakeSQLException ex =
        assertThrows(SnowflakeSQLException.class, pooledConnection::getConnection);
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
  }

  @Test
  public void shouldErrorEventListenerExceptionDoesNotPropagate() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener throwingListener = mock(ConnectionEventListener.class);
    org.mockito.Mockito.doThrow(new RuntimeException("listener boom"))
        .when(throwingListener)
        .connectionErrorOccurred(org.mockito.ArgumentMatchers.any());
    ConnectionEventListener healthyListener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(throwingListener);
    pooledConnection.addConnectionEventListener(healthyListener);

    // A misbehaving listener must not break error-event fan-out to the remaining listeners.
    pooledConnection.fireConnectionErrorEvent(new SQLException("boom"));

    // Verify the throwing listener was actually invoked, otherwise the test would pass even if the
    // first listener were silently skipped.
    verify(throwingListener).connectionErrorOccurred(org.mockito.ArgumentMatchers.any());
    verify(healthyListener).connectionErrorOccurred(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldGetConnectionReturnsUsableLogicalConnectionAndNotifiesListenersOnClose()
      throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);
    when(physicalConnection.getAutoCommit()).thenReturn(true);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener listener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(listener);

    // getConnection() returns a usable logical handle wired to the real pooled connection, now
    // behind its decorated boundary (the delegation assertions below prove it stays usable).
    Connection logicalConnection = pooledConnection.getConnection();
    assertInstanceOf(DecoratedLogicalConnection.class, logicalConnection);
    assertFalse(logicalConnection.isClosed());
    // Delegation reaches the physical connection.
    assertTrue(logicalConnection.getAutoCommit());

    // Closing the logical handle fires connectionClosed to the real listener without closing the
    // physical connection, exercising the wiring that mock-based tests cannot.
    logicalConnection.close();
    assertTrue(logicalConnection.isClosed());
    verify(listener).connectionClosed(org.mockito.ArgumentMatchers.any());
    verify(physicalConnection, never()).close();
  }

  @Test
  public void shouldRemovedListenerNoLongerReceivesEvents() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener listener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(listener);

    // Positive control: while registered, the listener does receive events, so the never()
    // assertions below cannot pass vacuously due to a broken dispatch.
    pooledConnection.fireConnectionCloseEvent();
    verify(listener).connectionClosed(org.mockito.ArgumentMatchers.any());

    pooledConnection.removeConnectionEventListener(listener);
    pooledConnection.fireConnectionCloseEvent();
    pooledConnection.fireConnectionErrorEvent(new SQLException("boom"));

    // A removed listener must not be notified of any further events: exactly one close event total
    // (from the positive control) and zero error events.
    verify(listener, org.mockito.Mockito.times(1))
        .connectionClosed(org.mockito.ArgumentMatchers.any());
    verify(listener, never()).connectionErrorOccurred(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldReentrantCloseFromListenerFiresExactlyOneCloseEvent() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    Connection logicalConnection = pooledConnection.getConnection();

    // BD#30: the logical handle flips its closed flag before firing the event, so a listener that
    // re-enters close() from its connectionClosed callback must not trigger a second event (the old
    // driver could fire duplicate connectionClosed because the flag was set after the event).
    java.util.concurrent.atomic.AtomicInteger closeEvents =
        new java.util.concurrent.atomic.AtomicInteger();
    ConnectionEventListener reentrantListener = mock(ConnectionEventListener.class);
    org.mockito.Mockito.doAnswer(
            invocation -> {
              closeEvents.incrementAndGet();
              logicalConnection.close();
              return null;
            })
        .when(reentrantListener)
        .connectionClosed(org.mockito.ArgumentMatchers.any());
    pooledConnection.addConnectionEventListener(reentrantListener);

    logicalConnection.close();

    assertEquals(1, closeEvents.get());
  }

  @Test
  public void shouldReborrowInvalidatesPriorHandleWithoutFiringCloseEvent() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    ConnectionEventListener listener = mock(ConnectionEventListener.class);
    pooledConnection.addConnectionEventListener(listener);

    Connection firstHandle = pooledConnection.getConnection();
    assertFalse(firstHandle.isClosed());

    // javax.sql.PooledConnection contract: borrowing a new logical handle invalidates the prior
    // one so two handles can never drive the same physical session.
    Connection secondHandle = pooledConnection.getConnection();
    assertTrue(firstHandle.isClosed());
    assertFalse(secondHandle.isClosed());

    // The silent invalidation must NOT fire a connectionClosed event (it is internal reclamation,
    // not an application close); otherwise the pool would think the connection became idle.
    verify(listener, never()).connectionClosed(org.mockito.ArgumentMatchers.any());

    // Operations on the invalidated handle fail with CONNECTION_CLOSED.
    SnowflakeSQLException ex =
        assertThrows(SnowflakeSQLException.class, firstHandle::createStatement);
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());

    // The live handle still delegates to the physical connection, and closing it fires exactly one
    // event for that checkout.
    secondHandle.close();
    verify(listener, org.mockito.Mockito.times(1))
        .connectionClosed(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldDoubleCloseClosesPhysicalConnectionOnce() throws SQLException {
    Connection physicalConnection = mock(Connection.class);
    SnowflakeConnectionImpl sfConnection = mock(SnowflakeConnectionImpl.class);
    when(physicalConnection.unwrap(SnowflakeConnectionImpl.class)).thenReturn(sfConnection);

    SnowflakePooledConnection pooledConnection = new SnowflakePooledConnection(physicalConnection);
    pooledConnection.close();
    pooledConnection.close();

    // close() atomically claims the physical connection, so a second close must be a no-op rather
    // than closing an already-closed (or null) physical connection again.
    verify(physicalConnection, org.mockito.Mockito.times(1)).close();
  }
}
