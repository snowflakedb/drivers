package net.snowflake.client.internal.api.implementation.pooling;

import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_CLOSED;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.Connection;
import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;

public class LogicalConnectionTest {

  // LogicalConnection is a @JdbcBoundary: its impl throws unchecked carriers (SFSQLException,
  // SFSQLFeatureNotSupportedException, SFClientInfoException) and the generated decorator is what
  // translates them into the checked SQLException the JDBC Connection contract promises. Public
  // exception-contract assertions therefore drive the handle through its decorator; the physical
  // failures still fire connectionErrorOccurred inside the impl regardless of decoration.
  private static Connection logical(SnowflakePooledConnection pooledConnection) {
    return new DecoratedLogicalConnection(new LogicalConnection(pooledConnection), Telemetry.NOOP);
  }

  @Test
  public void shouldConstructorRejectsAlreadyClosedPhysicalConnection() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(true);

    // A physical connection closed in the borrow window must fail the borrow with CONNECTION_CLOSED
    // rather than hand back a handle backed by an already-dead physical session (which would let
    // the
    // pool recycle a broken connection on the handle's later close()). A constructor cannot be
    // decorated, so it surfaces the runtime carrier directly; translate it the way the boundary
    // would to assert the public errorCode + SQLState contract.
    SFSQLException carrier =
        assertThrows(SFSQLException.class, () -> new LogicalConnection(pooledConnection));
    SnowflakeSQLException ex = (SnowflakeSQLException) carrier.toSQLException();
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
  }

  @Test
  public void shouldAbortDelegatesToPhysicalConnection() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.abort(null);

    verify(physicalConnection).abort(null);
  }

  @Test
  public void shouldAbortMarksLogicalClosedAndFiresErrorEventForEviction() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    assertFalse(logicalConnection.isClosed());

    logicalConnection.abort(null);

    assertTrue(logicalConnection.isClosed());
    // A successful abort kills the physical connection, so the pool must EVICT it. In javax.sql,
    // only connectionErrorOccurred evicts; connectionClosed signals idle/reusable and would leave
    // the pool handing out a dead connection.
    verify(pooledConnection).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
    verify(pooledConnection, never()).fireConnectionCloseEvent();
  }

  @Test
  public void shouldAbortOnClosedLogicalConnectionIsNoOp() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();
    logicalConnection.abort(null);

    // physical connection must not be aborted after the logical handle was already returned
    verify(physicalConnection, never()).abort(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldClosedLogicalConnectionDoesNotFireErrorEvents() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    assertThrows(SQLException.class, logicalConnection::createStatement);
    assertThrows(SQLException.class, logicalConnection::commit);
    assertThrows(SQLException.class, () -> logicalConnection.setCatalog("db"));
    assertThrows(SQLException.class, logicalConnection::getSchema);

    // Operations on a closed logical handle are caller errors, not physical failures,
    // so they must not signal connectionErrorOccurred to the pool.
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldFeatureNotSupportedDoesNotFireErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    doThrow(new SQLFeatureNotSupportedException("not supported"))
        .when(physicalConnection)
        .setHoldability(1);

    Connection logicalConnection = logical(pooledConnection);

    assertThrows(SQLFeatureNotSupportedException.class, () -> logicalConnection.setHoldability(1));

    // Positive control: the call must actually be delegated to the physical connection (otherwise
    // the never()-fire assertion below would pass vacuously even if delegation were broken).
    verify(physicalConnection).setHoldability(1);
    // Unsupported-feature failures must not cause the pool to evict a healthy connection.
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldFailedUnwrapDoesNotFireErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    when(physicalConnection.unwrap(String.class))
        .thenThrow(new SQLException("Cannot unwrap to java.lang.String"));

    Connection logicalConnection = logical(pooledConnection);

    // Unwrapping to an unsupported interface is a caller/type mistake, not a broken physical
    // connection, so it must not signal connectionErrorOccurred and evict a healthy pooled handle.
    assertThrows(SQLException.class, () -> logicalConnection.unwrap(String.class));
    // Positive control: unwrap must reach the physical connection (proves the never()-fire below is
    // meaningful and that the exemption is on the delegate path, not a short-circuit).
    verify(physicalConnection).unwrap(String.class);
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldDoubleCloseFiresExactlyOneCloseEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();
    logicalConnection.close();

    // close() is idempotent: a second close must not return the connection to the pool again.
    verify(pooledConnection, times(1)).fireConnectionCloseEvent();
  }

  @Test
  public void shouldAbortAfterCloseDoesNotRefireCloseEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();
    logicalConnection.abort(null);

    // The close event from close() must not be duplicated by a subsequent no-op abort().
    verify(pooledConnection, times(1)).fireConnectionCloseEvent();
  }

  @Test
  public void shouldUnwrapOnClosedHandleThrowsAndDoesNotDelegate() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    // A closed logical handle must not expose the live physical connection via the wrapper API.
    assertThrows(SQLException.class, () -> logicalConnection.unwrap(SnowflakeConnectionImpl.class));
    assertThrows(
        SQLException.class, () -> logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class));
    verify(physicalConnection, never()).unwrap(org.mockito.ArgumentMatchers.any());
    verify(physicalConnection, never()).isWrapperFor(org.mockito.ArgumentMatchers.any());
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldSetClientInfoFailureDoesNotFireErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    doThrow(new SQLClientInfoException("unknown property", null))
        .when(physicalConnection)
        .setClientInfo("k", "v");

    Connection logicalConnection = logical(pooledConnection);

    // Rejecting an unknown client-info property is a caller error, not a broken connection,
    // so it must not evict the pooled connection.
    assertThrows(SQLClientInfoException.class, () -> logicalConnection.setClientInfo("k", "v"));
    // Positive control: the exemption must be on the delegate path (setClientInfo was invoked).
    verify(physicalConnection).setClientInfo("k", "v");
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldSetClientInfoPropertiesFailureDoesNotFireErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    java.util.Properties properties = new java.util.Properties();
    properties.setProperty("k", "v");
    doThrow(new SQLClientInfoException("unknown property", null))
        .when(physicalConnection)
        .setClientInfo(properties);

    Connection logicalConnection = logical(pooledConnection);

    assertThrows(SQLClientInfoException.class, () -> logicalConnection.setClientInfo(properties));
    // Positive control: the exemption must be on the delegate path (setClientInfo was invoked).
    verify(physicalConnection).setClientInfo(properties);
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldGetClientInfoClientInfoExceptionDoesNotFireErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    when(physicalConnection.getClientInfo("k"))
        .thenThrow(new SQLClientInfoException("unknown property", null));

    Connection logicalConnection = logical(pooledConnection);

    // getClientInfo must be symmetric with setClientInfo: a client-info error is not a connection
    // failure and must not evict the pooled connection.
    assertThrows(SQLClientInfoException.class, () -> logicalConnection.getClientInfo("k"));
    // Positive control: the exemption must be on the delegate path (getClientInfo was invoked).
    verify(physicalConnection).getClientInfo("k");
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldGetClientInfoOnClosedHandleThrowsConnectionClosedCodeWithoutDelegating()
      throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    // BD#29: getClientInfo on a closed handle throws CONNECTION_CLOSED without touching the
    // physical
    // connection and without firing a (mis-leading) connection error event.
    SnowflakeSQLException byName =
        assertThrows(
            SnowflakeSQLException.class, () -> logicalConnection.getClientInfo("ApplicationName"));
    assertEquals(CONNECTION_CLOSED.getMessageCode(), byName.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), byName.getSQLState());

    SnowflakeSQLException all =
        assertThrows(SnowflakeSQLException.class, logicalConnection::getClientInfo);
    assertEquals(CONNECTION_CLOSED.getMessageCode(), all.getErrorCode());

    verify(physicalConnection, never()).getClientInfo(org.mockito.ArgumentMatchers.anyString());
    verify(physicalConnection, never()).getClientInfo();
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldSetClientInfoOnClosedHandleThrowsConnectionClosedCode() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    // setClientInfo can only throw SQLClientInfoException, but on a closed handle it must still
    // carry the same CONNECTION_CLOSED SQLState/vendor code as the other closed-state guards.
    SQLClientInfoException ex =
        assertThrows(SQLClientInfoException.class, () -> logicalConnection.setClientInfo("k", "v"));
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(
        java.sql.ClientInfoStatus.REASON_UNKNOWN_PROPERTY, ex.getFailedProperties().get("k"));
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldSetClientInfoPropertiesOnClosedHandleReportsFailedProperties()
      throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    java.util.Properties props = new java.util.Properties();
    props.setProperty("ApplicationName", "app");
    props.setProperty("ClientUser", "user");

    // The Properties overload is a distinct closed-state path from the single-key overload: it must
    // carry CONNECTION_CLOSED and report every property it could not set in getFailedProperties().
    SQLClientInfoException ex =
        assertThrows(SQLClientInfoException.class, () -> logicalConnection.setClientInfo(props));
    assertEquals(CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(2, ex.getFailedProperties().size());
    // Match the physical SnowflakeConnectionImpl closed-state status (BD#29 parity / BD#22).
    assertEquals(
        java.sql.ClientInfoStatus.REASON_UNKNOWN_PROPERTY,
        ex.getFailedProperties().get("ApplicationName"));
    assertEquals(
        java.sql.ClientInfoStatus.REASON_UNKNOWN_PROPERTY,
        ex.getFailedProperties().get("ClientUser"));
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldLogicalConnectionWhenPhysicalConnectionThrowsErrors() throws SQLException {
    Connection connection = mock(Connection.class);
    SnowflakePooledConnection snowflakePooledConnection = mock(SnowflakePooledConnection.class);
    when(snowflakePooledConnection.getPhysicalConnection()).thenReturn(connection);
    SQLException sqlException = new SQLException("mocking error");
    when(connection.createStatement()).thenThrow(sqlException);
    when(connection.createStatement(1, 2, 3)).thenThrow(sqlException);

    when(connection.prepareStatement("mocksql")).thenThrow(sqlException);
    when(connection.prepareCall("mocksql")).thenThrow(sqlException);
    when(connection.prepareCall("mocksql", 1, 2, 3)).thenThrow(sqlException);
    when(connection.nativeSQL("mocksql")).thenThrow(sqlException);
    when(connection.getAutoCommit()).thenThrow(sqlException);
    when(connection.getMetaData()).thenThrow(sqlException);
    when(connection.isReadOnly()).thenThrow(sqlException);
    when(connection.getCatalog()).thenThrow(sqlException);
    when(connection.getTransactionIsolation()).thenThrow(sqlException);
    when(connection.getWarnings()).thenThrow(sqlException);
    when(connection.prepareCall("mocksql", 1, 2)).thenThrow(sqlException);
    when(connection.getTypeMap()).thenThrow(sqlException);
    when(connection.getHoldability()).thenThrow(sqlException);
    when(connection.createClob()).thenThrow(sqlException);
    when(connection.getClientInfo("mocksql")).thenThrow(sqlException);
    when(connection.getClientInfo()).thenThrow(sqlException);
    when(connection.createArrayOf("mock", null)).thenThrow(sqlException);
    when(connection.getSchema()).thenThrow(sqlException);
    when(connection.getNetworkTimeout()).thenThrow(sqlException);
    when(connection.isWrapperFor(Connection.class)).thenThrow(sqlException);

    doThrow(sqlException).when(connection).setAutoCommit(false);
    doThrow(sqlException).when(connection).commit();
    doThrow(sqlException).when(connection).rollback();
    doThrow(sqlException).when(connection).setReadOnly(false);
    doThrow(sqlException).when(connection).clearWarnings();
    doThrow(sqlException).when(connection).setSchema(null);
    doThrow(sqlException).when(connection).setNetworkTimeout(null, 1);

    // This test exercises the raw impl (not its decorator): firing connectionErrorOccurred on a
    // delegated physical failure is LogicalConnection's own responsibility, independent of the
    // boundary translation, so the delegated calls surface the runtime SFSQLException carrier
    // directly. Routing through the decorator would also change isWrapperFor(Connection.class)
    // semantics (the decorator answers wrapper queries about itself), which this test must observe
    // on the impl.
    LogicalConnection logicalConnection = new LogicalConnection(snowflakePooledConnection);

    assertThrows(SFSQLException.class, logicalConnection::createStatement);
    assertThrows(SFSQLException.class, () -> logicalConnection.createStatement(1, 2, 3));
    assertThrows(SFSQLException.class, () -> logicalConnection.nativeSQL("mocksql"));
    assertThrows(SFSQLException.class, logicalConnection::getAutoCommit);
    assertThrows(SFSQLException.class, logicalConnection::getMetaData);
    assertThrows(SFSQLException.class, logicalConnection::isReadOnly);
    assertThrows(SFSQLException.class, logicalConnection::getCatalog);
    assertThrows(SFSQLException.class, logicalConnection::getTransactionIsolation);
    assertThrows(SFSQLException.class, logicalConnection::getWarnings);
    assertThrows(SFSQLException.class, () -> logicalConnection.prepareCall("mocksql"));
    assertThrows(SFSQLException.class, logicalConnection::getTypeMap);
    assertThrows(SFSQLException.class, logicalConnection::getHoldability);
    assertThrows(SFSQLException.class, logicalConnection::createClob);
    assertThrows(SFSQLException.class, () -> logicalConnection.getClientInfo("mocksql"));
    assertThrows(SFSQLException.class, logicalConnection::getClientInfo);
    assertThrows(SFSQLException.class, () -> logicalConnection.createArrayOf("mock", null));
    assertThrows(SFSQLException.class, logicalConnection::getSchema);
    assertThrows(SFSQLException.class, logicalConnection::getNetworkTimeout);
    assertThrows(SFSQLException.class, () -> logicalConnection.setAutoCommit(false));
    assertThrows(SFSQLException.class, logicalConnection::rollback);
    assertThrows(SFSQLException.class, () -> logicalConnection.setReadOnly(false));
    assertThrows(SFSQLException.class, logicalConnection::clearWarnings);
    assertThrows(SFSQLException.class, () -> logicalConnection.setSchema(null));
    assertThrows(SFSQLException.class, () -> logicalConnection.setNetworkTimeout(null, 1));
    assertThrows(SFSQLException.class, () -> logicalConnection.prepareStatement("mocksql"));
    assertThrows(SFSQLException.class, () -> logicalConnection.prepareCall("mocksql", 1, 2, 3));
    assertThrows(SFSQLException.class, () -> logicalConnection.prepareCall("mocksql", 1, 2));
    assertThrows(SFSQLException.class, logicalConnection::commit);

    // Each assertThrows above is itself a delegation positive control: the SFSQLException carrier
    // wraps the exact stubbed physical SQLException, so it can only surface if the call was
    // delegated.
    // A few explicit verifies make that intent unambiguous for future readers.
    verify(connection).createStatement();
    verify(connection).commit();
    verify(connection).getSchema();
    verify(snowflakePooledConnection, times(28)).fireConnectionErrorEvent(sqlException);

    // isWrapperFor (like unwrap) is a type-resolution call: a physical SQLException propagates as
    // the carrier but must NOT fire connectionErrorOccurred, so it is intentionally excluded from
    // the count above.
    assertThrows(SFSQLException.class, () -> logicalConnection.isWrapperFor(Connection.class));
    verify(snowflakePooledConnection, times(28)).fireConnectionErrorEvent(sqlException);
  }

  @Test
  public void shouldAbortFailureFiresErrorEventMarksClosedAndDoesNotFireCloseEvent()
      throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    SQLException sqlException = new SQLException("abort failed");
    doThrow(sqlException).when(physicalConnection).abort(null);

    Connection logicalConnection = logical(pooledConnection);

    // A failed abort leaves the physical connection dead, so the handle stays closed and the pool
    // is told to discard it via an error event, never a (recycle) close event.
    assertThrows(SQLException.class, () -> logicalConnection.abort(null));
    assertTrue(logicalConnection.isClosed());
    // Positive control: the failure path must have actually invoked the physical abort.
    verify(physicalConnection).abort(null);
    verify(pooledConnection).fireConnectionErrorEvent(sqlException);
    verify(pooledConnection, never()).fireConnectionCloseEvent();

    // A subsequent close() must be a no-op: it must not fire a close event that would recycle the
    // already-dead physical connection back into the pool.
    logicalConnection.close();
    verify(pooledConnection, never()).fireConnectionCloseEvent();
  }

  @Test
  public void shouldAbortFeatureNotSupportedKeepsHandleOpenAndFiresNoEvents() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    doThrow(new SQLFeatureNotSupportedException("abort not supported"))
        .when(physicalConnection)
        .abort(null);

    Connection logicalConnection = logical(pooledConnection);

    // An unsupported abort does not touch the physical connection, so the handle remains usable and
    // no pool events are fired.
    assertThrows(SQLFeatureNotSupportedException.class, () -> logicalConnection.abort(null));
    assertFalse(logicalConnection.isClosed());
    // Positive control: abort must actually reach the physical connection (proves the handle was
    // claimed and then reopened on the unsupported result, not short-circuited before delegation).
    verify(physicalConnection).abort(null);
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
    verify(pooledConnection, never()).fireConnectionCloseEvent();
  }

  @Test
  public void shouldCloseDoesNotClosePhysicalConnection() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    // Logical close returns the handle to the pool; the physical connection must stay open.
    verify(pooledConnection).fireConnectionCloseEvent();
    verify(physicalConnection, never()).close();
  }

  @Test
  public void shouldIsValidOnClosedHandleReturnsFalseWithoutDelegatingOrFiringEvents()
      throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);

    Connection logicalConnection = logical(pooledConnection);
    logicalConnection.close();

    // A closed logical handle reports not-valid without throwing, touching the physical connection,
    // or signalling the pool.
    assertFalse(logicalConnection.isValid(10));
    verify(physicalConnection, never()).isValid(org.mockito.ArgumentMatchers.anyInt());
    verify(pooledConnection, never()).fireConnectionErrorEvent(org.mockito.ArgumentMatchers.any());
  }

  @Test
  public void shouldIsValidPhysicalFailureFiresErrorEvent() throws SQLException {
    SnowflakePooledConnection pooledConnection = mock(SnowflakePooledConnection.class);
    Connection physicalConnection = mock(Connection.class);
    when(pooledConnection.getPhysicalConnection()).thenReturn(physicalConnection);
    when(physicalConnection.isClosed()).thenReturn(false);
    SQLException sqlException = new SQLException("invalid timeout");
    when(physicalConnection.isValid(-1)).thenThrow(sqlException);

    Connection logicalConnection = logical(pooledConnection);

    assertThrows(SQLException.class, () -> logicalConnection.isValid(-1));
    // Positive control: the failing isValid must have been delegated to the physical connection.
    verify(physicalConnection).isValid(-1);
    verify(pooledConnection).fireConnectionErrorEvent(sqlException);
  }
}
