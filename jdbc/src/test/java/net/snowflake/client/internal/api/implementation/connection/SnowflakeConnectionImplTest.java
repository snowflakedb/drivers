package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImplTestFixtures.assertConnectionClosedClientInfoException;
import static net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImplTestFixtures.assertConnectionClosedException;
import static net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImplTestFixtures.assertFeatureNotSupported;
import static net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImplTestFixtures.boundary;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.anyInt;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.clearInvocations;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.ClientInfoStatus;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Savepoint;
import java.sql.Statement;
import java.util.Collections;
import java.util.Map;
import java.util.Properties;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFClientInfoException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHeartbeatResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseDatabaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionUseSchemaResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity;
import net.snowflake.client.internal.util.NotImplementedException;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

class SnowflakeConnectionImplTest {

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
  class Close extends MockCoreApiConnectionSupport {

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
      when(mockCoreApi.connectionClose(any())).thenThrow(driverException("server error"));
      when(mockCoreApi.connectionRelease(any()))
          .thenReturn(ConnectionReleaseResponse.getDefaultInstance());
      when(mockCoreApi.databaseRelease(any()))
          .thenReturn(DatabaseReleaseResponse.getDefaultInstance());

      Connection conn = createConnection();
      assertThrows(CoreException.class, conn::close);

      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
      assertTrue(conn.isClosed());
    }

    @Test
    void operationsThrowAfterClose() throws Exception {
      Connection conn = createConnection();
      conn.close();

      assertConnectionClosedException(assertThrows(SFSQLException.class, conn::createStatement));
      assertConnectionClosedException(
          assertThrows(SFSQLException.class, () -> conn.prepareStatement("SELECT 1")));
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
  class IsValid extends MockCoreApiConnectionSupport {

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
          .thenThrow(driverException("session expired"));

      try (Connection conn = createConnection()) {
        assertFalse(conn.isValid(0));
      }
    }

    @Test
    void returnsFalseAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertFalse(conn.isValid(0));
      }
      verify(mockCoreApi, never()).connectionHeartbeat(any(), anyInt());
    }

    @Test
    void throwsOnNegativeTimeout() throws Exception {
      try (Connection conn = createConnection()) {
        assertThrows(SFSQLException.class, () -> conn.isValid(-1));
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
  class AutoCommit {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
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
          .thenThrow(driverException("parameter lookup failed"));
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldThrowOnGetAutoCommitAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getAutoCommit));
      }
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
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setAutoCommit(false)));
      }
    }

    @Test
    void shouldUpdateCacheBeforeRpcEvenWhenRpcFails() throws Exception {
      try (Connection conn = createConnection()) {
        when(mockCoreApi.connectionSetAutocommit(any(), anyBoolean()))
            .thenThrow(driverException("simulated set-autocommit failure"));
        assertThrows(CoreException.class, () -> conn.setAutoCommit(false));
        assertFalse(conn.getAutoCommit());
      }
    }

    @Test
    void shouldUpdateCacheBeforeRpcWhenSetAutoCommitTrueFails() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        when(mockCoreApi.connectionSetAutocommit(any(), eq(true)))
            .thenThrow(driverException("simulated set-autocommit failure"));
        assertThrows(CoreException.class, () -> conn.setAutoCommit(true));
        assertTrue(conn.getAutoCommit());
      }
    }
  }

  @Nested
  class Transactions {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
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
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::commit));
      }
    }

    @Test
    void shouldThrowOnRollbackAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::rollback));
      }
    }

    @Test
    void shouldInvokeCommitRpcAfterSetAutoCommitFalse() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        clearInvocations(mockCoreApi);
        conn.commit();
        verify(mockCoreApi).connectionCommit(any());
      }
    }

    @Test
    void shouldInvokeRollbackRpcAfterSetAutoCommitFalse() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        clearInvocations(mockCoreApi);
        conn.rollback();
        verify(mockCoreApi).connectionRollback(any());
      }
    }

    @Test
    void shouldPropagateCommitRpcFailure() throws Exception {
      when(mockCoreApi.connectionCommit(any()))
          .thenThrow(driverException("commit failed", "40001", 100));
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        CoreException ex = assertThrows(CoreException.class, conn::commit);
        assertEquals("40001", ex.getError().getSqlState());
        assertEquals(100, ex.getError().getVendorCode());
      }
    }

    @Test
    void shouldPropagateRollbackRpcFailure() throws Exception {
      when(mockCoreApi.connectionRollback(any()))
          .thenThrow(driverException("rollback failed", "40001", 101));
      try (Connection conn = createConnection()) {
        conn.setAutoCommit(false);
        CoreException ex = assertThrows(CoreException.class, conn::rollback);
        assertEquals("40001", ex.getError().getSqlState());
        assertEquals(101, ex.getError().getVendorCode());
      }
    }

    @Test
    void shouldPropagateCommitRpcFailureUnderAutoCommit() throws Exception {
      when(mockCoreApi.connectionCommit(any()))
          .thenThrow(driverException("commit failed", "40001", 100));
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
        CoreException ex = assertThrows(CoreException.class, conn::commit);
        assertEquals("40001", ex.getError().getSqlState());
        assertEquals(100, ex.getError().getVendorCode());
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldPropagateRollbackRpcFailureUnderAutoCommit() throws Exception {
      when(mockCoreApi.connectionRollback(any()))
          .thenThrow(driverException("rollback failed", "40001", 101));
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
        CoreException ex = assertThrows(CoreException.class, conn::rollback);
        assertEquals("40001", ex.getError().getSqlState());
        assertEquals(101, ex.getError().getVendorCode());
        assertTrue(conn.getAutoCommit());
      }
    }

    @Test
    void shouldAllowCommitAndRollbackUnderAutoCommit() throws Exception {
      try (Connection conn = createConnection()) {
        assertTrue(conn.getAutoCommit());
        conn.commit();
        conn.rollback();
        verify(mockCoreApi).connectionCommit(any());
        verify(mockCoreApi).connectionRollback(any());
      }
    }
  }

  @Nested
  class Catalog {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
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
              ConnectionGetInfoResponse.getDefaultInstance(),
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
          .thenThrow(driverException("Object does not exist", "42000", 2003));

      try (Connection conn = createConnection()) {
        CoreException ex = assertThrows(CoreException.class, () -> conn.setCatalog("MISSING_DB"));
        assertEquals("42000", ex.getError().getSqlState());
        assertEquals(2003, ex.getError().getVendorCode());
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        SFSQLException ex = assertThrows(SFSQLException.class, conn::getCatalog);
        assertConnectionClosedException(ex);
        ex = assertThrows(SFSQLException.class, () -> conn.setCatalog("OTHER_DB"));
        assertConnectionClosedException(ex);
      }
    }
  }

  @Nested
  class Schema {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
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
              ConnectionGetInfoResponse.getDefaultInstance(),
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
          .thenThrow(driverException("Object does not exist", "42000", 2003));

      try (Connection conn = createConnection()) {
        CoreException ex =
            assertThrows(CoreException.class, () -> conn.setSchema("MISSING_SCHEMA"));
        assertEquals("42000", ex.getError().getSqlState());
        assertEquals(2003, ex.getError().getVendorCode());
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getSchema));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setSchema("OTHER_SCHEMA")));
      }
    }
  }

  @Nested
  class ClientInfo {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldReturnEmptyClientInfoProperties() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(0, conn.getClientInfo().size());
      }
    }

    @Test
    void shouldReturnNullForUnknownClientInfoKey() throws Exception {
      try (Connection conn = createConnection()) {
        assertNull(conn.getClientInfo("ApplicationName"));
      }
    }

    @Test
    void shouldRejectSetClientInfoWithUnknownProperty() throws Exception {
      Properties clientInfo = new Properties();
      clientInfo.setProperty("name", "Peter");
      clientInfo.setProperty("description", "SNOWFLAKE JDBC");

      try (Connection conn = boundary(createConnection())) {
        SQLClientInfoException ex =
            assertThrows(SQLClientInfoException.class, () -> conn.setClientInfo(clientInfo));
        assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), ex.getSQLState());
        assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), ex.getErrorCode());
        assertEquals(2, ex.getFailedProperties().size());
        assertEquals(
            ClientInfoStatus.REASON_UNKNOWN_PROPERTY, ex.getFailedProperties().get("name"));
        assertEquals(
            ClientInfoStatus.REASON_UNKNOWN_PROPERTY, ex.getFailedProperties().get("description"));
      }
    }

    @Test
    void shouldRejectSetClientInfoSingleUnknownProperty() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        SQLClientInfoException ex =
            assertThrows(
                SQLClientInfoException.class,
                () -> conn.setClientInfo("ApplicationName", "valueA"));
        assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), ex.getSQLState());
        assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), ex.getErrorCode());
        assertEquals(1, ex.getFailedProperties().size());
        assertEquals(
            ClientInfoStatus.REASON_UNKNOWN_PROPERTY,
            ex.getFailedProperties().get("ApplicationName"));
      }
    }

    @Test
    void shouldAllowNullPropertiesSetClientInfoOnOpenConnection() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setClientInfo(null);
      }
    }

    @Test
    void shouldAllowEmptyPropertiesSetClientInfoOnOpenConnection() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setClientInfo(new Properties());
      }
    }

    @Test
    void shouldThrowClientInfoExceptionWhenClosedWithSingleProperty() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        conn.close();
        SQLClientInfoException ex =
            assertThrows(
                SQLClientInfoException.class, () -> conn.setClientInfo("ApplicationName", "x"));
        assertConnectionClosedClientInfoException(ex);
        assertEquals(
            ClientInfoStatus.REASON_UNKNOWN_PROPERTY,
            ex.getFailedProperties().get("ApplicationName"));
      }
    }

    @Test
    void shouldThrowClientInfoExceptionWhenClosedWithMultipleProperties() throws Exception {
      Properties clientInfo = new Properties();
      clientInfo.setProperty("a", "1");
      clientInfo.setProperty("b", "2");

      try (Connection conn = boundary(createConnection())) {
        conn.close();
        SQLClientInfoException ex =
            assertThrows(SQLClientInfoException.class, () -> conn.setClientInfo(clientInfo));
        assertConnectionClosedClientInfoException(ex);
        assertEquals(2, ex.getFailedProperties().size());
      }
    }

    @Test
    void shouldThrowClientInfoExceptionWhenClosedWithNullProperties() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        conn.close();
        SQLClientInfoException ex =
            assertThrows(SQLClientInfoException.class, () -> conn.setClientInfo((Properties) null));
        assertConnectionClosedClientInfoException(ex);
        assertTrue(ex.getFailedProperties().isEmpty());
      }
    }

    @Test
    void shouldThrowWhenGetClientInfoAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        SFSQLException ex = assertThrows(SFSQLException.class, conn::getClientInfo);
        assertConnectionClosedException(ex);
      }
    }

    @Test
    void shouldThrowWhenGetClientInfoKeyAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        SFSQLException ex = assertThrows(SFSQLException.class, () -> conn.getClientInfo("key"));
        assertConnectionClosedException(ex);
      }
    }
  }

  @Nested
  class ClosedConnectionGuard {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldRejectOperationsAfterCloseWithConnectionClosedCode() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();

        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getMetaData));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getAutoCommit));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::commit));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::rollback));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::isReadOnly));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getCatalog));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getSchema));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, conn::getTransactionIsolation));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getWarnings));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::clearWarnings));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.nativeSQL("select 1")));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setAutoCommit(false)));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setReadOnly(false)));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setCatalog("db")));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setSchema("sch")));
        assertConnectionClosedException(
            assertThrows(
                SFSQLException.class,
                () -> conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED)));
        // ClosedConnectionGuard is a white-box check that every guarded method surfaces the runtime
        // closed carrier on the raw impl; setClientInfo's carrier is SFClientInfoException, which
        // the
        // boundary would translate to the SQLClientInfoException asserted here.
        assertConnectionClosedClientInfoException(
            (SQLClientInfoException)
                assertThrows(
                        SFClientInfoException.class, () -> conn.setClientInfo(new Properties()))
                    .toSQLException());
        assertConnectionClosedClientInfoException(
            (SQLClientInfoException)
                assertThrows(SFClientInfoException.class, () -> conn.setClientInfo("name", "value"))
                    .toSQLException());
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.prepareCall("call foo()")));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.createArrayOf("INT", new Object[] {1})));
      }
    }
  }

  @Nested
  class TransactionIsolation {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldDefaultToTransactionNone() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldAcceptTransactionNone() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setTransactionIsolation(Connection.TRANSACTION_NONE);
        assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldAcceptReadCommitted() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED);
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      }
    }

    @Test
    void shouldRejectSerializable() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setTransactionIsolation(Connection.TRANSACTION_SERIALIZABLE)));
      }
    }

    @Test
    void shouldRejectRepeatableRead() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setTransactionIsolation(Connection.TRANSACTION_REPEATABLE_READ)));
      }
    }

    @Test
    void shouldRejectReadUncommitted() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setTransactionIsolation(Connection.TRANSACTION_READ_UNCOMMITTED)));
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, conn::getTransactionIsolation));
        assertConnectionClosedException(
            assertThrows(
                SFSQLException.class,
                () -> conn.setTransactionIsolation(Connection.TRANSACTION_NONE)));
      }
    }
  }

  @Nested
  class NativeSqlAndTypeMap {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldReturnNativeSqlUnchanged() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals("select 1", conn.nativeSQL("select 1"));
      }
    }

    @Test
    void shouldReturnEmptyTypeMap() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(Collections.emptyMap(), conn.getTypeMap());
      }
    }

    @Test
    void shouldRejectSetTypeMap() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setTypeMap(Collections.emptyMap())));
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.nativeSQL("select 1")));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getTypeMap));
      }
    }
  }

  @Nested
  class ReadOnlyAndWarnings {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldReportNotReadOnly() throws Exception {
      try (Connection conn = createConnection()) {
        assertFalse(conn.isReadOnly());
      }
    }

    @Test
    void shouldAllowSetReadOnlyAsNoOp() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setReadOnly(true);
        assertFalse(conn.isReadOnly());
      }
    }

    @Test
    void shouldReturnNullWarnings() throws Exception {
      try (Connection conn = createConnection()) {
        assertNull(conn.getWarnings());
      }
    }

    @Test
    void shouldAllowClearWarningsAsNoOp() throws Exception {
      try (Connection conn = createConnection()) {
        conn.clearWarnings();
      }
    }

    @Test
    void shouldThrowWhenClosed() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::isReadOnly));
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setReadOnly(true)));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getWarnings));
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::clearWarnings));
      }
    }
  }

  @Nested
  class Abort {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldCloseConnectionOnAbort() throws Exception {
      try (Connection conn = createConnection()) {
        assertFalse(conn.isClosed());
        conn.abort(null);
        assertTrue(conn.isClosed());
        verify(mockCoreApi).connectionClose(any());
      }
    }
  }

  @Nested
  class FeatureNotSupported {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldRejectSetSavepoint() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, conn::setSavepoint));
      }
    }

    @Test
    void shouldRejectSetSavepointWithName() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, () -> conn.setSavepoint("sp")));
      }
    }

    @Test
    void shouldRejectRollbackToSavepoint() throws Exception {
      Savepoint savepoint = new FakeSavepoint();
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, () -> conn.rollback(savepoint)));
      }
    }

    @Test
    void shouldRejectReleaseSavepoint() throws Exception {
      Savepoint savepoint = new FakeSavepoint();
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class, () -> conn.releaseSavepoint(savepoint)));
      }
    }

    @Test
    void shouldRejectPrepareStatementWithColumnIndexes() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.prepareStatement("select 1", new int[] {1})));
      }
    }

    @Test
    void shouldRejectPrepareStatementWithColumnNames() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.prepareStatement("select 1", new String[] {"c1"})));
      }
    }

    @Test
    void shouldRejectPrepareStatementWithGeneratedKeys() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.prepareStatement("select 1", Statement.RETURN_GENERATED_KEYS)));
      }
    }

    @Test
    void shouldRejectCreateBlob() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, conn::createBlob));
      }
    }

    @Test
    void shouldRejectCreateNClob() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, conn::createNClob));
      }
    }

    @Test
    void shouldRejectCreateSQLXML() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(SQLFeatureNotSupportedException.class, conn::createSQLXML));
      }
    }

    @Test
    void shouldRejectCreateStruct() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.createStruct("fakeType", new Object[] {})));
      }
    }

    @Test
    void shouldRejectCreateArrayOfOnOpenConnection() throws Exception {
      try (Connection conn = createConnection()) {
        assertThrows(
            NotImplementedException.class, () -> conn.createArrayOf("INT", new Object[] {1}));
      }
    }
  }

  @Nested
  class StatementFactory {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldRejectScrollSensitiveCreateStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(
                        ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY)));
      }
    }

    @Test
    void shouldRejectUpdatableConcurrencyCreateStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE)));
      }
    }

    @Test
    void shouldRejectScrollSensitivePrepareStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "select 1", ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY)));
      }
    }

    @Test
    void shouldRejectScrollSensitivePrepareCall() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "call foo()",
                        ResultSet.TYPE_SCROLL_SENSITIVE,
                        ResultSet.CONCUR_READ_ONLY)));
      }
    }

    @Test
    void shouldRejectScrollInsensitiveCreateStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(
                        ResultSet.TYPE_SCROLL_INSENSITIVE, ResultSet.CONCUR_READ_ONLY)));
      }
    }

    @Test
    void shouldRejectUpdatableConcurrencyPrepareStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "select 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE)));
      }
    }

    @Test
    void shouldRejectUpdatableConcurrencyPrepareCall() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "call foo()", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE)));
      }
    }
  }

  @Nested
  class DatabaseMetadata {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldReturnSnowflakeProductNameFromOpenConnection() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals("Snowflake", conn.getMetaData().getDatabaseProductName());
      }
    }
  }

  @Nested
  class CreateClob {

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }

    @Test
    void shouldCreateClobOnOpenConnection() throws Exception {
      try (Connection conn = createConnection()) {
        Clob clob = conn.createClob();
        assertEquals(0, clob.length());
        clob.free();
      }
    }

    @Test
    void shouldThrowWhenCreateClobAfterClose() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::createClob));
      }
    }
  }

  @Nested
  class SetNetworkTimeout extends MockCoreApiConnectionSupport {

    @Test
    void shouldDefaultNetworkTimeoutToZero() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(0, conn.getNetworkTimeout());
      }
    }

    @Test
    void shouldAcceptSetNetworkTimeoutAsNoOpUntilCoreSupport() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setNetworkTimeout(null, 2000);
        assertEquals(0, conn.getNetworkTimeout());
      }
    }

    @Test
    void shouldThrowAfterCloseWhenSettingNetworkTimeout() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, () -> conn.setNetworkTimeout(null, 1000)));
      }
    }

    @Test
    void shouldThrowAfterCloseWhenGettingNetworkTimeout() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(SFSQLException.class, conn::getNetworkTimeout));
      }
    }
  }

  @Nested
  class SetHoldability extends MockCoreApiConnectionSupport {

    @Test
    void shouldAcceptSupportedHoldabilityAsNoOp() throws Exception {
      try (Connection conn = createConnection()) {
        conn.setHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT);
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, conn.getHoldability());
      }
    }

    @Test
    void shouldDefaultHoldabilityToCloseCursorsAtCommit() throws Exception {
      try (Connection conn = createConnection()) {
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, conn.getHoldability());
      }
    }

    @Test
    void shouldPreserveDefaultHoldabilityAfterRejectedSet() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT)));
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, conn.getHoldability());
      }
    }

    @Test
    void shouldRejectInvalidHoldabilityConstant() throws Exception {
      try (Connection conn = createConnection()) {
        assertThrows(SFSQLException.class, () -> conn.setHoldability(999));
      }
    }

    @Test
    void shouldRejectUnsupportedHoldability() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () -> conn.setHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT)));
      }
    }

    @Test
    void shouldCreateStatementWithSupportedHoldability() throws Exception {
      StatementHandle stmtHandle = StatementHandle.newBuilder().setId(10).setMagic(1000).build();
      when(mockCoreApi.statementNew(any()))
          .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());

      try (Connection conn = createConnection()) {
        try (Statement stmt =
            conn.createStatement(
                ResultSet.TYPE_FORWARD_ONLY,
                ResultSet.CONCUR_READ_ONLY,
                ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
          assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, conn.getHoldability());
          assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, stmt.getResultSetHoldability());
        }
      }
    }

    @Test
    void shouldThrowAfterCloseWhenSettingHoldability() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(
            assertThrows(
                SFSQLException.class,
                () -> conn.setHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT)));
      }
    }

    @Test
    void shouldThrowAfterCloseWhenGettingHoldability() throws Exception {
      try (Connection conn = createConnection()) {
        conn.close();
        assertConnectionClosedException(assertThrows(SFSQLException.class, conn::getHoldability));
      }
    }

    @Test
    void shouldRejectUnsupportedHoldabilityWhenCreatingStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.createStatement(
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT)));
      }
    }

    @Test
    void shouldRejectUnsupportedHoldabilityWhenPreparingStatement() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareStatement(
                        "select 1",
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT)));
      }
    }

    @Test
    void shouldRejectUnsupportedHoldabilityWhenPreparingCall() throws Exception {
      try (Connection conn = boundary(createConnection())) {
        assertFeatureNotSupported(
            assertThrows(
                SQLFeatureNotSupportedException.class,
                () ->
                    conn.prepareCall(
                        "call foo()",
                        ResultSet.TYPE_FORWARD_ONLY,
                        ResultSet.CONCUR_READ_ONLY,
                        ResultSet.HOLD_CURSORS_OVER_COMMIT)));
      }
    }
  }

  private static CoreException driverException(String message) {
    return new CoreException(DriverException.newBuilder().setMessage(message).build(), null);
  }

  private static CoreException driverException(String message, String sqlState, int vendorCode) {
    return new CoreException(
        DriverException.newBuilder()
            .setMessage(message)
            .setSqlState(sqlState)
            .setVendorCode(vendorCode)
            .build(),
        null);
  }

  private static final class FakeSavepoint implements Savepoint {
    @Override
    public int getSavepointId() {
      return 1;
    }

    @Override
    public String getSavepointName() {
      return "fake";
    }
  }

  @Nested
  class ConnectionOptions extends MockCoreApiConnectionSupport {

    @Test
    void shouldNormalizeDataSourceNonProxyHostsWhenConnecting() throws Exception {
      Properties props = new Properties();
      props.setProperty("account", "test_account");
      props.setProperty("user", "test_user");
      props.setProperty("password", "dummy");
      props.setProperty("nonProxyHosts", "*.foo.com|host1");

      @SuppressWarnings("unchecked")
      ArgumentCaptor<Map<String, ConfigSetting>> optionsCaptor = ArgumentCaptor.forClass(Map.class);

      new SnowflakeConnectionImpl(
          "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);

      verify(mockCoreApi).connectionSetOptions(any(), optionsCaptor.capture());
      assertEquals(".foo.com,host1", optionsCaptor.getValue().get("no_proxy").getStringValue());
    }
  }

  @Nested
  class WrapperIdentityInit extends MockCoreApiConnectionSupport {

    private WrapperIdentity captureIdentitySentToCore() throws Exception {
      ArgumentCaptor<WrapperIdentity> identityCaptor =
          ArgumentCaptor.forClass(WrapperIdentity.class);
      try (Connection ignored = createConnection()) {
        verify(mockCoreApi).connectionInit(any(), any(), identityCaptor.capture());
      }
      return identityCaptor.getValue();
    }

    @Test
    void shouldSendJdbcDriverNameAndVersionToCore() throws Exception {
      WrapperIdentity identity = captureIdentitySentToCore();

      assertEquals("JDBC", identity.getDriverName());
      assertEquals(SnowflakeDriver.CLIENT_APP_VERSION, identity.getDriverVersion());
    }

    @Test
    void shouldSendJvmRuntimeInfoToCore() throws Exception {
      WrapperIdentity identity = captureIdentitySentToCore();

      // wrapperIdentity() reads these straight from the running JVM's system properties, both of
      // which are always populated on a HotSpot/OpenJDK test runtime.
      assertEquals(System.getProperty("java.vm.name"), identity.getLanguageRuntime());
      assertEquals(System.getProperty("java.version"), identity.getLanguageVersion());
    }
  }

  abstract static class MockCoreApiConnectionSupport {

    CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = SnowflakeConnectionImplTestFixtures.newMockCoreApiWithCloseStubs();
    }

    SnowflakeConnectionImpl createConnection() throws SQLException {
      return SnowflakeConnectionImplTestFixtures.newTestConnection(mockCoreApi);
    }
  }
}
