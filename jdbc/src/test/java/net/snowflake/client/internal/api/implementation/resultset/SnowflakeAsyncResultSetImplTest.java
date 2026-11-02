package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.Types;
import java.util.Collections;
import java.util.UUID;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class SnowflakeAsyncResultSetImplTest {

  private final String queryId = UUID.randomUUID().toString();

  private InternalSnowflakeConnection mockConnection;
  private SnowflakeStatementImpl mockStatement;

  @BeforeEach
  void setUp() {
    mockConnection = mock(InternalSnowflakeConnection.class);
    mockStatement = mock(SnowflakeStatementImpl.class);
  }

  private SnowflakeAsyncResultSetImpl createAsyncResultSet() {
    return new SnowflakeAsyncResultSetImpl(queryId, mockConnection, mockStatement, false);
  }

  /**
   * Public JDBC view of the same result set: the impl throws unchecked carriers, and the generated
   * decorator is what turns them into the checked {@link SQLException} callers see.
   */
  private ResultSet createDecoratedAsyncResultSet() {
    return new DecoratedSnowflakeAsyncResultSetImpl(createAsyncResultSet(), Telemetry.NOOP);
  }

  private static QueryStatus successStatus() {
    return new QueryStatus(0, 0, "", "", "SUCCESS", 0, "", 0, "SUCCESS", 0, "", 0, "", "");
  }

  private static QueryStatus runningStatus() {
    return new QueryStatus(0, 0, "", "", "RUNNING", 0, "", 0, "RUNNING", 0, "", 0, "", "");
  }

  private static QueryStatus failedStatus() {
    return new QueryStatus(
        0, 1234, "Some error", "", "FAILED_WITH_ERROR", 0, "", 0, "FAILED", 0, "", 0, "", "");
  }

  @Test
  void shouldReturnQueryId() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertEquals(queryId, rs.getQueryID());
    }
  }

  @Test
  void shouldDelegateGetStatusToConnection() throws Exception {
    when(mockConnection.getQueryStatus(queryId)).thenReturn(runningStatus());

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      QueryStatus status = rs.getStatus();

      assertEquals("RUNNING", status.getName());
      verify(mockConnection).getQueryStatus(queryId);
    }
  }

  @Test
  void shouldCacheSuccessStatusAndNotRequery() throws Exception {
    when(mockConnection.getQueryStatus(queryId)).thenReturn(successStatus());

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      rs.getStatus();
      rs.getStatus();
      rs.getStatus();

      verify(mockConnection).getQueryStatus(queryId);
    }
  }

  @Test
  void shouldCacheFailedStatusAndNotRequery() throws Exception {
    when(mockConnection.getQueryStatus(queryId)).thenReturn(failedStatus());

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      rs.getStatus();
      rs.getStatus();
      rs.getStatus();

      verify(mockConnection).getQueryStatus(queryId);
    }
  }

  @Test
  void shouldReturnTrueForIsBeforeFirstBeforeMaterialization() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertTrue(rs.isBeforeFirst());
      verify(mockConnection, never()).createResultSetFromSfqid(any(), any());
    }
  }

  @Test
  void shouldReturnFalseForIsAfterLastBeforeMaterialization() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertFalse(rs.isAfterLast());
    }
  }

  @Test
  void shouldReturnFalseForIsFirstBeforeMaterialization() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertFalse(rs.isFirst());
    }
  }

  @Test
  void shouldReturnZeroForGetRowBeforeMaterialization() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertEquals(0, rs.getRow());
    }
  }

  @Test
  void shouldNotDelegateCloseWithoutMaterialization() throws Exception {
    SnowflakeAsyncResultSetImpl rs = createAsyncResultSet();
    rs.close();

    assertTrue(rs.isClosed());
    verify(mockStatement).removeClosedResultSet(rs);
  }

  @Test
  void shouldCloseDelegateWhenMaterialized() throws Exception {
    SnowflakeResultSetImpl mockDelegate = mock(SnowflakeResultSetImpl.class);
    when(mockConnection.getQueryStatus(queryId)).thenReturn(successStatus());
    when(mockConnection.createResultSetFromSfqid(eq(queryId), eq(mockStatement)))
        .thenReturn(mockDelegate);
    when(mockDelegate.next()).thenReturn(false);

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      rs.next();
    }

    verify(mockDelegate).close();
    verify(mockStatement).removeClosedResultSet(any());
  }

  @Test
  void shouldCloseOwnedStatement() throws Exception {
    SnowflakeAsyncResultSetImpl rs =
        new SnowflakeAsyncResultSetImpl(queryId, mockConnection, mockStatement, true);
    rs.close();

    verify(mockStatement).close();
  }

  @Test
  void shouldNotCloseUnownedStatement() throws Exception {
    SnowflakeAsyncResultSetImpl rs =
        new SnowflakeAsyncResultSetImpl(queryId, mockConnection, mockStatement, false);
    rs.close();

    verify(mockStatement, never()).close();
  }

  @Test
  void shouldBeIdempotentOnDoubleClose() throws Exception {
    SnowflakeAsyncResultSetImpl rs = createAsyncResultSet();
    rs.close();
    rs.close();

    verify(mockStatement).removeClosedResultSet(rs);
  }

  @Test
  void shouldThrowOnOperationsAfterClose() throws Exception {
    ResultSet rs = createDecoratedAsyncResultSet();
    rs.close();

    assertThrows(SQLException.class, rs::next);
    assertThrows(SQLException.class, rs::getMetaData);
    assertThrows(SQLException.class, () -> rs.getString(1));
  }

  @Test
  void shouldAnswerIsWrapperForWithoutMaterializing() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertTrue(rs.isWrapperFor(SnowflakeAsyncResultSetImpl.class));
      assertTrue(rs.isWrapperFor(SnowflakeResultSet.class));
      assertTrue(rs.isWrapperFor(SnowflakeResultSetImpl.class));
      assertFalse(rs.isWrapperFor(String.class));

      verify(mockConnection, never()).createResultSetFromSfqid(any(), any());
    }
  }

  @Test
  void shouldReturnOwningStatement() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      // getStatement() now hands back the decorated boundary; the owning raw statement is reached
      // through unwrap().
      assertEquals(mockStatement, rs.getStatement().unwrap(SnowflakeStatementImpl.class));
    }
  }

  @Test
  void shouldThrowOnPositionMethodsAfterClose() throws Exception {
    ResultSet rs = createDecoratedAsyncResultSet();
    rs.close();

    assertThrows(SQLException.class, rs::isBeforeFirst);
    assertThrows(SQLException.class, rs::isAfterLast);
    assertThrows(SQLException.class, rs::isFirst);
    assertThrows(SQLException.class, rs::getRow);
  }

  @Test
  void shouldMaterializeOnceAndDelegateDataAndPositionReads() throws Exception {
    SnowflakeResultSetImpl mockDelegate = mock(SnowflakeResultSetImpl.class);
    when(mockConnection.getQueryStatus(queryId)).thenReturn(successStatus());
    when(mockConnection.createResultSetFromSfqid(eq(queryId), eq(mockStatement)))
        .thenReturn(mockDelegate);
    when(mockDelegate.next()).thenReturn(true, false);
    when(mockDelegate.getString(1)).thenReturn("alpha");
    when(mockDelegate.wasNull()).thenReturn(false);
    when(mockDelegate.isBeforeFirst()).thenReturn(false);
    when(mockDelegate.isFirst()).thenReturn(true);
    when(mockDelegate.isAfterLast()).thenReturn(false);
    when(mockDelegate.getRow()).thenReturn(1);

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertTrue(rs.next());
      assertEquals("alpha", rs.getString(1));
      assertFalse(rs.wasNull());
      assertFalse(rs.isBeforeFirst());
      assertTrue(rs.isFirst());
      assertFalse(rs.isAfterLast());
      assertEquals(1, rs.getRow());
      assertFalse(rs.next());

      verify(mockConnection, times(1)).createResultSetFromSfqid(queryId, mockStatement);
    }
  }

  @Test
  void shouldUnwrapSelfWithoutMaterializingAndDelegateUnwrapForInnerType() throws Exception {
    SnowflakeResultSetImpl mockDelegate = mock(SnowflakeResultSetImpl.class);
    when(mockConnection.getQueryStatus(queryId)).thenReturn(successStatus());
    when(mockConnection.createResultSetFromSfqid(eq(queryId), eq(mockStatement)))
        .thenReturn(mockDelegate);
    when(mockDelegate.unwrap(SnowflakeResultSetImpl.class)).thenReturn(mockDelegate);

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertSame(rs, rs.unwrap(SnowflakeResultSet.class));
      verify(mockConnection, never()).createResultSetFromSfqid(any(), any());

      assertSame(mockDelegate, rs.unwrap(SnowflakeResultSetImpl.class));
      verify(mockConnection, times(1)).createResultSetFromSfqid(queryId, mockStatement);
    }
  }

  @Test
  void shouldReturnAsyncMetaDataViewSuppressingSourceObjectNames() throws Exception {
    SnowflakeResultSetImpl mockDelegate = mock(SnowflakeResultSetImpl.class);
    SnowflakeResultSetMetaDataImpl syncMeta =
        SnowflakeResultSetMetaDataImpl.fromColumnSpec(
            "sync-qid",
            Collections.singletonList("COL1"),
            Collections.singletonList("VARCHAR"),
            Collections.singletonList(Types.VARCHAR));
    when(mockConnection.getQueryStatus(queryId)).thenReturn(successStatus());
    when(mockConnection.createResultSetFromSfqid(eq(queryId), eq(mockStatement)))
        .thenReturn(mockDelegate);
    when(mockDelegate.getMetaDataImpl()).thenReturn(syncMeta);

    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      ResultSetMetaData asyncMeta = rs.getMetaData();

      assertEquals(1, asyncMeta.getColumnCount());
      assertEquals("COL1", asyncMeta.getColumnName(1));
      assertEquals("", asyncMeta.getCatalogName(1));
      assertEquals("", asyncMeta.getSchemaName(1));
      assertEquals("", asyncMeta.getTableName(1));
      assertEquals("T", syncMeta.getTableName(1));
      assertEquals(queryId, asyncMeta.unwrap(SnowflakeResultSetMetaData.class).getQueryID());
    }
  }
}
