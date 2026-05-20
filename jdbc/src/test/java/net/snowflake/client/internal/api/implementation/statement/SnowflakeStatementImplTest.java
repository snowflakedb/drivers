package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class SnowflakeStatementImplTest {

  private final ConnectionHandle connHandle =
      ConnectionHandle.newBuilder().setId(1).setMagic(100).build();
  private final StatementHandle stmtHandle =
      StatementHandle.newBuilder().setId(10).setMagic(1000).build();

  private CoreDriverApi mockCoreApi;
  private InternalSnowflakeConnection mockConnection;

  @BeforeEach
  void setUp() throws Exception {
    mockCoreApi = mock(CoreDriverApi.class);
    mockConnection = mock(InternalSnowflakeConnection.class);
    when(mockConnection.getHandle()).thenReturn(connHandle);
    when(mockCoreApi.statementNew(any()))
        .thenReturn(StatementNewResponse.newBuilder().setStmtHandle(stmtHandle).build());
    when(mockCoreApi.statementRelease(any()))
        .thenReturn(StatementReleaseResponse.getDefaultInstance());
  }

  private Statement createStatement() {
    return new SnowflakeStatementImpl(mockConnection, mockCoreApi);
  }

  @Test
  void closeReleasesStatementHandle() throws Exception {
    Statement stmt = createStatement();

    stmt.close();

    verify(mockCoreApi).statementRelease(any());
    assertTrue(stmt.isClosed());
  }

  @Test
  void closeDeregistersFromConnection() throws Exception {
    Statement stmt = createStatement();

    stmt.close();

    verify(mockConnection).removeStatement(stmt);
  }

  @Test
  void doubleCloseDoesNotReleaseHandleTwice() throws Exception {
    Statement stmt = createStatement();

    stmt.close();
    stmt.close();

    verify(mockCoreApi, times(1)).statementRelease(any());
    verify(mockConnection, times(1)).removeStatement(stmt);
  }

  @Test
  void closeSucceedsEvenWhenReleaseThrows() throws Exception {
    when(mockCoreApi.statementRelease(any())).thenThrow(new SQLException("release failed"));

    Statement stmt = createStatement();
    stmt.close();

    assertTrue(stmt.isClosed());
    verify(mockConnection).removeStatement(stmt);
  }

  @Test
  void operationsThrowAfterClose() throws Exception {
    Statement stmt = createStatement();
    stmt.close();

    assertThrows(SQLException.class, () -> stmt.execute("SELECT 1"));
    assertThrows(SQLException.class, () -> stmt.executeQuery("SELECT 1"));
  }
}
