package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.ArgumentMatchers.notNull;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.lang.ref.WeakReference;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.BatchUpdateException;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.NativeBuffer;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.MultiStatementResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteAsyncResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementNewResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementReleaseResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementSetOptionsResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;
import org.mockito.ArgumentCaptor;

@Isolated("asserts the process-wide NativeBuffer.allocatedBytes() counter")
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

  /**
   * The public-contract view: the decorator that wraps the raw impl and translates the runtime
   * carriers it throws into the checked {@link java.sql.SQLException} types the JDBC API promises.
   * Tests asserting the public exception contract must go through this, not the raw impl.
   */
  private Statement createDecoratedStatement() {
    return new DecoratedSnowflakeStatementImpl(
        new SnowflakeStatementImpl(mockConnection, mockCoreApi), Telemetry.NOOP);
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
    when(mockCoreApi.statementRelease(any()))
        .thenThrow(
            new CoreException(
                DriverException.newBuilder().setMessage("release failed").build(), null));

    Statement stmt = createStatement();
    stmt.close();

    assertTrue(stmt.isClosed());
    verify(mockConnection).removeStatement(stmt);
  }

  @Test
  void shouldReleaseStatementHandleWhenUnclosedStatementIsGarbageCollected() throws Exception {
    CountDownLatch released = new CountDownLatch(1);
    when(mockCoreApi.statementRelease(stmtHandle))
        .thenAnswer(
            invocation -> {
              released.countDown();
              return StatementReleaseResponse.getDefaultInstance();
            });

    createStatement();

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    while (released.getCount() > 0 && System.nanoTime() < deadline) {
      System.gc();
      released.await(100, TimeUnit.MILLISECONDS);
    }
    assertEquals(0, released.getCount());
  }

  @Test
  void shouldNotRetainPreviousResultSetDroppedByCallerWhileStatementStaysOpen() throws Exception {
    when(mockCoreApi.statementExecuteQuery(any(), isNull())).thenReturn(insertResponse(1L));
    SnowflakeResultSetMetaDataImpl metaData = mock(SnowflakeResultSetMetaDataImpl.class);
    when(metaData.getColumnNames()).thenReturn(Collections.emptyList());
    SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi);

    WeakReference<InternalResultSet> previous = installCurrentResultSet(stmt, metaData);
    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    WeakReference<InternalResultSet> current = installCurrentResultSet(stmt, metaData);

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    while (previous.get() != null && System.nanoTime() < deadline) {
      System.gc();
      Thread.sleep(100);
    }

    assertNull(previous.get(), "a result set held only by openResultSets must be collectable");
    assertNotNull(current.get(), "the current result set must stay reachable");
    assertFalse(stmt.isClosed());
  }

  @Test
  void shouldThrowSqlExceptionForOperationsAfterClose() throws Exception {
    Statement stmt = createDecoratedStatement();
    stmt.close();

    assertThrows(SQLException.class, () -> stmt.execute("SELECT 1"));
    assertThrows(SQLException.class, () -> stmt.executeQuery("SELECT 1"));
  }

  @Test
  void shouldExposeConnectionWithoutClosedCheckForResultSetConstruction() throws Exception {
    SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi);
    stmt.close();

    // ResultSetFactory.parametersOf() relies on getConnectionInternal() to build the conversion
    // context during ResultSet construction; it must not enforce checkClosed(), so a statement
    // concurrently closed after execute() still yields a ResultSet (legacy parity).
    assertSame(mockConnection, stmt.getConnectionInternal());

    // The public JDBC getConnection() still enforces the closed check.
    Statement decorated = new DecoratedSnowflakeStatementImpl(stmt, Telemetry.NOOP);
    SQLException ex = assertThrows(SQLException.class, decorated::getConnection);
    assertEquals("Statement is closed", ex.getMessage());
  }

  @Test
  void shouldReportConnectionClosedWhenBothConnectionAndStatementAreClosed() throws Exception {
    when(mockConnection.isClosed()).thenReturn(true);
    Statement stmt = createDecoratedStatement();
    stmt.close();

    SQLException ex = assertThrows(SQLException.class, () -> stmt.execute("SELECT 1"));

    assertEquals("Connection is closed", ex.getMessage());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
  }

  @Test
  void shouldReportConnectionClosedWhenConnectionIsClosedButStatementStaysOpen() throws Exception {
    when(mockConnection.isClosed()).thenReturn(true);
    Statement stmt = createDecoratedStatement();
    assertFalse(stmt.isClosed());

    SQLException ex = assertThrows(SQLException.class, () -> stmt.execute("SELECT 1"));

    assertEquals("Connection is closed", ex.getMessage());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), ex.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
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
  void shouldContinueBatchAfterFailureAndAlignBatchQueryIds() throws Exception {
    // Route through the decorator so executeBatch surfaces the checked BatchUpdateException. The
    // failure is seeded as a CoreException — the type the real CoreDriverApi facade produces and
    // the only type StatementBatch catches.
    Statement stmt = createDecoratedStatement();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-ok-1"))
        .thenThrow(
            new CoreException(
                DriverException.newBuilder()
                    .setMessage("boom")
                    .setSqlState("42000")
                    .setVendorCode(1234)
                    .build(),
                null))
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
  void executeLargeBatchReturnsExactLongCountsForEachAddedSql() throws Exception {
    Statement stmt = createStatement();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L), insertResponse(2L), insertResponse(3L));

    stmt.addBatch("INSERT INTO t VALUES (1)");
    stmt.addBatch("INSERT INTO t VALUES (2), (3)");
    stmt.addBatch("INSERT INTO t VALUES (4), (5), (6)");

    long[] counts = stmt.executeLargeBatch();

    assertArrayEquals(new long[] {1L, 2L, 3L}, counts);
    verify(mockCoreApi, times(3)).statementExecuteQuery(any(), isNull());
    assertNull(stmt.getResultSet());
    assertEquals(-1, stmt.getUpdateCount());
    assertEquals(0, stmt.executeLargeBatch().length, "batch is cleared after a successful call");
  }

  @Test
  void executeLargeBatchPreservesCountAboveIntegerMaxWhileExecuteBatchNarrows() throws Exception {
    Statement stmt = createStatement();
    long largeCount = (long) Integer.MAX_VALUE + 1L;
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(largeCount), insertResponse(Integer.MAX_VALUE));

    stmt.addBatch("INSERT INTO t SELECT * FROM big");
    stmt.addBatch("INSERT INTO t SELECT * FROM max_int");

    long[] largeCounts = stmt.executeLargeBatch();
    assertArrayEquals(
        new long[] {largeCount, Integer.MAX_VALUE},
        largeCounts,
        "executeLargeBatch keeps counts above Integer.MAX_VALUE; Integer.MAX_VALUE itself is unchanged");

    stmt.addBatch("INSERT INTO t SELECT * FROM big");
    stmt.addBatch("INSERT INTO t SELECT * FROM max_int");
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(largeCount), insertResponse(Integer.MAX_VALUE));

    int[] intCounts = stmt.executeBatch();
    assertArrayEquals(
        new int[] {Statement.SUCCESS_NO_INFO, Integer.MAX_VALUE},
        intCounts,
        "executeBatch still maps overflow to SUCCESS_NO_INFO");
  }

  @Test
  void executeLargeBatchContinuesAfterFailureAndPreservesLargeCounts() throws Exception {
    Statement stmt = createDecoratedStatement();
    long largeCount = (long) Integer.MAX_VALUE + 1L;
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(largeCount, "qid-ok-1"))
        .thenThrow(
            new CoreException(
                DriverException.newBuilder()
                    .setMessage("boom")
                    .setSqlState("42000")
                    .setVendorCode(1234)
                    .build(),
                null))
        .thenReturn(insertResponse(3L, "qid-ok-3"));

    stmt.addBatch("INSERT INTO t SELECT * FROM big");
    stmt.addBatch("INSERT INTO t VALUES (2)");
    stmt.addBatch("INSERT INTO t VALUES (3), (4), (5)");

    BatchUpdateException ex = assertThrows(BatchUpdateException.class, stmt::executeLargeBatch);
    assertArrayEquals(
        new long[] {largeCount, Statement.EXECUTE_FAILED, 3L},
        ex.getLargeUpdateCounts(),
        "failed slot is EXECUTE_FAILED; surrounding counts keep full long fidelity");
    assertEquals("42000", ex.getSQLState());
    assertEquals(1234, ex.getErrorCode());

    java.util.List<String> ids =
        ((net.snowflake.client.api.statement.SnowflakeStatement) stmt).getBatchQueryIDs();
    assertArrayEquals(
        new String[] {"qid-ok-1", null, "qid-ok-3"},
        ids.toArray(new String[0]),
        "batchQueryIds positionally aligns with large update counts; failed slot is null");

    assertNull(stmt.getResultSet());
    assertEquals(-1, stmt.getUpdateCount());
    assertEquals(0, stmt.executeLargeBatch().length, "batch is cleared even after a failed call");
  }

  @Test
  void executeLargeBatchOnEmptyBatchReturnsEmptyArrayAndSkipsRpc() throws Exception {
    Statement stmt = createStatement();

    long[] counts = stmt.executeLargeBatch();

    assertEquals(0, counts.length);
    verify(mockCoreApi, times(0)).statementExecuteQuery(any(), isNull());
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
  void shouldPreserveQueryIdFromFailedExecuteWhenServerProvidesIt() throws Exception {
    Statement stmt = createStatement();
    // Seed a prior successful execute so a buggy impl that ignored the failure path on a fresh
    // field would not pass: the failed execute MUST overwrite the prior value. The failure is a
    // CoreException (what the real CoreDriverApi facade surfaces), carrying the server-side query
    // id.
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(1L, "qid-prior-success"))
        .thenThrow(driverExceptionWithQueryId("qid-failed"));

    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    assertEquals("qid-prior-success", ((SnowflakeStatement) stmt).getQueryID());

    assertThrows(CoreException.class, () -> stmt.executeUpdate("INSERT INTO t VALUES (2)"));
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
        .thenThrow(
            new CoreException(
                DriverException.newBuilder().setMessage("transport error").build(), null));

    stmt.executeUpdate("INSERT INTO t VALUES (1)");
    assertEquals("qid-stale", ((SnowflakeStatement) stmt).getQueryID());

    assertThrows(CoreException.class, () -> stmt.executeUpdate("INSERT INTO t VALUES (2)"));
    assertNull(
        ((SnowflakeStatement) stmt).getQueryID(),
        "Failed execute with no server-side query id wipes any stale value");
  }

  @Test
  void shouldAcceptBatchIDWithoutThrowing() throws Exception {
    Statement stmt = createStatement();
    assertDoesNotThrow(() -> ((SnowflakeStatement) stmt).setBatchID("batch-1"));
  }

  @Test
  void shouldReturnEmptyGeneratedKeys() throws Exception {
    try (Statement stmt = createDecoratedStatement();
        ResultSet keys = stmt.getGeneratedKeys()) {
      assertFalse(keys.isBeforeFirst());
      assertFalse(keys.isAfterLast());
      assertFalse(keys.isFirst());
      assertFalse(keys.isLast());
      assertFalse(keys.next());
      assertFalse(keys.isClosed());
      assertEquals(0, keys.getInt(1));
      assertFalse(keys.wasNull());
      assertEquals("", keys.getString("col1"));
      assertFalse(keys.wasNull());
      assertNull(keys.getMetaData());
      assertNull(keys.getStatement());
    }
  }

  @Test
  void shouldRejectGeneratedKeysAfterStatementClosed() throws Exception {
    Statement stmt = createDecoratedStatement();
    stmt.close();
    assertThrows(SQLException.class, stmt::getGeneratedKeys);
  }

  @Test
  void shouldRejectExecuteWithGeneratedKeysOtherThanNone() throws Exception {
    try (Statement stmt = createDecoratedStatement()) {
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.execute("INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.executeUpdate("INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.execute("INSERT INTO t VALUES (1)", new int[] {1}));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.execute("INSERT INTO t VALUES (1)", new String[] {"id"}));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.executeUpdate("INSERT INTO t VALUES (1)", new int[] {1}));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> stmt.executeUpdate("INSERT INTO t VALUES (1)", new String[] {"id"}));
    }
  }

  @Test
  void shouldDelegateExecuteQueryInternalToUnboundQueryWhenParametersAreNullOrEmpty()
      throws Exception {
    when(mockCoreApi.statementExecuteQuery(eq(stmtHandle), isNull()))
        .thenReturn(insertResponse(0L, "qid-unbound"));
    when(mockCoreApi.resultSetGetStream(any()))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());

    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      assertNull(stmt.executeQueryInternal("SELECT 1", null));
      assertEquals("qid-unbound", stmt.getQueryID());
      assertNull(stmt.executeQueryInternal("SELECT 1", Collections.emptyList()));
      verify(mockCoreApi, times(2)).statementExecuteQuery(eq(stmtHandle), isNull());
    }
  }

  @Test
  void shouldSerializeOneBasedTextBindingsAndReleaseNativeBufferInExecuteQueryInternal()
      throws Exception {
    long baseline = NativeBuffer.allocatedBytes();
    AtomicReference<String> capturedJson = new AtomicReference<>();
    when(mockCoreApi.resultSetGetStream(any()))
        .thenReturn(ResultSetGetStreamResponse.getDefaultInstance());
    when(mockCoreApi.statementExecuteQuery(eq(stmtHandle), notNull(QueryBindings.class)))
        .thenAnswer(
            invocation -> {
              QueryBindings bindings = invocation.getArgument(1);
              BinaryDataPtr jsonPtr = bindings.getJson();
              long address =
                  ByteBuffer.wrap(jsonPtr.getValue().toByteArray())
                      .order(ByteOrder.LITTLE_ENDIAN)
                      .getLong();
              ByteBuffer view = NativeBuffer.view(address, (int) jsonPtr.getLength());
              byte[] copy = new byte[view.remaining()];
              view.get(copy);
              capturedJson.set(new String(copy, StandardCharsets.UTF_8));
              return insertResponse(0L, "qid-bound");
            })
        .thenThrow(driverExceptionWithQueryId("qid-bound-failed"));

    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      assertNull(
          stmt.executeQueryInternal(
              "SELECT * FROM info WHERE t = ? AND s = ?", Arrays.asList("T_ONE", "S_TWO")));
      assertEquals("qid-bound", stmt.getQueryID());
      assertEquals(
          "{\"1\":{\"type\":\"TEXT\",\"value\":\"T_ONE\"},"
              + "\"2\":{\"type\":\"TEXT\",\"value\":\"S_TWO\"}}",
          capturedJson.get());
      assertEquals(baseline, NativeBuffer.allocatedBytes());

      CoreException ex =
          assertThrows(
              CoreException.class,
              () ->
                  stmt.executeQueryInternal(
                      "SELECT * FROM info WHERE t = ?", Collections.singletonList("T_ONE")));
      assertEquals("qid-bound-failed", ex.getQueryId());
      assertEquals("qid-bound-failed", stmt.getQueryID());
      assertEquals(baseline, NativeBuffer.allocatedBytes());
    }
  }

  @Test
  @SuppressWarnings("unchecked")
  void shouldSetNumericAndStringStatementParametersViaCoreOptions() throws Exception {
    when(mockCoreApi.statementSetOptions(eq(stmtHandle), any()))
        .thenReturn(StatementSetOptionsResponse.getDefaultInstance());

    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      stmt.setParameter("MULTI_STATEMENT_COUNT", 3);
      stmt.setParameter("QUERY_TAG", "etl-job");
    }

    ArgumentCaptor<Map<String, ConfigSetting>> optionsCaptor = ArgumentCaptor.forClass(Map.class);
    verify(mockCoreApi, times(2)).statementSetOptions(eq(stmtHandle), optionsCaptor.capture());
    List<Map<String, ConfigSetting>> calls = optionsCaptor.getAllValues();
    assertEquals(
        ConfigSetting.newBuilder().setIntValue(3L).build(),
        calls.get(0).get("MULTI_STATEMENT_COUNT"));
    assertEquals(
        ConfigSetting.newBuilder().setStringValue("etl-job").build(),
        calls.get(1).get("QUERY_TAG"));
  }

  @Test
  void shouldExecuteAsyncQueryAndCacheDecoratedResultSetWhileParkingPreviousOpenResultSet()
      throws Exception {
    when(mockCoreApi.statementExecuteAsync(eq(stmtHandle), isNull()))
        .thenReturn(StatementExecuteAsyncResponse.newBuilder().setQueryId("qid-async-1").build());
    SnowflakeResultSetMetaDataImpl metaData = mock(SnowflakeResultSetMetaDataImpl.class);
    when(metaData.getColumnNames()).thenReturn(Collections.emptyList());

    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      InternalResultSet previous = ResultSetFactory.createEmpty(stmt, metaData, false);
      stmt.setCurrentResultSet(previous);

      try (ResultSet asyncRs = stmt.executeAsyncQuery("SELECT 1")) {
        assertEquals("qid-async-1", stmt.getQueryID());
        assertSame(asyncRs, stmt.getResultSet());
        assertTrue(stmt.getOpenResultSets().contains(previous));
      }
    }
  }

  @Test
  void shouldRejectExecuteUpdateAndExecuteLargeUpdateWithBindingsWhenStatementProducesResultSet()
      throws Exception {
    SnowflakeStatementImpl raw =
        new SnowflakeStatementImpl(mockConnection, mockCoreApi) {
          @Override
          protected boolean executeWithBindings(
              String sql, PreparedStatementBindingSerializer.NativeBindings bindings) {
            this.queryId = "qid-select";
            return true;
          }
        };

    try (Statement decorated = new DecoratedSnowflakeStatementImpl(raw, Telemetry.NOOP)) {
      SnowflakeSQLException updateEx =
          assertInstanceOf(
              SnowflakeSQLException.class,
              assertThrows(SQLException.class, () -> decorated.executeUpdate("SELECT 1")));
      assertEquals(
          "executeUpdate() cannot be used for statements that produce a ResultSet",
          updateEx.getMessage());
      assertEquals("qid-select", updateEx.getQueryId());
      assertNull(updateEx.getSQLState());
      assertEquals(0, updateEx.getErrorCode());
    }

    // The result-set guard lives on the batch path, executeLargeUpdateWithBindings, which is what
    // callers of large batches hit. Statement.executeLargeUpdate(String) remains the JDK default.
    SFSQLException largeUpdateEx =
        assertThrows(
            SFSQLException.class, () -> raw.executeLargeUpdateWithBindings("SELECT 1", null));
    assertEquals(
        "executeLargeUpdate() cannot be used for statements that produce a ResultSet",
        largeUpdateEx.getMessage());
    assertEquals("qid-select", largeUpdateEx.getQueryId());
  }

  @Test
  void shouldLeavePublicExecuteLargeUpdateAsUnsupportedOperation() throws Exception {
    // BD#statement-execute-large-update-unsupported: the public method is still the JDK
    // default. Large batches use executeLargeUpdateWithBindings instead.
    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      UnsupportedOperationException publicLargeUpdate =
          assertThrows(
              UnsupportedOperationException.class, () -> stmt.executeLargeUpdate("SELECT 1"));
      assertEquals("executeLargeUpdate not implemented", publicLargeUpdate.getMessage());
    }
  }

  @Test
  void shouldRejectUpdateCountExceedingJdbcIntRangeWithQueryIdAndArithmeticCause()
      throws Exception {
    long overflowCount = (long) Integer.MAX_VALUE + 5L;
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(insertResponse(overflowCount, "qid-overflow"));

    try (Statement decorated = createDecoratedStatement()) {
      SnowflakeSQLException updateEx =
          assertInstanceOf(
              SnowflakeSQLException.class,
              assertThrows(
                  SQLException.class,
                  () -> decorated.executeUpdate("INSERT INTO big SELECT * FROM src")));
      assertEquals("Update count exceeds JDBC int range", updateEx.getMessage());
      assertEquals("qid-overflow", updateEx.getQueryId());
      assertInstanceOf(ArithmeticException.class, updateEx.getCause());
      assertNull(updateEx.getSQLState());
      assertEquals(0, updateEx.getErrorCode());

      assertFalse(decorated.execute("INSERT INTO big SELECT * FROM src"));
      SnowflakeSQLException countEx =
          assertInstanceOf(
              SnowflakeSQLException.class,
              assertThrows(SQLException.class, decorated::getUpdateCount));
      assertEquals("Update count exceeds JDBC int range", countEx.getMessage());
      assertEquals("qid-overflow", countEx.getQueryId());
      assertInstanceOf(ArithmeticException.class, countEx.getCause());
    }
  }

  @Test
  void shouldNavigateMultiStatementUpdateCountsAndManageOpenResultSetsAcrossGetMoreResultsModes()
      throws Exception {
    long insertTypeId = 0x3000L + 0x100L;
    ExecuteQueryResponse emptyMulti =
        ExecuteQueryResponse.newBuilder()
            .setMulti(
                MultiStatementResult.newBuilder()
                    .setParent(ResultSetDescriptor.newBuilder().setQueryId("qid-empty-multi"))
                    .build())
            .build();
    ExecuteQueryResponse twoChildMulti =
        ExecuteQueryResponse.newBuilder()
            .setMulti(
                MultiStatementResult.newBuilder()
                    .setParent(ResultSetDescriptor.newBuilder().setQueryId("qid-parent"))
                    .addQueryIds("qid-c1")
                    .addQueryIds("qid-c2")
                    .addStatementTypeIds(insertTypeId)
                    .build())
            .build();
    when(mockCoreApi.statementExecuteQuery(any(), isNull()))
        .thenReturn(emptyMulti)
        .thenReturn(twoChildMulti);
    when(mockCoreApi.connectionGetResultSet(connHandle, "qid-c1"))
        .thenReturn(insertResponse(2L, "qid-c1").getSingle());
    when(mockCoreApi.connectionGetResultSet(connHandle, "qid-c2"))
        .thenReturn(insertResponse(5L, "qid-c2").getSingle());

    SnowflakeResultSetMetaDataImpl metaData = mock(SnowflakeResultSetMetaDataImpl.class);
    when(metaData.getColumnNames()).thenReturn(Collections.emptyList());

    try (SnowflakeStatementImpl stmt = new SnowflakeStatementImpl(mockConnection, mockCoreApi)) {
      assertFalse(stmt.execute(";"));
      assertEquals("qid-empty-multi", stmt.getQueryID());
      assertEquals(-1, stmt.getUpdateCount());
      assertFalse(stmt.getMoreResults());

      assertFalse(stmt.execute("INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"));
      assertEquals("qid-parent", stmt.getQueryID());
      assertEquals(2, stmt.getUpdateCount());

      InternalResultSet rs1 = ResultSetFactory.createEmpty(stmt, metaData, false);
      stmt.setCurrentResultSet(rs1);
      assertFalse(stmt.getMoreResults(Statement.KEEP_CURRENT_RESULT));
      assertFalse(rs1.isClosed());
      assertTrue(stmt.getOpenResultSets().contains(rs1));
      assertEquals(5, stmt.getUpdateCount());

      InternalResultSet rs2 = ResultSetFactory.createEmpty(stmt, metaData, false);
      stmt.setCurrentResultSet(rs2);
      assertFalse(stmt.getMoreResults(Statement.CLOSE_ALL_RESULTS));
      assertTrue(rs1.isClosed());
      assertTrue(rs2.isClosed());
      assertTrue(stmt.getOpenResultSets().isEmpty());
      assertEquals(-1, stmt.getUpdateCount());
    }
  }

  @Test
  void shouldRejectNullBatchSqlAndUnsupportedStatementOperations() throws Exception {
    try (Statement stmt = createDecoratedStatement()) {
      SQLException nullBatchEx = assertThrows(SQLException.class, () -> stmt.addBatch(null));
      assertEquals("addBatch requires a non-null SQL string", nullBatchEx.getMessage());
      assertNull(nullBatchEx.getSQLState());
      assertEquals(0, nullBatchEx.getErrorCode());

      stmt.setFetchDirection(ResultSet.FETCH_FORWARD);
      assertEquals(ResultSet.FETCH_FORWARD, stmt.getFetchDirection());

      SQLFeatureNotSupportedException fetchDirEx =
          assertThrows(
              SQLFeatureNotSupportedException.class,
              () -> stmt.setFetchDirection(ResultSet.FETCH_REVERSE));
      assertEquals("Only FETCH_FORWARD supported", fetchDirEx.getMessage());
      assertNull(fetchDirEx.getSQLState());
      assertEquals(0, fetchDirEx.getErrorCode());

      SQLFeatureNotSupportedException cursorEx =
          assertThrows(SQLFeatureNotSupportedException.class, () -> stmt.setCursorName("cursor_1"));
      assertEquals("setCursorName not supported", cursorEx.getMessage());
      assertNull(cursorEx.getSQLState());
      assertEquals(0, cursorEx.getErrorCode());

      try (ResultSet keys = stmt.getGeneratedKeys()) {
        assertFalse(keys.next());
      }

      SQLFeatureNotSupportedException asyncTimeoutEx =
          assertThrows(
              SQLFeatureNotSupportedException.class,
              () -> ((SnowflakeStatement) stmt).setAsyncQueryTimeout(30));
      assertEquals("setAsyncQueryTimeout not supported", asyncTimeoutEx.getMessage());
      assertNull(asyncTimeoutEx.getSQLState());
      assertEquals(0, asyncTimeoutEx.getErrorCode());
    }
  }

  private static CoreException driverExceptionWithQueryId(String queryId) {
    DriverException error =
        DriverException.newBuilder().setMessage("server-side failure").setQueryId(queryId).build();
    return new CoreException(error, null);
  }

  private static WeakReference<InternalResultSet> installCurrentResultSet(
      SnowflakeStatementImpl stmt, SnowflakeResultSetMetaDataImpl metaData) {
    InternalResultSet resultSet = ResultSetFactory.createEmpty(stmt, metaData, false);
    stmt.setCurrentResultSet(resultSet);
    return new WeakReference<>(resultSet);
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
