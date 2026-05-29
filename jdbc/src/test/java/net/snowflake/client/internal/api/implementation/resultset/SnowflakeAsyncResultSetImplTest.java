package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.SQLException;
import java.util.UUID;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
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
    InternalResultSet mockDelegate = mock(InternalResultSet.class);
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
    SnowflakeAsyncResultSetImpl rs = createAsyncResultSet();
    rs.close();

    assertThrows(SQLException.class, rs::next);
    assertThrows(SQLException.class, rs::getMetaData);
    assertThrows(SQLException.class, () -> rs.getString(1));
  }

  @Test
  void shouldReturnOwningStatement() throws Exception {
    try (SnowflakeAsyncResultSetImpl rs = createAsyncResultSet()) {
      assertEquals(mockStatement, rs.getStatement());
    }
  }

  @Test
  void shouldThrowOnPositionMethodsAfterClose() throws Exception {
    SnowflakeAsyncResultSetImpl rs = createAsyncResultSet();
    rs.close();

    assertThrows(SQLException.class, rs::isBeforeFirst);
    assertThrows(SQLException.class, rs::isAfterLast);
    assertThrows(SQLException.class, rs::isFirst);
    assertThrows(SQLException.class, rs::getRow);
  }
}
