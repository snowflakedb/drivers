package net.snowflake.jdbc.e2e.pooling;

import static java.sql.Connection.TRANSACTION_READ_COMMITTED;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertGetConnectionAfterPooledCloseFails;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertIsValidFalseOnClosedHandle;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertNetworkTimeoutAfterSet;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertPhysicalConnectionClosedAfterAbort;
import static net.snowflake.jdbc.utils.PoolingTestCompat.closePooledConnection;
import static net.snowflake.jdbc.utils.PoolingTestCompat.invokeAbort;
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

import com.mchange.v2.c3p0.ComboPooledDataSource;
import com.zaxxer.hikari.HikariConfig;
import com.zaxxer.hikari.HikariDataSource;
import java.beans.PropertyVetoException;
import java.sql.CallableStatement;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Savepoint;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.Callable;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.DataSource;
import javax.sql.PooledConnection;
import javax.sql.StatementEvent;
import javax.sql.StatementEventListener;
import net.snowflake.client.api.pooling.SnowflakeConnectionPoolDataSource;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.pooling.SnowflakePooledConnection;
import net.snowflake.jdbc.utils.PoolingTestCompat;
import net.snowflake.jdbc.utils.PoolingTestResources;
import org.apache.commons.dbcp.BasicDataSource;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class ConnectionPoolTests extends PoolingE2ETestBase {

  private static final String TABLE_NAME = "test_pooling_" + PoolingTestResources.SUFFIX;
  private static final String EXPECTED_VALUE = "test_str";
  // Arbitrary non-default value; only needs to differ from the default (0) to prove the round-trip.
  private static final int NON_DEFAULT_LOGIN_TIMEOUT_SECONDS = 42;

  @Test
  public void shouldBorrowLogicalConnectionAndKeepPhysicalConnectionAliveAfterClose()
      throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    // When A logical connection is borrowed and closed
    SnowflakeConnectionImpl firstPhysicalConnection;
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertFalse(logicalConnection.isClosed());
      // Capture the physical connection by identity before the handle is returned to the pool.
      firstPhysicalConnection = logicalConnection.unwrap(SnowflakeConnectionImpl.class);
      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }

    // Then A new logical connection can run queries on the same physical connection
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      assertSame(firstPhysicalConnection, logicalConnection2.unwrap(SnowflakeConnectionImpl.class));
      try (Statement stmt = logicalConnection2.createStatement();
          ResultSet rs = stmt.executeQuery("SELECT 1")) {
        assertTrue(rs.next());
        assertEquals(1, rs.getInt(1));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldFireConnectionClosedEventWhenLogicalConnectionCloses() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A connection event listener is registered
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    // When A logical connection is borrowed and closed
    SnowflakeConnectionImpl physicalConnection;
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // Capture the physical connection by identity to prove the same one survives the close.
      physicalConnection = logicalConnection.unwrap(SnowflakeConnectionImpl.class);
      try (Statement stmt = logicalConnection.createStatement()) {
        stmt.execute("SELECT 1");
      }
    }

    // Then A connection closed event should be fired without error
    assertEquals(1, listener.closedEvents.size());
    ConnectionEvent closedEvent = listener.closedEvents.get(0);
    assertNull(closedEvent.getSQLException());
    assertInstanceOf(SnowflakePooledConnection.class, closedEvent.getSource());
    assertSame(pooledConnection, closedEvent.getSource());

    // And The physical connection should remain open
    assertFalse(physicalConnection.isClosed());
    try (Connection logicalConnection2 = pooledConnection.getConnection()) {
      assertFalse(logicalConnection2.isClosed());
      // The re-borrowed logical handle must be backed by the very same physical connection.
      assertSame(physicalConnection, logicalConnection2.unwrap(SnowflakeConnectionImpl.class));
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
  public void shouldFireConnectionErrorEventWhenLogicalConnectionOperationFails() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A connection event listener is registered
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    // When An invalid catalog is set on a logical connection
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      SQLException thrown =
          assertThrows(
              SQLException.class,
              () -> logicalConnection.setCatalog("nonexistent_database_xyz_9999"));

      // Then A connection error event should be fired with matching error code
      assertEquals(1, listener.errorEvents.size());
      ConnectionEvent errorEvent = listener.errorEvents.get(0);
      assertInstanceOf(SnowflakePooledConnection.class, errorEvent.getSource());
      assertSame(pooledConnection, errorEvent.getSource());
      assertEquals(thrown.getErrorCode(), errorEvent.getSQLException().getErrorCode());
    }

    pooledConnection.close();
  }

  @Test
  public void shouldGetPooledConnectionWithUsernameAndPassword() throws Exception {
    // The (user, password) overload is password-specific; skip in JWT-only environments where
    // SNOWFLAKE_TEST_PASSWORD is not configured.
    assumeTrue(
        has("SNOWFLAKE_TEST_PASSWORD"),
        "Skipping credential-overload test: SNOWFLAKE_TEST_PASSWORD not configured");
    // Given Snowflake connection pool data source is configured
    SnowflakeConnectionPoolDataSource poolDataSource = createPasswordConfiguredPoolDataSource();

    // When A pooled connection is obtained with username and password
    PooledConnection pooledConnection =
        trackPooledConnection(poolDataSource.getPooledConnection(getUser(), getPassword()));

    // Then A query can be executed on the logical connection
    try (Connection logicalConnection = pooledConnection.getConnection();
        Statement statement = logicalConnection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT 1")) {
      // Assert the query returns a result so the test fails if the credential overload produces an
      // unusable connection rather than passing vacuously on a no-throw execute().
      assertTrue(resultSet.next());
      assertEquals(1, resultSet.getInt(1));
    }
    pooledConnection.close();
  }

  @Test
  public void shouldRejectBorrowingALogicalConnectionAfterThePooledConnectionIsClosed()
      throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    // When The pooled connection is closed
    pooledConnection.close();

    // Universal driver throws CONNECTION_CLOSED; legacy throws NullPointerException (BD#27) - the
    // compat helper asserts the per-driver failure mode.
    // Then Borrowing a logical connection should raise connection closed error
    assertGetConnectionAfterPooledCloseFails(pooledConnection);
  }

  @Test
  public void shouldInvalidateThePreviousLogicalConnectionWhenANewOneIsBorrowed() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    Connection firstLogicalConnection = pooledConnection.getConnection();
    Connection secondLogicalConnection = null;
    // The javax.sql.PooledConnection contract allows one active handle: the universal driver
    // invalidates the prior handle (BD#36). assertThrowsConnectionClosed is a no-op on the legacy
    // reference driver, which does not enforce single-active-handle.
    try {
      assertEquals(1, querySingleInt(firstLogicalConnection, "SELECT 1"));

      // When A second logical connection is borrowed while the first is still open
      secondLogicalConnection = pooledConnection.getConnection();

      // Then The first logical connection should be invalidated and the second should run queries
      PoolingTestCompat.assertThrowsConnectionClosed(firstLogicalConnection::createStatement);
      assertEquals(2, querySingleInt(secondLogicalConnection, "SELECT 2"));
    } finally {
      firstLogicalConnection.close();
      if (secondLogicalConnection != null) {
        secondLogicalConnection.close();
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldAcceptStatementEventListenersWithoutFiringStatementEvents() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A statement event listener is registered
    TestStatementListener statementListener = new TestStatementListener();
    pooledConnection.addStatementEventListener(statementListener);

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When A statement is prepared and executed on a logical connection
      try (PreparedStatement preparedStatement = logicalConnection.prepareStatement("SELECT 1");
          ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next());
        assertEquals(1, resultSet.getInt(1));
      }
    }

    // Snowflake does no statement pooling, so statement-event listeners are accepted no-ops on both
    // drivers (BD#39): a registered listener must never receive a callback.
    // Then No statement events should be fired and the listener can be removed
    assertEquals(0, statementListener.closedEvents.size());
    assertEquals(0, statementListener.errorEvents.size());
    pooledConnection.removeStatementEventListener(statementListener);
    pooledConnection.close();
  }

  @Test
  public void shouldStopFiringConnectionEventsAfterTheListenerIsRemoved() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A connection event listener is registered
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    // When The listener is removed after one logical connection close and another handle is closed
    try (Connection firstLogicalConnection = pooledConnection.getConnection()) {
      assertFalse(firstLogicalConnection.isClosed());
    }
    assertEquals(1, listener.closedEvents.size());
    pooledConnection.removeConnectionEventListener(listener);
    try (Connection secondLogicalConnection = pooledConnection.getConnection()) {
      assertFalse(secondLogicalConnection.isClosed());
    }

    // Then Only the close before removal should be delivered to the listener
    assertEquals(1, listener.closedEvents.size());
    pooledConnection.close();
  }

  @Test
  public void shouldIsolateConnectionEventListenerExceptionsDuringDispatch() throws Exception {
    // The universal driver isolates listener exceptions during dispatch (BD#28); the legacy
    // reference driver aborts dispatch on the first throwing listener, so gate to the universal
    // driver rather than asserting behavior the legacy driver does not provide.
    assumeTrue(
        PoolingTestCompat.isUniversalDriverPooling(),
        "Listener-isolation dispatch is universal-driver behavior (BD#28)");
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A failing and a recording connection event listener are registered
    ConnectionEventListener failingListener =
        new ConnectionEventListener() {
          @Override
          public void connectionClosed(ConnectionEvent event) {
            throw new RuntimeException("listener failure");
          }

          @Override
          public void connectionErrorOccurred(ConnectionEvent event) {
            throw new RuntimeException("listener failure");
          }
        };
    TestConnectionListener recordingListener = new TestConnectionListener();
    // Register the failing listener first so its thrown exception would abort dispatch to the
    // recording listener if exceptions were not isolated.
    pooledConnection.addConnectionEventListener(failingListener);
    pooledConnection.addConnectionEventListener(recordingListener);

    // When A logical connection is borrowed and closed
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertFalse(logicalConnection.isClosed());
    }

    // Then The recording listener should still receive the connection closed event
    assertEquals(1, recordingListener.closedEvents.size());
    pooledConnection.removeConnectionEventListener(failingListener);
    pooledConnection.removeConnectionEventListener(recordingListener);
    pooledConnection.close();
  }

  @Test
  public void shouldGetAndSetLoginTimeoutOnThePoolDataSource() throws Exception {
    // Given Snowflake connection pool data source is configured
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();

    // When The login timeout is set on the pool data source
    poolDataSource.setLoginTimeout(NON_DEFAULT_LOGIN_TIMEOUT_SECONDS);

    // Then The login timeout getter should return the configured value
    assertEquals(NON_DEFAULT_LOGIN_TIMEOUT_SECONDS, poolDataSource.getLoginTimeout());
  }

  @Test
  public void shouldRunConcurrentQueriesOnSeparateLogicalConnections() throws Exception {
    // Given Snowflake connection pool data source is configured
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    int[] expectedValues = {2837, 6104, 1592, 8471, 3963};
    ExecutorService executor = Executors.newFixedThreadPool(expectedValues.length);
    // Release all workers at once so the queries genuinely overlap.
    CountDownLatch startLatch = new CountDownLatch(1);
    // javax.sql.PooledConnection allows at most one active logical handle per pooled connection, so
    // each worker borrows its own PooledConnection (each with one logical handle). The barrier
    // proves all workers hold live logical connections concurrently before any query runs.
    CountDownLatch allBorrowed = new CountDownLatch(expectedValues.length);

    try {
      List<Future<Integer>> futures = new ArrayList<>();
      for (int value : expectedValues) {
        futures.add(
            executor.submit(
                () -> {
                  startLatch.await();
                  PooledConnection pooledConnection = poolDataSource.getPooledConnection();
                  try (Connection connection = pooledConnection.getConnection()) {
                    allBorrowed.countDown();
                    assertTrue(allBorrowed.await(60, TimeUnit.SECONDS));
                    try (Statement statement = connection.createStatement();
                        ResultSet resultSet = statement.executeQuery("SELECT " + value + " AS N")) {
                      resultSet.next();
                      return resultSet.getInt(1);
                    }
                  } finally {
                    pooledConnection.close();
                  }
                }));
      }

      // When Concurrent queries are executed on separate logical connections
      startLatch.countDown();
      List<Integer> results = new ArrayList<>();
      for (Future<Integer> future : futures) {
        results.add(future.get(60, TimeUnit.SECONDS));
      }

      // Then Each query should return its distinct value
      assertEquals(
          Arrays.stream(expectedValues).boxed().sorted().collect(Collectors.toList()),
          results.stream().sorted().collect(Collectors.toList()));
    } finally {
      executor.shutdownNow();
    }
  }

  @Test
  public void shouldPropagateErrorsFromLogicalConnectionOperations() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection();
        Statement statement = logicalConnection.createStatement()) {
      // When Invalid SQL is executed on a logical connection
      SQLException error =
          assertThrows(
              SQLException.class, () -> statement.execute("SELECT FROM non_existent_table"));
      // Then A SQL exception should be raised
      assertNotNull(error.getSQLState(), "expected a SQLSTATE on the propagated server error");
    } finally {
      pooledConnection.close();
    }
  }

  @Test
  public void shouldReusePhysicalConnectionAfterLogicalCloseAndReborrow() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    Connection firstLogicalConnection = pooledConnection.getConnection();
    SnowflakeConnectionImpl firstPhysicalConnection;
    try {
      // Both logical handles unwrap to the one physical connection the pool owns; capturing the
      // instance before close lets us prove reuse by identity (getSessionID is not implemented
      // yet).
      firstPhysicalConnection = firstLogicalConnection.unwrap(SnowflakeConnectionImpl.class);
      assertEquals(1, querySingleInt(firstLogicalConnection, "SELECT 1"));
    } finally {
      // When A logical connection is borrowed released and borrowed again
      firstLogicalConnection.close();
    }

    Connection secondLogicalConnection = pooledConnection.getConnection();
    try {
      // Then The second logical connection should execute queries successfully
      assertEquals(2, querySingleInt(secondLogicalConnection, "SELECT 2"));
      assertFalse(secondLogicalConnection.isClosed());
      // And Both borrows should use the same underlying physical connection
      assertSame(
          firstPhysicalConnection, secondLogicalConnection.unwrap(SnowflakeConnectionImpl.class));
    } finally {
      secondLogicalConnection.close();
      pooledConnection.close();
    }
  }

  private static int querySingleInt(Connection connection, String sql) throws SQLException {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());
      return resultSet.getInt(1);
    }
  }

  @Test
  public void shouldRejectOperationsOnClosedLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When A logical connection is closed
      logicalConnection.close();

      // Then Subsequent operations should raise connection closed error
      expectConnectionClosed(logicalConnection::getMetaData);
      expectConnectionClosed(logicalConnection::getAutoCommit);
      expectConnectionClosed(logicalConnection::commit);
      expectConnectionClosed(logicalConnection::rollback);
      expectConnectionClosed(logicalConnection::isReadOnly);
      expectConnectionClosed(logicalConnection::getCatalog);
      expectConnectionClosed(logicalConnection::getSchema);
      expectConnectionClosed(logicalConnection::getTransactionIsolation);
      expectConnectionClosed(logicalConnection::getWarnings);
      expectConnectionClosed(logicalConnection::clearWarnings);
      expectConnectionClosed(() -> logicalConnection.nativeSQL("select 1"));
      expectConnectionClosed(() -> logicalConnection.setAutoCommit(false));
      expectConnectionClosed(() -> logicalConnection.setReadOnly(false));
      expectConnectionClosed(() -> logicalConnection.setCatalog("fakedb"));
      expectConnectionClosed(() -> logicalConnection.setSchema("fakedb"));
      expectConnectionClosed(
          () -> logicalConnection.setTransactionIsolation(TRANSACTION_READ_COMMITTED));
      expectConnectionClosed(() -> logicalConnection.createArrayOf("faketype", null));
      expectConnectionClosed(logicalConnection::createStatement);
      expectConnectionClosed(() -> logicalConnection.prepareStatement("select 1"));
      expectConnectionClosed(() -> logicalConnection.prepareCall("call foo()"));
      expectConnectionClosed(() -> logicalConnection.unwrap(SnowflakeConnectionImpl.class));
      expectConnectionClosed(() -> logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class));
      // isValid() on a closed handle is a no-throw false (universal driver); legacy may still
      // probe.
      assertIsValidFalseOnClosedHandle(logicalConnection);
    }

    pooledConnection.close();
  }

  @Test
  public void shouldRejectUnsupportedFeaturesOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Unsupported JDBC features are invoked on a logical connection
      Connection connectionUnderTest = logicalConnection;
      // Then SQLFeatureNotSupportedException should be raised
      expectFeatureNotSupported(() -> connectionUnderTest.rollback(new FakeSavepoint()));
      expectFeatureNotSupported(
          () -> logicalConnection.setTransactionIsolation(Connection.TRANSACTION_SERIALIZABLE));
      expectFeatureNotSupported(
          () -> logicalConnection.setTransactionIsolation(Connection.TRANSACTION_REPEATABLE_READ));
      expectFeatureNotSupported(
          () -> logicalConnection.prepareStatement("select 1", new int[] {1, 2}));
      expectFeatureNotSupported(
          () -> logicalConnection.prepareStatement("select 1", new String[] {"c1", "c2"}));
      expectFeatureNotSupported(
          () ->
              logicalConnection.prepareStatement(
                  "select 1", ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY));
      expectFeatureNotSupported(
          () ->
              logicalConnection.prepareStatement(
                  "select 1",
                  ResultSet.TYPE_SCROLL_SENSITIVE,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.HOLD_CURSORS_OVER_COMMIT));
      expectFeatureNotSupported(
          () ->
              logicalConnection.createStatement(
                  ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY));
      expectFeatureNotSupported(() -> logicalConnection.setTypeMap(new HashMap<>()));
      expectFeatureNotSupported(logicalConnection::setSavepoint);
      expectFeatureNotSupported(() -> logicalConnection.setSavepoint("fake"));
      expectFeatureNotSupported(() -> logicalConnection.releaseSavepoint(new FakeSavepoint()));
      expectFeatureNotSupported(logicalConnection::createBlob);
      expectFeatureNotSupported(logicalConnection::createNClob);
      expectFeatureNotSupported(logicalConnection::createSQLXML);
      expectFeatureNotSupported(
          () -> logicalConnection.setHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT));
      expectFeatureNotSupported(() -> logicalConnection.createStruct("fakeType", new Object[] {}));
      expectFeatureNotSupported(
          () -> logicalConnection.prepareStatement("select 1", Statement.RETURN_GENERATED_KEYS));

      // Unsupported features are caller errors, not physical-connection failures: the pool must not
      // have been notified that the connection is broken (universal driver only; legacy may fire).
      PoolingTestCompat.assertNoConnectionErrorEventsForUnsupportedFeatures(
          listener.errorEvents.size());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldCreateStatementWithHoldabilityOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    // When A statement is created with holdability on a logical connection
    try (Connection logicalConnection = pooledConnection.getConnection();
        Statement statement =
            logicalConnection.createStatement(
                ResultSet.TYPE_FORWARD_ONLY,
                ResultSet.CONCUR_READ_ONLY,
                ResultSet.CLOSE_CURSORS_AT_COMMIT);
        ResultSet resultSet = statement.executeQuery("show parameters")) {
      // Then The statement should execute and report expected holdability
      assertTrue(resultSet.next());
      assertFalse(logicalConnection.isClosed());
      assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, statement.getResultSetHoldability());
      assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, logicalConnection.getHoldability());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldValidateLogicalConnectionLivenessWithTimeoutLabelTimeout() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Connection validity is checked with <timeout_seconds> second timeout on a logical
      // connection
      assertTrue(logicalConnection.isValid(10));
      // Then Validation should <outcome>
    }
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Connection validity is checked with <timeout_seconds> second timeout on a logical
      // connection
      assertThrows(SQLException.class, () -> logicalConnection.isValid(-10));
      // Then Validation should <outcome>
    }
    pooledConnection.close();
  }

  @Test
  public void shouldReadClientInfoFromLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Client info is read from a logical connection
      Properties clientInfo = logicalConnection.getClientInfo();
      String namedClientInfo = logicalConnection.getClientInfo("Peter");
      // Then Client info should be empty and unknown keys return null
      assertEquals(0, clientInfo.size());
      assertNull(namedClientInfo);
    }
    pooledConnection.close();
  }

  @Test
  public void shouldExecuteTransactionStatementsOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      logicalConnection.setAutoCommit(false);
      assertFalse(logicalConnection.getAutoCommit());
      logicalConnection.setTransactionIsolation(TRANSACTION_READ_COMMITTED);
      assertEquals(TRANSACTION_READ_COMMITTED, logicalConnection.getTransactionIsolation());

      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute(
            "create or replace temporary table test_transaction (colA int, colB string)");
        statement.execute("insert into test_transaction values (1, 'abc')");

        // When Commit and rollback are used on a logical connection
        logicalConnection.commit();
        try (ResultSet resultSet =
            statement.executeQuery("select count(*) from test_transaction")) {
          // Then Transaction state should reflect DML changes correctly
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
        }

        statement.execute("delete from test_transaction");
        logicalConnection.rollback();
        try (ResultSet resultSet =
            statement.executeQuery("select count(*) from test_transaction")) {
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldSetAndReadSchemaOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      String schema = logicalConnection.getSchema();
      try (Statement statement = logicalConnection.createStatement();
          ResultSet rst = statement.executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals(schema, rst.getString(1));
      }

      // When Schema is read and set on a logical connection
      logicalConnection.setSchema("PUBLIC");

      // Then Current schema should match database state
      try (Statement statement = logicalConnection.createStatement();
          ResultSet rst = statement.executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals("PUBLIC", rst.getString(1));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldExposeDatabaseMetadataOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Database metadata is requested from a logical connection
      DatabaseMetaData databaseMetaData = logicalConnection.getMetaData();
      // Then Product name should be Snowflake
      assertEquals("Snowflake", databaseMetaData.getDatabaseProductName());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldUnwrapLogicalConnectionToSnowflakeConnectionImplementation() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Logical connection wrapper is inspected
      boolean canUnwrap = logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class);
      // Then It should be a wrapper for SnowflakeConnectionImpl
      assertTrue(canUnwrap);
      // unwrap() must actually return the live physical connection, not merely report wrapper-ness.
      SnowflakeConnectionImpl physical = logicalConnection.unwrap(SnowflakeConnectionImpl.class);
      assertFalse(physical.isClosed());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldExecutePreparedStatementOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute("create or replace temporary table test_prep (colA int, colB varchar)");
        try (PreparedStatement preparedStatement =
            logicalConnection.prepareStatement("insert into test_prep values (?, ?)")) {
          // When A prepared statement inserts rows on a logical connection
          preparedStatement.setInt(1, 25);
          preparedStatement.setString(2, "hello world");
          preparedStatement.execute();
          int count = 0;
          try (ResultSet resultSet = statement.executeQuery("select colA, colB from test_prep")) {
            while (resultSet.next()) {
              count++;
              // The values read back must match what the prepared statement bound and inserted.
              assertEquals(25, resultSet.getInt(1));
              assertEquals("hello world", resultSet.getString(2));
            }
          }
          // Then Inserted rows should be readable
          assertEquals(1, count);
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldReturnNativeSqlUnchangedOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When nativeSQL is called on a logical connection
      String nativeSql = logicalConnection.nativeSQL("select 1");
      // Then The SQL text should be returned unchanged
      assertEquals("select 1", nativeSql);
    }
    pooledConnection.close();
  }

  @Test
  public void shouldQueryAndSetReadOnlyStateOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Read-only state is queried and set on a logical connection
      assertFalse(logicalConnection.isReadOnly());
      logicalConnection.setReadOnly(true);
      // Then Read-only should remain false
      assertFalse(logicalConnection.isReadOnly());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldReturnEmptyTypeMapOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Type map is read from a logical connection
      java.util.Map<String, Class<?>> typeMap = logicalConnection.getTypeMap();
      // Then An empty map should be returned
      assertEquals(Collections.emptyMap(), typeMap);
    }
    pooledConnection.close();
  }

  @Test
  public void shouldGetAndSetNetworkTimeoutOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Network timeout is read and updated on a logical connection
      assertEquals(0, logicalConnection.getNetworkTimeout());
      logicalConnection.setNetworkTimeout(null, 200);
      // Then Network timeout getter returns zero because it is not yet wired to sf_core
      assertNetworkTimeoutAfterSet(logicalConnection, 200);
    }
    pooledConnection.close();
  }

  @Test
  public void shouldClosePhysicalConnectionWhenLogicalConnectionIsAborted() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    Connection logicalConnection = pooledConnection.getConnection();
    SnowflakeConnectionImpl physicalConnection =
        logicalConnection.unwrap(SnowflakeConnectionImpl.class);
    assertFalse(physicalConnection.isClosed());

    // When A logical connection is aborted
    invokeAbort(logicalConnection);

    // Then The underlying physical connection should be closed
    assertPhysicalConnectionClosedAfterAbort(physicalConnection, logicalConnection);
    closePooledConnection(pooledConnection);
  }

  @Test
  public void shouldExecuteCallableStatementOnLogicalConnection() throws Exception {
    String procedureName = "output_message_" + PoolingTestResources.SUFFIX;
    String procedure =
        "CREATE OR REPLACE PROCEDURE "
            + procedureName
            + "(message VARCHAR)\n"
            + "RETURNS VARCHAR NOT NULL\n"
            + "LANGUAGE SQL\n"
            + "AS\n"
            + "BEGIN\n"
            + "  RETURN message;\n"
            + "END;";
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute(procedure);

        try {
          // When Callable statements are prepared and executed on a logical connection
          try (CallableStatement callableStatement =
              logicalConnection.prepareCall("call " + procedureName + "(?)")) {
            callableStatement.setString(1, "hello world");
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
            }
          }

          try (CallableStatement callableStatement =
              logicalConnection.prepareCall(
                  "call " + procedureName + "('hello world')",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY)) {
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
              assertEquals(ResultSet.TYPE_FORWARD_ONLY, callableStatement.getResultSetType());
              assertEquals(ResultSet.CONCUR_READ_ONLY, callableStatement.getResultSetConcurrency());
            }
          }

          try (CallableStatement callableStatement =
              logicalConnection.prepareCall(
                  "call " + procedureName + "('hello world')",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
              assertEquals(
                  ResultSet.CLOSE_CURSORS_AT_COMMIT, callableStatement.getResultSetHoldability());
            }
          }
        } finally {
          // Then Stored procedure results and statement properties should match expectations
          statement.execute("drop procedure if exists " + procedureName + "(varchar)");
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldCreateAndBindClobOnLogicalConnection() throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());

    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute("create or replace temporary table test_clob (colA text)");
      }

      // When A clob value is created and inserted via prepared statement on a logical connection
      try (PreparedStatement preparedStatement =
          logicalConnection.prepareStatement("insert into test_clob values (?)")) {
        Clob clob = logicalConnection.createClob();
        clob.setString(1, "hello world");
        preparedStatement.setClob(1, clob);
        preparedStatement.execute();
      }

      try (Statement statement = logicalConnection.createStatement();
          ResultSet resultSet = statement.executeQuery("select * from test_clob")) {
        // Then The inserted clob value should be readable
        assertTrue(resultSet.next());
        assertEquals("hello world", resultSet.getString("COLA"));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldFireConnectionErrorEventWhenPhysicalConnectionDelegatesThrow()
      throws Exception {
    // Given Snowflake connection pool data source is configured
    PooledConnection pooledConnection =
        trackPooledConnection(createConfiguredPoolDataSource().getPooledConnection());
    // And A connection event listener is registered
    TestConnectionListener listener = new TestConnectionListener();
    pooledConnection.addConnectionEventListener(listener);

    // Only Connection-interface delegate failures flow through the logical wrapper and fire
    // connection error events; Statement-level failures do not (no logical Statement wrapper).
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // When Physical connection delegate operations throw SQLException on a logical connection
      assertThrows(
          SQLException.class, () -> logicalConnection.setCatalog("nonexistent_database_xyz_9999"));
      assertThrows(
          SQLException.class, () -> logicalConnection.setSchema("nonexistent_schema_xyz_9999"));
      assertThrows(SQLException.class, () -> logicalConnection.isValid(-10));
      // Then Connection error event should be fired for each failing delegate operation
      assertEquals(3, listener.errorEvents.size());
      // Each error event must carry the originating SQLException and reference the pooled conn.
      for (ConnectionEvent errorEvent : listener.errorEvents) {
        assertNotNull(errorEvent.getSQLException(), "error event should carry the SQLException");
        assertSame(pooledConnection, errorEvent.getSource());
      }
    }

    pooledConnection.removeConnectionEventListener(listener);
    pooledConnection.close();
  }

  private void expectConnectionClosed(SQLErrorThrowingRunnable f) {
    PoolingTestCompat.assertThrowsConnectionClosed(f::run);
  }

  private void expectFeatureNotSupported(SQLErrorThrowingRunnable f) {
    SQLException ex = assertThrows(SQLException.class, f::run);
    assertInstanceOf(
        SQLFeatureNotSupportedException.class,
        ex,
        "Expected SQLFeatureNotSupportedException but got " + ex.getClass().getName());
  }

  static class FakeSavepoint implements Savepoint {
    @Override
    public int getSavepointId() throws SQLException {
      return 0;
    }

    @Override
    public String getSavepointName() throws SQLException {
      return "";
    }
  }

  /**
   * Fully-qualified ({@code db.schema.table}) name so connections that do not share the default
   * connection's session schema - e.g. the third-party pool (Hikari/C3P0/DBCP) connections - can
   * still resolve the table.
   */
  private String qualifiedTableName() {
    Properties props = getConnectionProperties();
    return props.getProperty("db") + "." + props.getProperty("schema") + "." + TABLE_NAME;
  }

  @BeforeEach
  void setUpPoolingTable() throws Exception {
    Connection connection = getDefaultConnection();
    try (Statement statement = connection.createStatement()) {
      statement.execute("create or replace table " + qualifiedTableName() + " (colA string)");
      statement.execute(
          "insert into " + qualifiedTableName() + " values('" + EXPECTED_VALUE + "')");
    }
  }

  @AfterEach
  void tearDownPoolingTable() throws Exception {
    Connection connection = getDefaultConnection();
    try (Statement statement = connection.createStatement()) {
      statement.execute("drop table if exists " + qualifiedTableName());
    }
  }

  @Test
  public void shouldQueryThroughPoolConnectionPoolWithThreadCountThreads() throws Exception {
    // Given A temporary pooling query table exists
    /*
     * getDefaultConnection() returns the shared per-class connection, so it must NOT be closed here
     * (closing it would break @AfterEach teardown and every subsequent test in the class).
     */
    Connection connection = getDefaultConnection();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("select colA from " + qualifiedTableName())) {
      assertTrue(resultSet.next());
      assertEquals(EXPECTED_VALUE, resultSet.getString(1));
    }

    // When <thread_count> threads query through <pool> connection pool
    HikariDataSource hikariDataSource = createHikariDataSource(10);
    List<String> hikariResults;
    try {
      hikariResults = runConcurrentPoolQueries(hikariDataSource, 10);
    } finally {
      hikariDataSource.close();
    }
    ComboPooledDataSource c3p0DataSource = createC3P0DataSource(10);
    List<String> c3p0Results;
    try {
      c3p0Results = runConcurrentPoolQueries(c3p0DataSource, 10);
    } finally {
      c3p0DataSource.close();
    }
    BasicDataSource dbcpDataSource = createDbcpDataSource(10);
    List<String> dbcpResults;
    try {
      dbcpResults = runConcurrentPoolQueries(dbcpDataSource, 10);
    } finally {
      dbcpDataSource.close();
    }
    // Then Each thread should read the expected row
    assertEachThreadReadExpectedRow(hikariResults, 10);
    assertEachThreadReadExpectedRow(c3p0Results, 10);
    assertEachThreadReadExpectedRow(dbcpResults, 10);
  }

  private static void assertEachThreadReadExpectedRow(List<String> values, int threadCount) {
    assertEquals(threadCount, values.size());
    for (String value : values) {
      assertEquals(EXPECTED_VALUE, value);
    }
  }

  private List<String> runConcurrentPoolQueries(DataSource dataSource, int threadCount)
      throws Exception {
    ExecutorService executor = Executors.newFixedThreadPool(threadCount);
    CountDownLatch startLatch = new CountDownLatch(1);
    // allBorrowed proves the pool actually hands out threadCount connections simultaneously: every
    // worker holds its borrowed connection open until all of them have been borrowed, so the test
    // cannot pass if the pool serialized the checkouts.
    CountDownLatch allBorrowed = new CountDownLatch(threadCount);
    List<Future<String>> futures = new ArrayList<>();
    try {
      for (int i = 0; i < threadCount; i++) {
        futures.add(executor.submit(new PoolQueryTask(dataSource, startLatch, allBorrowed)));
      }
      // Release all workers simultaneously so they genuinely contend on the pool.
      startLatch.countDown();
      List<String> results = new ArrayList<>();
      for (Future<String> future : futures) {
        results.add(future.get(120, TimeUnit.SECONDS));
      }
      return results;
    } finally {
      executor.shutdownNow();
    }
  }

  private HikariDataSource createHikariDataSource(int maxPoolSize) {
    HikariConfig config = new HikariConfig();
    config.setDriverClassName(DRIVER_CLASS);
    config.setJdbcUrl(getJdbcUrl());
    config.setDataSourceProperties(createDriverManagerProperties());
    config.setMaximumPoolSize(maxPoolSize);
    return new HikariDataSource(config);
  }

  private ComboPooledDataSource createC3P0DataSource(int maxPoolSize)
      throws PropertyVetoException, SQLException {
    ComboPooledDataSource dataSource = new ComboPooledDataSource();
    dataSource.setDriverClass(DRIVER_CLASS);
    dataSource.setJdbcUrl(getJdbcUrl());
    dataSource.setProperties(createDriverManagerProperties());
    // Pre-open the full pool (default initial/min is ~3) so the allBorrowed barrier does not race
    // C3P0's lazy session creation under a tight timeout on a cold pool.
    dataSource.setInitialPoolSize(maxPoolSize);
    dataSource.setMinPoolSize(maxPoolSize);
    dataSource.setMaxPoolSize(maxPoolSize);
    return dataSource;
  }

  private BasicDataSource createDbcpDataSource(int maxTotal) {
    Properties properties = getConnectionProperties();
    BasicDataSource dataSource = new BasicDataSource();
    dataSource.setDriverClassName(DRIVER_CLASS);
    dataSource.setUsername(properties.getProperty("user"));
    dataSource.setUrl(getJdbcUrl());
    dataSource.setConnectionProperties(buildDbcpConnectionProperties(properties));
    // Propagate the active auth method as discrete connection properties. In CI this is key pair
    // (SNOWFLAKE_JWT); locally it may be password. Added via addConnectionProperty so the base64
    // key
    // is not embedded in the ';'-delimited connection-properties string.
    if (properties.getProperty("password") != null) {
      dataSource.setPassword(properties.getProperty("password"));
    }
    if (properties.getProperty("authenticator") != null) {
      dataSource.addConnectionProperty("authenticator", properties.getProperty("authenticator"));
    }
    if (properties.getProperty("private_key_base64") != null) {
      dataSource.addConnectionProperty(
          "private_key_base64", properties.getProperty("private_key_base64"));
    }
    if (properties.getProperty("private_key_pwd") != null) {
      dataSource.addConnectionProperty(
          "private_key_pwd", properties.getProperty("private_key_pwd"));
    }
    // DBCP 1.x defaults maxActive to 8, below the worker count, which would deadlock the
    // allBorrowed
    // barrier; size the pool to the number of concurrent workers and pre-open it.
    dataSource.setMaxActive(maxTotal);
    dataSource.setInitialSize(maxTotal);
    return dataSource;
  }

  private static String buildDbcpConnectionProperties(Properties properties) {
    StringBuilder builder =
        new StringBuilder(
            String.format(
                "account=%s;db=%s;schema=%s;warehouse=%s;",
                properties.getProperty("account"),
                properties.getProperty("db"),
                properties.getProperty("schema"),
                properties.getProperty("warehouse")));
    // Include role for parity with the Hikari/C3P0 pools (createDriverManagerProperties copies it),
    // so DBCP checkouts resolve the fixture table under the same role it was created with.
    String role = properties.getProperty("role");
    if (role != null) {
      builder.append("role=").append(role).append(';');
    }
    return builder.toString();
  }

  private final class PoolQueryTask implements Callable<String> {
    private final DataSource dataSource;
    private final CountDownLatch startLatch;
    private final CountDownLatch allBorrowed;

    private PoolQueryTask(
        DataSource dataSource, CountDownLatch startLatch, CountDownLatch allBorrowed) {
      this.dataSource = dataSource;
      this.startLatch = startLatch;
      this.allBorrowed = allBorrowed;
    }

    @Override
    public String call() {
      try {
        startLatch.await();
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new AssertionError("Interrupted before pool query", e);
      }
      try (Connection connection = dataSource.getConnection();
          Statement statement = connection.createStatement()) {
        // Signal that this worker holds a connection, then wait until every worker does. This keeps
        // all threadCount connections checked out at once, forcing genuine pool concurrency.
        allBorrowed.countDown();
        if (!allBorrowed.await(60, TimeUnit.SECONDS)) {
          throw new AssertionError("Timed out waiting for all workers to borrow a connection");
        }
        try (ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + qualifiedTableName())) {
          String value = null;
          int rowCount = 0;
          while (resultSet.next()) {
            value = resultSet.getString(1);
            rowCount++;
          }
          if (rowCount == 0) {
            throw new AssertionError("Pool query returned no rows");
          }
          return value;
        }
      } catch (Exception e) {
        throw new AssertionError("Pool query failed", e);
      }
    }
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

  static class TestStatementListener implements StatementEventListener {
    final List<StatementEvent> closedEvents = new ArrayList<>();
    final List<StatementEvent> errorEvents = new ArrayList<>();

    @Override
    public void statementClosed(StatementEvent event) {
      closedEvents.add(event);
    }

    @Override
    public void statementErrorOccurred(StatementEvent event) {
      errorEvents.add(event);
    }
  }
}
