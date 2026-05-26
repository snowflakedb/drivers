package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.BatchUpdateException;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
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

  @Test
  void executeBatchReturnsUpdateCountsForEachAddedSql() throws Exception {
    Statement stmt = createStatement();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L), insertResponse(2L), insertResponse(3L));

    stmt.addBatch("INSERT INTO t VALUES (1)");
    stmt.addBatch("INSERT INTO t VALUES (2), (3)");
    stmt.addBatch("INSERT INTO t VALUES (4), (5), (6)");

    int[] counts = stmt.executeBatch();

    assertArrayEquals(new int[] {1, 2, 3}, counts);
    verify(mockCoreApi, times(3)).statementExecuteQuery(any(), isNull());
  }

  @Test
  void clearBatchEmptiesAccumulatedSql() throws Exception {
    Statement stmt = createStatement();

    stmt.addBatch("INSERT INTO t VALUES (1)");
    stmt.addBatch("INSERT INTO t VALUES (2)");
    stmt.clearBatch();

    int[] counts = stmt.executeBatch();
    assertEquals(0, counts.length);
    verify(mockCoreApi, times(0)).statementExecuteQuery(any(), isNull());
  }

  @Test
  void executeBatchContinuesAfterFailureAndAlignsBatchQueryIds() throws Exception {
    Statement stmt = createStatement();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-ok-1"))
        .thenThrow(new SQLException("boom", "42000", 1234))
        .thenReturn(insertResponse(3L, "qid-ok-3"));

    stmt.addBatch("INSERT INTO t VALUES (1)");
    stmt.addBatch("INSERT INTO t VALUES (2)");
    stmt.addBatch("INSERT INTO t VALUES (3), (4), (5)");

    BatchUpdateException ex = assertThrows(BatchUpdateException.class, stmt::executeBatch);
    assertArrayEquals(
        new int[] {1, Statement.EXECUTE_FAILED, 3},
        ex.getUpdateCounts(),
        "continue-on-error: failed slot is EXECUTE_FAILED, surrounding entries keep real counts");
    assertEquals("42000", ex.getSQLState());
    assertEquals(1234, ex.getErrorCode());

    java.util.List<String> ids =
        ((net.snowflake.client.api.statement.SnowflakeStatement) stmt).getBatchQueryIDs();
    assertArrayEquals(
        new String[] {"qid-ok-1", null, "qid-ok-3"},
        ids.toArray(new String[0]),
        "batchQueryIds positionally aligns with updateCounts; failed slot is null");

    int[] secondRun = stmt.executeBatch();
    assertEquals(0, secondRun.length, "batch is cleared even after a failed executeBatch");
  }

  @Test
  void getQueryIdIsNullOnFreshStatement() throws Exception {
    Statement stmt = createStatement();
    assertNull(((SnowflakeStatement) stmt).getQueryID());
  }

  @Test
  void getQueryIdReflectsLastSuccessfulExecute() throws Exception {
    Statement stmt = createStatement();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-A"))
        .thenReturn(insertResponse(1L, "qid-B"));

    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    assertEquals("qid-A", ((SnowflakeStatement) stmt).getQueryID());

    stmt.executeUpdate("INSERT INTO t VALUES (2)");
    assertEquals(
        "qid-B",
        ((SnowflakeStatement) stmt).getQueryID(),
        "Second execute should overwrite the first query ID");
  }

  @Test
  void getQueryIdIsPreservedFromFailedExecuteWhenServerProvidesIt() throws Exception {
    Statement stmt = createStatement();
    // Seed a prior successful execute so a buggy implementation that simply ignored the failure
    // path on a fresh field would not pass: the failed execute MUST overwrite the prior value.
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-prior-success"))
        .thenThrow(driverExceptionWithQueryId("qid-failed"));

    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    assertEquals("qid-prior-success", ((SnowflakeStatement) stmt).getQueryID());

    assertThrows(SQLException.class, () -> stmt.executeUpdate("INSERT INTO t VALUES (2)"));
    assertEquals(
        "qid-failed",
        ((SnowflakeStatement) stmt).getQueryID(),
        "Failed execute should overwrite the prior successful query ID with the server-side one");
  }

  @Test
  void getQueryIdIsNullAfterFailedExecuteWithoutServerQueryId() throws Exception {
    Statement stmt = createStatement();
    // First a successful execute populates the field.
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-stale"))
        .thenThrow(new SQLException("transport error"));

    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    assertEquals("qid-stale", ((SnowflakeStatement) stmt).getQueryID());

    assertThrows(SQLException.class, () -> stmt.executeUpdate("INSERT INTO t VALUES (2)"));
    assertNull(
        ((SnowflakeStatement) stmt).getQueryID(),
        "Failed execute with no server-side query id wipes any stale value");
  }

  private static SnowflakeSQLException driverExceptionWithQueryId(String queryId) {
    DriverException error =
        DriverException.newBuilder().setMessage("server-side failure").setQueryId(queryId).build();
    return new SnowflakeSQLException(error, new RuntimeException("test cause"));
  }

  private static ExecuteQueryResponse insertResponse(long rowsAffected) {
    return insertResponse(rowsAffected, "qid-" + rowsAffected);
  }

  private static ExecuteQueryResponse insertResponse(long rowsAffected, String queryId) {
    long INSERT_TYPE_ID = 0x3000L + 0x100L;
    ResultSetDescriptor descriptor =
        ResultSetDescriptor.newBuilder()
            .setQueryId(queryId)
            .setStatementTypeId(INSERT_TYPE_ID)
            .setRowsAffected(rowsAffected)
            .build();
    ResultSetResponse single =
        ResultSetResponse.newBuilder().setResultDescriptor(descriptor).build();
    return ExecuteQueryResponse.newBuilder().setSingle(single).build();
  }
}
