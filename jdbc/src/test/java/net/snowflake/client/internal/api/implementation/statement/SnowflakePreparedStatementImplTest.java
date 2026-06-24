package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.notNull;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.BatchUpdateException;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class SnowflakePreparedStatementImplTest {

  private static final long INSERT_TYPE_ID = 0x3000L + 0x100L;

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

  private SnowflakePreparedStatementImpl createPreparedStatement(String sql) {
    return new SnowflakePreparedStatementImpl(mockConnection, sql, mockCoreApi);
  }

  @Test
  void executeBatchSendsSingleQueryAndExpandsAggregateUpdateCount() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(3L));

    ps.setInt(1, 1);
    ps.addBatch();
    ps.setInt(1, 2);
    ps.addBatch();
    ps.setInt(1, 3);
    ps.addBatch();

    int[] counts = ps.executeBatch();

    assertArrayEquals(new int[] {1, 1, 1}, counts);
    verify(mockCoreApi, times(1)).statementExecuteQuery(any(), notNull(QueryBindings.class));
  }

  @Test
  void executeBatchReturnsSuccessNoInfoWhenServerCountDiffersFromBatchSize() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(7L));

    ps.setInt(1, 1);
    ps.addBatch();
    ps.setInt(1, 2);
    ps.addBatch();

    int[] counts = ps.executeBatch();
    assertArrayEquals(
        new int[] {Statement.SUCCESS_NO_INFO, Statement.SUCCESS_NO_INFO},
        counts,
        "non-matching aggregate must fan out to one SUCCESS_NO_INFO per batch row");
  }

  @Test
  void executeBatchOnEmptyBatchReturnsEmptyArrayAndSkipsRpc() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");

    int[] counts = ps.executeBatch();
    assertEquals(0, counts.length);
    verify(mockCoreApi, times(0)).statementExecuteQuery(any(), any());
  }

  @Test
  void executeLargeBatchReturnsLongCountsExpandedFromAggregate() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(2L));

    ps.setInt(1, 1);
    ps.addBatch();
    ps.setInt(1, 2);
    ps.addBatch();

    long[] counts = ps.executeLargeBatch();
    assertEquals(2, counts.length);
    assertEquals(1L, counts[0]);
    assertEquals(1L, counts[1]);
  }

  @Test
  void preparedStatementRejectsAddBatchWithSql() {
    PreparedStatement ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> ps.addBatch("INSERT INTO t VALUES (1)"));
  }

  @Test
  void executeBatchOnFailureWrapsAsBatchUpdateExceptionWithExecuteFailedSlots() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenThrow(new SQLException("boom", "42000", 7));

    ps.setInt(1, 1);
    ps.addBatch();
    ps.setInt(1, 2);
    ps.addBatch();
    ps.setInt(1, 3);
    ps.addBatch();

    BatchUpdateException ex = assertThrows(BatchUpdateException.class, ps::executeBatch);
    assertArrayEquals(
        new int[] {Statement.EXECUTE_FAILED, Statement.EXECUTE_FAILED, Statement.EXECUTE_FAILED},
        ex.getUpdateCounts());
    assertEquals("42000", ex.getSQLState());
    assertEquals(7, ex.getErrorCode());

    int[] second = ps.executeBatch();
    assertEquals(0, second.length, "batch is cleared even after a failed executeBatch");
  }

  @Test
  void executeBatchPopulatesAndResetsBatchQueryIds() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(1L));

    ps.setInt(1, 1);
    ps.addBatch();
    ps.executeBatch();
    assertEquals(1, ps.getBatchQueryIDs().size(), "PS array-bind batch is a single round-trip");

    ps.setInt(1, 2);
    ps.addBatch();
    ps.executeBatch();
    assertEquals(
        1,
        ps.getBatchQueryIDs().size(),
        "second executeBatch resets and re-populates rather than appending");
  }

  @Test
  void getQueryIdReflectsLastSuccessfulExecuteQuery() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(1L, "qid-prepared"));

    ps.setInt(1, 1);
    ps.executeUpdate();

    assertEquals("qid-prepared", ps.getQueryID());
  }

  @Test
  void getQueryIdIsPreservedFromFailedPreparedExecuteWhenServerProvidesIt() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("INSERT INTO t VALUES (?)");
    when(mockCoreApi.statementExecuteQuery(any(), notNull(QueryBindings.class)))
        .thenReturn(insertResponse(1L, "qid-prior"))
        .thenThrow(driverExceptionWithQueryId("qid-prepared-failed"));

    ps.setInt(1, 1);
    ps.executeUpdate();
    assertEquals("qid-prior", ps.getQueryID());

    ps.setInt(1, 2);
    assertThrows(SQLException.class, ps::executeUpdate);
    assertEquals(
        "qid-prepared-failed",
        ps.getQueryID(),
        "Failed prepared execute should overwrite the prior id with the server-side one");
  }

  @Test
  void getParameterMetaDataFallsBackToEmptyOnIgnoredDescribeErrorAndCachesResult()
      throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("CREATE TABLE t (id INT)");
    // Error code 7 (statement cannot be prepared) is ignored in describe mode.
    when(mockCoreApi.statementPrepare(any())).thenThrow(driverExceptionWithVendorCode(7));

    assertEquals(0, ps.getParameterMetaData().getParameterCount());
    // Second call must reuse the cached empty metadata rather than re-issuing describe.
    assertEquals(0, ps.getParameterMetaData().getParameterCount());
    verify(mockCoreApi, times(1)).statementPrepare(any());
  }

  @Test
  void getParameterMetaDataPropagatesNonIgnoredDescribeError() throws Exception {
    SnowflakePreparedStatementImpl ps = createPreparedStatement("SELECT ?");
    when(mockCoreApi.statementPrepare(any())).thenThrow(driverExceptionWithVendorCode(1003));

    assertThrows(SnowflakeSQLException.class, ps::getParameterMetaData);
  }

  private static SnowflakeSQLException driverExceptionWithVendorCode(int vendorCode) {
    DriverException error =
        DriverException.newBuilder()
            .setMessage("describe failure")
            .setVendorCode(vendorCode)
            .build();
    return new SnowflakeSQLException(error, new RuntimeException("test cause"));
  }

  private static SnowflakeSQLException driverExceptionWithQueryId(String queryId) {
    DriverException error =
        DriverException.newBuilder().setMessage("server-side failure").setQueryId(queryId).build();
    return new SnowflakeSQLException(error, new RuntimeException("test cause"));
  }

  private static ExecuteQueryResponse insertResponse(long rowsAffected) {
    return insertResponse(rowsAffected, "qid");
  }

  private static ExecuteQueryResponse insertResponse(long rowsAffected, String queryId) {
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
