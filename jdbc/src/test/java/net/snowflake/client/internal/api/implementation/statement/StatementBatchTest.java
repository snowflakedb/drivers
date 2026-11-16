package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.sql.BatchUpdateException;
import java.sql.Statement;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;
import org.junit.jupiter.api.Test;

class StatementBatchTest {

  @Test
  void shouldReportSuccessNoInfoForStatementsWithoutUpdateCount() {
    SnowflakeStatementImpl stmt = mock(SnowflakeStatementImpl.class);
    when(stmt.executeLargeUpdateWithBindings("INSERT INTO t VALUES (1)", null)).thenReturn(3L);
    when(stmt.executeLargeUpdateWithBindings("CREATE TABLE t2 (x INT)", null))
        .thenReturn(StatementTypeClassifier.NO_UPDATE_COUNT);

    StatementBatch batch = new StatementBatch();
    batch.add("INSERT INTO t VALUES (1)");
    batch.add("CREATE TABLE t2 (x INT)");

    assertArrayEquals(new long[] {3L, Statement.SUCCESS_NO_INFO}, batch.executeAll(stmt));
    assertArrayEquals(new int[] {3, Statement.SUCCESS_NO_INFO}, batch.executeAllAsInt(stmt));
  }

  @Test
  void shouldSurfaceFirstFailureAsCauseWhenSeveralStatementsFail() {
    SnowflakeStatementImpl stmt = mock(SnowflakeStatementImpl.class);
    CoreException firstError = new CoreException("row 2 failed");
    when(stmt.executeLargeUpdateWithBindings("SQL_OK", null)).thenReturn(5L);
    when(stmt.executeLargeUpdateWithBindings("SQL_FAIL_1", null)).thenThrow(firstError);
    when(stmt.executeLargeUpdateWithBindings("SQL_FAIL_2", null))
        .thenThrow(new CoreException("row 3 failed"));

    StatementBatch batch = new StatementBatch();
    batch.add("SQL_OK");
    batch.add("SQL_FAIL_1");
    batch.add("SQL_FAIL_2");

    SFBatchUpdateException thrown =
        assertThrows(SFBatchUpdateException.class, () -> batch.executeAll(stmt));

    assertSame(firstError, thrown.getCause());
    assertArrayEquals(
        new long[] {5L, Statement.EXECUTE_FAILED, Statement.EXECUTE_FAILED},
        ((BatchUpdateException) thrown.toSQLException()).getLargeUpdateCounts());
    verify(stmt, times(3)).recordBatchQueryId();
    verify(stmt).finalizeBatch(thrown);
  }
}
