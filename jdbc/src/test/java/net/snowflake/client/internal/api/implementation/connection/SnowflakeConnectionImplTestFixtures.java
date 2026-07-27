package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCloseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionCommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionRollbackResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetAutocommitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseReleaseResponse;

final class SnowflakeConnectionImplTestFixtures {

  static final DatabaseHandle DEFAULT_DB_HANDLE =
      DatabaseHandle.newBuilder().setId(1).setMagic(100).build();
  static final ConnectionHandle DEFAULT_CONN_HANDLE =
      ConnectionHandle.newBuilder().setId(2).setMagic(200).build();
  private static final String DEFAULT_JDBC_URL = "jdbc:snowflake://test.snowflakecomputing.com";

  private SnowflakeConnectionImplTestFixtures() {}

  static CoreDriverApi newMockCoreApiWithCloseStubs() throws Exception {
    CoreDriverApi mockCoreApi = mock(CoreDriverApi.class);
    when(mockCoreApi.databaseNew())
        .thenReturn(DatabaseNewResponse.newBuilder().setDbHandle(DEFAULT_DB_HANDLE).build());
    when(mockCoreApi.databaseInit(any())).thenReturn(DatabaseInitResponse.getDefaultInstance());
    when(mockCoreApi.connectionNew())
        .thenReturn(ConnectionNewResponse.newBuilder().setConnHandle(DEFAULT_CONN_HANDLE).build());
    when(mockCoreApi.connectionSetOptions(any(), any()))
        .thenReturn(ConnectionSetOptionsResponse.getDefaultInstance());
    when(mockCoreApi.connectionSetAutocommit(any(), anyBoolean()))
        .thenReturn(ConnectionSetAutocommitResponse.getDefaultInstance());
    when(mockCoreApi.connectionInit(any(), any(), any()))
        .thenReturn(ConnectionInitResponse.getDefaultInstance());
    when(mockCoreApi.connectionGetParameter(any(), eq("AUTOCOMMIT")))
        .thenReturn(ConnectionGetParameterResponse.getDefaultInstance());
    when(mockCoreApi.connectionCommit(any()))
        .thenReturn(ConnectionCommitResponse.getDefaultInstance());
    when(mockCoreApi.connectionRollback(any()))
        .thenReturn(ConnectionRollbackResponse.getDefaultInstance());
    when(mockCoreApi.connectionGetInfo(any()))
        .thenReturn(ConnectionGetInfoResponse.getDefaultInstance());
    when(mockCoreApi.connectionClose(any()))
        .thenReturn(ConnectionCloseResponse.getDefaultInstance());
    when(mockCoreApi.connectionRelease(any()))
        .thenReturn(ConnectionReleaseResponse.getDefaultInstance());
    when(mockCoreApi.databaseRelease(any()))
        .thenReturn(DatabaseReleaseResponse.getDefaultInstance());
    return mockCoreApi;
  }

  static SnowflakeConnectionImpl newTestConnection(CoreDriverApi mockCoreApi) throws SQLException {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "dummy");
    return new SnowflakeConnectionImpl(DEFAULT_JDBC_URL, props, mockCoreApi);
  }

  static void assertConnectionClosedSqlException(SQLException ex) {
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
  }

  static void assertConnectionClosedClientInfoException(SQLClientInfoException ex) {
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
  }

  static void assertFeatureNotSupported(SQLFeatureNotSupportedException ex) {
    assertEquals(ErrorCode.FEATURE_UNSUPPORTED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.FEATURE_UNSUPPORTED.getMessageCode(), ex.getErrorCode());
  }
}
