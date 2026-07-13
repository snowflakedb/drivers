package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.jdbc.utils.TestParameters.props;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.anyInt;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.clearInvocations;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.CallableStatement;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.Statement;
import java.util.Properties;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetAutocommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

class SnowflakeConnectionImplTest {

  private static final String MOCK_PASSWORD = "***";

  @Test
  void stripVersionSuffixReturnsInputWhenNoSpace() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1"));
  }

  @Test
  void stripVersionSuffixDropsBuildSuffix() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 abcdef"));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 a b c"));
  }

  @Test
  void stripVersionSuffixHandlesEmptyAndNull() {
    assertNull(SnowflakeConnectionImpl.stripVersionSuffix(null));
    assertEquals("", SnowflakeConnectionImpl.stripVersionSuffix(""));
    assertEquals("", SnowflakeConnectionImpl.stripVersionSuffix("   "));
  }

  @Test
  void stripVersionSuffixTrimsLeadingAndTrailingWhitespace() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix(" 8.46.1"));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 "));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("  8.46.1 abc  "));
  }

  @Nested
  class Close {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void sendsConnectionCloseAndReleasesHandles() throws Exception {
      Connection conn = createConnection();
      conn.close();

      verify(mockCoreApi).connectionClose(any());
      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
    }

    @Test
    void isClosedReturnsTrueAfterClose() throws Exception {
      Connection conn = createConnection();
      assertFalse(conn.isClosed());

      conn.close();
      assertTrue(conn.isClosed());
    }

    @Test
    void isIdempotent() throws Exception {
      Connection conn = createConnection();
      conn.close();
      conn.close();
      conn.close();

      verify(mockCoreApi, times(1)).connectionClose(any());
      verify(mockCoreApi, times(1)).connectionRelease(any());
      verify(mockCoreApi, times(1)).databaseRelease(any());
    }

    @Test
    void releasesHandlesEvenWhenConnectionCloseThrows() throws Exception {
      when(mockCoreApi.connectionClose(any())).thenThrow(new SQLException("server error"));

      Connection conn = createConnection();
      assertThrows(SQLException.class, conn::close);

      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
      assertTrue(conn.isClosed());
    }

    @Test
    void operationsThrowAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();

      assertThrows(SQLException.class, conn::createStatement);
      assertThrows(SQLException.class, () -> conn.prepareStatement("SELECT 1"));
    }

    @Test
    void concurrentCallsResultInSingleLogout() throws Exception {
      Connection conn = createConnection();
      int threadCount = 5;
      CyclicBarrier barrier = new CyclicBarrier(threadCount);
      AtomicInteger exceptions = new AtomicInteger(0);

      Thread[] threads = new Thread[threadCount];
      for (int i = 0; i < threadCount; i++) {
        threads[i] =
            new Thread(
                () -> {
                  try {
                    barrier.await();
                    conn.close();
                  } catch (Exception e) {
                    exceptions.incrementAndGet();
                  }
                });
        threads[i].start();
      }

      for (Thread t : threads) {
        t.join();
      }

      verify(mockCoreApi, times(1)).connectionClose(any());
      assertTrue(conn.isClosed());
      assertEquals(0, exceptions.get(), "No thread should have thrown an exception");
    }

    @Test
    void doesNotCallRpcsWhenAlreadyClosed() throws Exception {
      Connection conn = createConnection();
      conn.close();

      clearInvocations(mockCoreApi);
      conn.close();

      verify(mockCoreApi, never()).connectionClose(any());
      verify(mockCoreApi, never()).connectionRelease(any());
      verify(mockCoreApi, never()).databaseRelease(any());
    }

    @Test
    void closesOpenStatementsBeforeConnectionClose() throws Exception {
      StatementHandle stmtHandle = StatementHandle.newBuilder().setId(10).setMagic(1000).build();
      when(mockCoreApi.statementNew(any()))
          .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());
      when(mockCoreApi.statementRelease(any()))
          .thenReturn(StatementReleaseResponse.getDefaultInstance());

      Connection conn = createConnection();
      Statement stmt = conn.createStatement();
      assertFalse(stmt.isClosed());

      conn.close();

      assertTrue(stmt.isClosed());
      verify(mockCoreApi).statementRelease(any());
    }

    @Test
    void shouldCloseConnectionWhenAborted() throws Exception {
      Connection conn = createConnection();
      assertFalse(conn.isClosed());

      conn.abort(null);

      assertTrue(conn.isClosed());
      verify(mockCoreApi).connectionClose(any());
      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
    }

    @Test
    void manuallyClosedStatementIsNotDoubleClosedOnConnectionClose() throws Exception {
      StatementHandle stmtHandle = StatementHandle.newBuilder().setId(10).setMagic(1000).build();
      when(mockCoreApi.statementNew(any()))
          .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());
      when(mockCoreApi.statementRelease(any()))
          .thenReturn(StatementReleaseResponse.getDefaultInstance());

      Connection conn = createConnection();
      Statement stmt = conn.createStatement();
      stmt.close();

      clearInvocations(mockCoreApi);
      conn.close();

      verify(mockCoreApi, never()).statementRelease(any());
    }
  }

  @Nested
  class IsValid {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void returnsTrueWhenHeartbeatSucceeds() throws Exception {
      when(mockCoreApi.connectionHeartbeat(any(), anyInt()))
          .thenReturn(ConnectionHeartbeatResponse.newBuilder().setValid(true).build());

      try (Connection conn = createConnection()) {
        assertTrue(conn.isValid(0));
      }
    }

    @Test
    void returnsFalseWhenHeartbeatReportsInvalid() throws Exception {
      when(mockCoreApi.connectionHeartbeat(any(), anyInt()))
          .thenReturn(ConnectionHeartbeatResponse.newBuilder().setValid(false).build());

      try (Connection conn = createConnection()) {
        assertFalse(conn.isValid(0));
      }
    }

    @Test
    void returnsFalseWhenHeartbeatThrows() throws Exception {
      when(mockCoreApi.connectionHeartbeat(any(), anyInt()))
          .thenThrow(new SQLException("session expired"));

      try (Connection conn = createConnection()) {
        assertFalse(conn.isValid(0));
      }
    }

    @Test
    void returnsFalseAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertFalse(conn.isValid(0));
      verify(mockCoreApi, never()).connectionHeartbeat(any(), anyInt());
    }

    @Test
    void throwsOnNegativeTimeout() throws Exception {
      try (Connection conn = createConnection()) {
        assertThrows(SQLException.class, () -> conn.isValid(-1));
      }
    }

    @Test
    void passesTimeoutToCore() throws Exception {
      when(mockCoreApi.connectionHeartbeat(any(), anyInt()))
          .thenReturn(ConnectionHeartbeatResponse.newBuilder().setValid(true).build());

      try (Connection conn = createConnection()) {
        assertTrue(conn.isValid(5));
        verify(mockCoreApi).connectionHeartbeat(any(), org.mockito.ArgumentMatchers.eq(5));
      }
    }
  }

  @Nested
  class Catalog {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void shouldReturnSessionDatabaseFromCoreOnConnect() throws Exception {
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(ConnectionGetInfoResponse.newBuilder().setDatabase("TEST_DB").build());

      try (Connection conn = createConnection()) {
        assertEquals("TEST_DB", conn.getCatalog());
      }
    }

    @Test
    void shouldReturnNullWhenSessionHasNoDatabase() throws Exception {
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(ConnectionGetInfoResponse.getDefaultInstance());

      try (Connection conn = createConnection()) {
        assertNull(conn.getCatalog());
      }
    }

    @Test
    void shouldUseDatabaseViaCoreOnSetCatalog() throws Exception {
      when(mockCoreApi.connectionUseDatabase(any(), org.mockito.ArgumentMatchers.eq("SECOND_DB")))
          .thenReturn(ConnectionUseDatabaseResponse.getDefaultInstance());
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(
              // First read happens at connect time (login-parity warning check).
              ConnectionGetInfoResponse.newBuilder().setDatabase("TEST_DB").build(),
              ConnectionGetInfoResponse.newBuilder().setDatabase("TEST_DB").build(),
              ConnectionGetInfoResponse.newBuilder().setDatabase("SECOND_DB").build());

      try (Connection conn = createConnection()) {
        assertEquals("TEST_DB", conn.getCatalog());
        conn.setCatalog("SECOND_DB");
        assertEquals("SECOND_DB", conn.getCatalog());
      }

      verify(mockCoreApi)
          .connectionUseDatabase(any(), org.mockito.ArgumentMatchers.eq("SECOND_DB"));
    }

    @Test
    void shouldThrowWhenSetCatalogFails() throws Exception {
      when(mockCoreApi.connectionUseDatabase(any(), any()))
          .thenThrow(new SQLException("Object does not exist", "42000", 2003));

      try (Connection conn = createConnection()) {
        SQLException ex = assertThrows(SQLException.class, () -> conn.setCatalog("MISSING_DB"));
        assertEquals("42000", ex.getSQLState());
        assertEquals(2003, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      Connection conn = createConnection();
      conn.close();

      assertThrows(SQLException.class, conn::getCatalog);
      assertThrows(SQLException.class, () -> conn.setCatalog("OTHER_DB"));
    }
  }

  @Nested
  class AutoCommit {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void shouldDefaultToTrue() throws Exception {
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldInitializeFromServerParameterFalse() throws Exception {
      when(mockCoreApi.connectionGetParameter(any(), eq("AUTOCOMMIT")))
          .thenReturn(ConnectionGetParameterResponse.newBuilder().setValue("false").build());
      try (Connection conn = createConnection()) {
        assertFalse(conn.getAutoCommit());
      }
    }

    @Test
    void shouldInitializeFromServerParameterTrue() throws Exception {
      when(mockCoreApi.connectionGetParameter(any(), eq("AUTOCOMMIT")))
          .thenReturn(ConnectionGetParameterResponse.newBuilder().setValue("true").build());
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldFallBackToTrueWhenServerParameterLookupFails() throws Exception {
      when(mockCoreApi.connectionGetParameter(any(), eq("AUTOCOMMIT")))
          .thenThrow(new SQLException("parameter lookup failed"));
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldThrowOnGetAutoCommitAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(SQLException.class, conn::getAutoCommit);
    }

    @Test
    void shouldInvokeSetAutocommitRpcWhenSetAutoCommitFalse() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        assertFalse(conn.getAutoCommit());
        verify(mockCoreApi).connectionSetAutocommit(any(), eq(false));
      }
    }

    @Test
    void shouldInvokeSetAutocommitRpcWhenSetAutoCommitTrueAfterFalse() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        clearInvocations(mockCoreApi);
        conn.setAutoCommit(true);
        assertTrue(conn.getAutoCommit());
        verify(mockCoreApi).connectionSetAutocommit(any(), eq(true));
      }
    }

    @Test
    void shouldNotInvokeSetAutocommitRpcWhenSetAutoCommitToCurrentValue() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(true);
        verify(mockCoreApi, never()).connectionSetAutocommit(any(), anyBoolean());
      }
    }

    @Test
    void shouldThrowOnSetAutoCommitAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(SQLException.class, () -> conn.setAutoCommit(false));
    }

    @Test
    void shouldUpdateCacheBeforeRpcEvenWhenRpcFails() throws Exception {
      try (Connection conn = createConnection()) {
        when(mockCoreApi.connectionSetAutocommit(any(), anyBoolean()))
            .thenThrow(new SQLException("simulated set-autocommit failure"));
        assertThrows(SQLException.class, () -> conn.setAutoCommit(false));
        // Cache reflects the new value despite the failed RPC; matches snowflake-jdbc parity.
        assertFalse(conn.getAutoCommit());
      }
    }
  }

  @Nested
  class TransactionIsolation {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void shouldDefaultToReadCommitted() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldStillReportReadCommittedAfterSettingReadCommitted() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED);
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldAcceptNoneAsNoOpButStillReportReadCommitted() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setTransactionIsolation(Connection.TRANSACTION_NONE);
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldRejectUnsupportedLevel() throws Exception {
      try (Connection conn = createConnection()) {
        assertThrows(
            SQLFeatureNotSupportedException.class,
            () -> conn.setTransactionIsolation(Connection.TRANSACTION_SERIALIZABLE));
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldThrowOnGetAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(SQLException.class, conn::getTransactionIsolation);
    }

    @Test
    void shouldThrowOnSetAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(
          SQLException.class,
          () -> conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED));
    }
  }

  @Nested
  class Schema {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void shouldReturnSessionSchemaFromCore() throws Exception {
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(ConnectionGetInfoResponse.newBuilder().setSchema("TEST_SCHEMA").build());

      try (Connection conn = createConnection()) {
        assertEquals("TEST_SCHEMA", conn.getSchema());
      }
    }

    @Test
    void shouldReturnNullWhenSessionHasNoSchema() throws Exception {
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(ConnectionGetInfoResponse.getDefaultInstance());

      try (Connection conn = createConnection()) {
        assertNull(conn.getSchema());
      }
    }

    @Test
    void shouldUseSchemaViaCoreOnSetSchema() throws Exception {
      when(mockCoreApi.connectionUseSchema(any(), org.mockito.ArgumentMatchers.eq("SECOND_SCHEMA")))
          .thenReturn(ConnectionUseSchemaResponse.getDefaultInstance());
      when(mockCoreApi.connectionGetInfo(any()))
          .thenReturn(
              // First read happens at connect time (login-parity warning check).
              ConnectionGetInfoResponse.newBuilder().setSchema("TEST_SCHEMA").build(),
              ConnectionGetInfoResponse.newBuilder().setSchema("TEST_SCHEMA").build(),
              ConnectionGetInfoResponse.newBuilder().setSchema("SECOND_SCHEMA").build());

      try (Connection conn = createConnection()) {
        assertEquals("TEST_SCHEMA", conn.getSchema());
        conn.setSchema("SECOND_SCHEMA");
        assertEquals("SECOND_SCHEMA", conn.getSchema());
      }

      verify(mockCoreApi)
          .connectionUseSchema(any(), org.mockito.ArgumentMatchers.eq("SECOND_SCHEMA"));
    }

    @Test
    void shouldThrowWhenSetSchemaFails() throws Exception {
      when(mockCoreApi.connectionUseSchema(any(), any()))
          .thenThrow(new SQLException("Object does not exist", "42000", 2003));

      try (Connection conn = createConnection()) {
        SQLException ex = assertThrows(SQLException.class, () -> conn.setSchema("MISSING_SCHEMA"));
        assertEquals("42000", ex.getSQLState());
        assertEquals(2003, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      Connection conn = createConnection();
      conn.close();

      assertThrows(SQLException.class, conn::getSchema);
      assertThrows(SQLException.class, () -> conn.setSchema("OTHER_SCHEMA"));
    }
  }

  @Nested
  class Transactions {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection createConnection() throws SQLException {
      return openConnection(mockCoreApi);
    }

    @Test
    void shouldInvokeCommitRpcOnCommit() throws Exception {
      try (Connection conn = createConnection()) {
        conn.commit();
        verify(mockCoreApi).connectionCommit(any());
      }
    }

    @Test
    void shouldInvokeRollbackRpcOnRollback() throws Exception {
      try (Connection conn = createConnection()) {
        conn.rollback();
        verify(mockCoreApi).connectionRollback(any());
      }
    }

    @Test
    void shouldThrowOnCommitAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(SQLException.class, conn::commit);
    }

    @Test
    void shouldThrowOnRollbackAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();
      assertThrows(SQLException.class, conn::rollback);
    }
  }

  @Nested
  class CreateAndPrepareStatement {

    private CoreDriverApi mockCoreApi;
    private StatementHandle stmtHandle;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
      stmtHandle = StatementHandle.newBuilder().setId(10).setMagic(1000).build();
      when(mockCoreApi.statementNew(any()))
          .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());
      when(mockCoreApi.statementRelease(any()))
          .thenReturn(StatementReleaseResponse.getDefaultInstance());
    }

    // createStatement() overloads

    @Test
    void shouldCreateStatementWithDefaultArgs() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          Statement stmt = conn.createStatement()) {
        assertInstanceOf(Statement.class, stmt);
      }
    }

    @Test
    void shouldCreateStatementWhenTypeAndConcurrencyAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          Statement stmt =
              conn.createStatement(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY)) {
        assertInstanceOf(Statement.class, stmt);
      }
    }

    @Test
    void shouldCreateStatementWhenAllThreeHoldabilityArgsAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          Statement stmt =
              conn.createStatement(
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
        assertInstanceOf(Statement.class, stmt);
      }
    }

    @Test
    void shouldThrowOnUnsupportedResultSetType() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(
                        ResultSet.TYPE_SCROLL_INSENSITIVE, ResultSet.CONCUR_READ_ONLY));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnUnsupportedResultSetConcurrency() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnUnsupportedHoldability() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    // prepareStatement() overloads

    @Test
    void shouldPrepareStatementWithSqlOnly() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          PreparedStatement stmt = conn.prepareStatement("SELECT 1")) {
        assertInstanceOf(PreparedStatement.class, stmt);
      }
    }

    @Test
    void shouldPrepareStatementWhenTypeAndConcurrencyAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          PreparedStatement stmt =
              conn.prepareStatement(
                  "SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY)) {
        assertInstanceOf(PreparedStatement.class, stmt);
      }
    }

    @Test
    void shouldPrepareStatementWhenAllThreeHoldabilityArgsAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          PreparedStatement stmt =
              conn.prepareStatement(
                  "SELECT 1",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
        assertInstanceOf(PreparedStatement.class, stmt);
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithUnsupportedResultSetType() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "SELECT 1", ResultSet.TYPE_SCROLL_INSENSITIVE, ResultSet.CONCUR_READ_ONLY));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithUnsupportedResultSetConcurrency() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithUnsupportedHoldability() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "SELECT 1",
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldPrepareStatementWhenAutoGeneratedKeysIsNoGeneratedKeys() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          PreparedStatement stmt =
              conn.prepareStatement("INSERT INTO t VALUES (1)", Statement.NO_GENERATED_KEYS)) {
        assertInstanceOf(PreparedStatement.class, stmt);
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithReturnGeneratedKeys() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        assertThrows(
            SQLFeatureNotSupportedException.class,
            () ->
                conn.prepareStatement("INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithColumnIndexes() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        assertThrows(
            SQLFeatureNotSupportedException.class,
            () -> conn.prepareStatement("INSERT INTO t VALUES (1)", new int[] {1}));
      }
    }

    @Test
    void shouldThrowOnPrepareStatementWithColumnNames() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        assertThrows(
            SQLFeatureNotSupportedException.class,
            () -> conn.prepareStatement("INSERT INTO t VALUES (1)", new String[] {"id"}));
      }
    }

    // prepareCall() overloads

    @Test
    void shouldPrepareCallWithSqlOnly() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          CallableStatement stmt = conn.prepareCall("{call my_proc()}")) {
        assertInstanceOf(CallableStatement.class, stmt);
      }
    }

    @Test
    void shouldPrepareCallWhenTypeAndConcurrencyAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          CallableStatement stmt =
              conn.prepareCall(
                  "{call my_proc()}", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY)) {
        assertInstanceOf(CallableStatement.class, stmt);
      }
    }

    @Test
    void shouldPrepareCallWhenAllThreeHoldabilityArgsAreSupported() throws Exception {
      try (Connection conn = openConnection(mockCoreApi);
          CallableStatement stmt =
              conn.prepareCall(
                  "{call my_proc()}",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
        assertInstanceOf(CallableStatement.class, stmt);
      }
    }

    @Test
    void shouldThrowOnPrepareCallWithUnsupportedResultSetType() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "{call my_proc()}",
                        ResultSet.TYPE_SCROLL_INSENSITIVE,
                        ResultSet.CONCUR_READ_ONLY));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnPrepareCallWithUnsupportedResultSetConcurrency() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "{call my_proc()}",
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_UPDATABLE));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }

    @Test
    void shouldThrowOnPrepareCallWithUnsupportedHoldability() throws Exception {
      try (Connection conn = openConnection(mockCoreApi)) {
        SQLFeatureNotSupportedException ex =
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "{call my_proc()}",
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT));
        assertEquals("0A000", ex.getSQLState());
        assertEquals(200035, ex.getErrorCode());
      }
    }
  }

  @Nested
  class Warnings {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = stubConnectionMock();
    }

    private Connection openConnectionRequesting(
        Properties requested, ConnectionGetInfoResponse info) throws SQLException {
      when(mockCoreApi.connectionGetInfo(any())).thenReturn(info);
      Properties props = new Properties();
      props.setProperty("account", "test_account");
      props.setProperty("user", "test_user");
      props.setProperty("password", MOCK_PASSWORD);
      props.putAll(requested);
      return new SnowflakeConnectionImpl(
          "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);
    }

    @Test
    void shouldExposeLoginMismatchWarningComputedAtConnect() throws Exception {
      try (Connection conn =
          openConnectionRequesting(
              props("database", "REQ_DB"),
              ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build())) {
        SQLWarning warning = conn.getWarnings();
        assertNotNull(warning);
        assertEquals(
            ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP.getMessageCode(),
            warning.getErrorCode());
      }
    }

    @Test
    void shouldReturnNullGetWarningsWhenNoPropertiesRequested() throws Exception {
      try (Connection conn =
          openConnectionRequesting(
              props(), ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build())) {
        assertNull(conn.getWarnings());
      }
    }

    @Test
    void shouldReturnNullAfterClearWarnings() throws Exception {
      try (Connection conn =
          openConnectionRequesting(
              props("database", "REQ_DB"),
              ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build())) {
        assertNotNull(conn.getWarnings());
        conn.clearWarnings();
        assertNull(conn.getWarnings());
      }
    }

    @Test
    void shouldThrowOnGetWarningsAfterClose() throws Exception {
      try (Connection conn =
          openConnectionRequesting(props(), ConnectionGetInfoResponse.getDefaultInstance())) {
        conn.close();
        assertThrows(SQLException.class, conn::getWarnings);
      }
    }

    @Test
    void shouldThrowOnClearWarningsAfterClose() throws Exception {
      try (Connection conn =
          openConnectionRequesting(props(), ConnectionGetInfoResponse.getDefaultInstance())) {
        conn.close();
        assertThrows(SQLException.class, conn::clearWarnings);
      }
    }
  }

  private static CoreDriverApi stubConnectionMock() throws SQLException {
    DatabaseHandle dbHandle = DatabaseHandle.newBuilder().setId(1).setMagic(100).build();
    ConnectionHandle connHandle = ConnectionHandle.newBuilder().setId(2).setMagic(200).build();

    CoreDriverApi mock = mock(CoreDriverApi.class);
    when(mock.databaseNew())
        .thenReturn(DatabaseNewResponse.newBuilder().setDbHandle(dbHandle).build());
    when(mock.databaseInit(any())).thenReturn(DatabaseInitResponse.getDefaultInstance());
    when(mock.connectionNew())
        .thenReturn(ConnectionNewResponse.newBuilder().setConnHandle(connHandle).build());
    when(mock.connectionSetOptions(any(), any()))
        .thenReturn(ConnectionSetOptionsResponse.getDefaultInstance());
    when(mock.connectionSetAutocommit(any(), anyBoolean()))
        .thenReturn(ConnectionSetAutocommitResponse.getDefaultInstance());
    when(mock.connectionInit(any(), any(), any()))
        .thenReturn(ConnectionInitResponse.getDefaultInstance());
    when(mock.connectionGetParameter(any(), eq("AUTOCOMMIT")))
        .thenReturn(ConnectionGetParameterResponse.getDefaultInstance());
    when(mock.connectionClose(any())).thenReturn(ConnectionCloseResponse.getDefaultInstance());
    when(mock.connectionRelease(any())).thenReturn(ConnectionReleaseResponse.getDefaultInstance());
    when(mock.databaseRelease(any())).thenReturn(DatabaseReleaseResponse.getDefaultInstance());
    return mock;
  }

  private static Connection openConnection(CoreDriverApi mockCoreApi) throws SQLException {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", MOCK_PASSWORD);
    return new SnowflakeConnectionImpl(
        "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);
  }
}
