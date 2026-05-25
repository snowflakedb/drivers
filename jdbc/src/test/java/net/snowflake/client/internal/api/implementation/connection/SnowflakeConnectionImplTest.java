package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.clearInvocations;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
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

public class SnowflakeConnectionImplTest {

  @Test
  public void toConfigSettingMapsLongValuesToInt64() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(1234567890123L);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, configSetting.getValueCase());
    assertEquals(1234567890123L, configSetting.getIntValue());
  }

  @Test
  public void toConfigSettingMapsStringValuesToStringValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting("test-account");

    assertEquals(ConfigSetting.ValueCase.STRING_VALUE, configSetting.getValueCase());
    assertEquals("test-account", configSetting.getStringValue());
  }

  @Test
  public void toConfigSettingMapsBooleanValuesToBoolValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(Boolean.TRUE);

    assertEquals(ConfigSetting.ValueCase.BOOL_VALUE, configSetting.getValueCase());
    assertTrue(configSetting.getBoolValue());
  }

  @Test
  public void toConfigSettingMapsDoubleValuesToDoubleValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(3.14d);

    assertEquals(ConfigSetting.ValueCase.DOUBLE_VALUE, configSetting.getValueCase());
    assertEquals(3.14d, configSetting.getDoubleValue());
  }

  @Test
  public void toConfigSettingReturnsNullForUnsupportedValues() {
    assertNull(SnowflakeConnectionImpl.toConfigSetting(new Object()));
  }

  @Test
  public void stripVersionSuffixReturnsInputWhenNoSpace() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1"));
  }

  @Test
  public void stripVersionSuffixDropsBuildSuffix() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 abcdef"));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 a b c"));
  }

  @Test
  public void stripVersionSuffixHandlesEmptyAndNull() {
    assertNull(SnowflakeConnectionImpl.stripVersionSuffix(null));
    assertEquals("", SnowflakeConnectionImpl.stripVersionSuffix(""));
    assertEquals("", SnowflakeConnectionImpl.stripVersionSuffix("   "));
  }

  @Test
  public void stripVersionSuffixTrimsLeadingAndTrailingWhitespace() {
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix(" 8.46.1"));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("8.46.1 "));
    assertEquals("8.46.1", SnowflakeConnectionImpl.stripVersionSuffix("  8.46.1 abc  "));
  }

  @Nested
  class Close {

    private final DatabaseHandle dbHandle =
        DatabaseHandle.newBuilder().setId(1).setMagic(100).build();
    private final ConnectionHandle connHandle =
        ConnectionHandle.newBuilder().setId(2).setMagic(200).build();

    private CoreDriverApi mockCoreApi;

    @BeforeEach
    void setUp() throws Exception {
      mockCoreApi = mock(CoreDriverApi.class);
      when(mockCoreApi.databaseNew())
          .thenReturn(DatabaseNewResponse.newBuilder().setDbHandle(dbHandle).build());
      when(mockCoreApi.databaseInit(any())).thenReturn(DatabaseInitResponse.getDefaultInstance());
      when(mockCoreApi.connectionNew())
          .thenReturn(ConnectionNewResponse.newBuilder().setConnHandle(connHandle).build());
      when(mockCoreApi.connectionSetOptions(any(), any()))
          .thenReturn(ConnectionSetOptionsResponse.getDefaultInstance());
      when(mockCoreApi.connectionInit(any(), any(), any()))
          .thenReturn(ConnectionInitResponse.getDefaultInstance());
    }

    private SnowflakeConnectionImpl createConnection() throws SQLException {
      Properties props = new Properties();
      props.setProperty("account", "test_account");
      props.setProperty("user", "test_user");
      props.setProperty("password", "test_password");
      return new SnowflakeConnectionImpl(
          "jdbc:snowflake://test.snowflakecomputing.com", props, mockCoreApi);
    }

    private void stubSuccessfulClose() throws Exception {
      when(mockCoreApi.connectionClose(any()))
          .thenReturn(ConnectionCloseResponse.getDefaultInstance());
      when(mockCoreApi.connectionRelease(any()))
          .thenReturn(ConnectionReleaseResponse.getDefaultInstance());
      when(mockCoreApi.databaseRelease(any()))
          .thenReturn(DatabaseReleaseResponse.getDefaultInstance());
    }

    @Test
    void sendsConnectionCloseAndReleasesHandles() throws Exception {
      stubSuccessfulClose();

      Connection conn = createConnection();
      conn.close();

      verify(mockCoreApi).connectionClose(any());
      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
    }

    @Test
    void isClosedReturnsTrueAfterClose() throws Exception {
      stubSuccessfulClose();

      Connection conn = createConnection();
      assertFalse(conn.isClosed());

      conn.close();
      assertTrue(conn.isClosed());
    }

    @Test
    void isIdempotent() throws Exception {
      stubSuccessfulClose();

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
      when(mockCoreApi.connectionRelease(any()))
          .thenReturn(ConnectionReleaseResponse.getDefaultInstance());
      when(mockCoreApi.databaseRelease(any()))
          .thenReturn(DatabaseReleaseResponse.getDefaultInstance());

      Connection conn = createConnection();
      assertThrows(SQLException.class, conn::close);

      verify(mockCoreApi).connectionRelease(any());
      verify(mockCoreApi).databaseRelease(any());
      assertTrue(conn.isClosed());
    }

    @Test
    void operationsThrowAfterClose() throws Exception {
      stubSuccessfulClose();

      Connection conn = createConnection();
      conn.close();

      assertThrows(SQLException.class, conn::createStatement);
      assertThrows(SQLException.class, () -> conn.prepareStatement("SELECT 1"));
    }

    @Test
    void concurrentCallsResultInSingleLogout() throws Exception {
      stubSuccessfulClose();

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
      stubSuccessfulClose();

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
      stubSuccessfulClose();
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
    void manuallyClosedStatementIsNotDoubleClosedOnConnectionClose() throws Exception {
      stubSuccessfulClose();
      StatementHandle stmtHandle = StatementHandle.newBuilder().setId(10).setMagic(1000).build();
      when(mockCoreApi.statementNew(any()))
          .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());
      when(mockCoreApi.statementRelease(any()))
          .thenReturn(StatementReleaseResponse.getDefaultInstance());

      Connection conn = createConnection();
      Statement stmt = conn.createStatement();
      stmt.close();

      clearInvocations(mockCoreApi);
      stubSuccessfulClose();
      conn.close();

      verify(mockCoreApi, never()).statementRelease(any());
    }
  }
}
