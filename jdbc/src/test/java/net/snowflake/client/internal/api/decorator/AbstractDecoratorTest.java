package net.snowflake.client.internal.api.decorator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Clob;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.Supplier;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.resultset.DecoratedSnowflakeAsyncResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.DecoratedSnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeAsyncResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.DecoratedSnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakeStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * Covers the {@link AbstractDecorator} telemetry seam and re-entrancy guard. The emitter is still
 * {@link Telemetry#NOOP} in production, so these tests drive a recording emitter to assert the hook
 * wiring that the real emitter will inherit unchanged.
 */
public class AbstractDecoratorTest {

  /** Records every api-usage / wrapper-error the decorator emits. */
  private static final class RecordingTelemetry implements Telemetry {
    final List<String> apiUsage = new ArrayList<>();
    final List<String> wrapperErrors = new ArrayList<>();

    @Override
    public void recordApiUsage(String apiMethod) {
      apiUsage.add(apiMethod);
    }

    @Override
    public void recordWrapperError(Throwable error) {
      wrapperErrors.add(error.getClass().getSimpleName());
    }
  }

  /** Minimal concrete decorator exposing the protected hooks for testing. */
  private static final class TestDecorator extends AbstractDecorator<String> {
    TestDecorator(String delegate, Telemetry telemetry) {
      super(delegate, telemetry);
    }

    <T> T instrumentedCall(String op, Supplier<T> action) throws SQLException {
      return call(op, action);
    }

    void instrumentedRun(String op, Runnable action) throws SQLException {
      run(op, action);
    }

    <T> T hotCall(Supplier<T> action) throws SQLException {
      return call(action);
    }

    void uninstrumentedRun(Runnable action) throws SQLException {
      run(action);
    }

    SQLException hotTranslate(RuntimeException error) {
      return translateHot(error);
    }
  }

  private RecordingTelemetry telemetry;
  private TestDecorator decorator;

  @BeforeEach
  public void setUp() {
    telemetry = new RecordingTelemetry();
    decorator = new TestDecorator("d", telemetry);
  }

  @Test
  public void shouldExposeDelegateForUnwrapping() {
    assertSame("d", decorator.getDelegate());
  }

  @Test
  public void shouldRecordApiUsageForInstrumentedCall() throws SQLException {
    assertEquals("ok", decorator.instrumentedCall("Statement.execute", () -> "ok"));
    assertEquals(Arrays.asList("Statement.execute"), telemetry.apiUsage);
  }

  @Test
  public void shouldNotRecordApiUsageForHotAccessor() throws SQLException {
    decorator.hotCall(() -> "value");
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldNotRecordApiUsageForResultSetMetaDataAccessors() throws SQLException {
    SnowflakeResultSetMetaDataImpl meta = mock(SnowflakeResultSetMetaDataImpl.class);
    when(meta.getColumnCount()).thenReturn(9);
    when(meta.getColumnType(1)).thenReturn(Types.BIGINT);
    ResultSetMetaData decorated = new DecoratedSnowflakeResultSetMetaDataImpl(meta, telemetry);
    assertEquals(9, decorated.getColumnCount());
    assertEquals(Types.BIGINT, decorated.getColumnType(1));
    decorated.getColumnLabel(1);
    decorated.getColumnName(1);
    decorated.getPrecision(1);
    decorated.getScale(1);
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldNotRecordApiUsageForResultSetGetMetaData() throws SQLException {
    SnowflakeResultSetImpl resultSet = mock(SnowflakeResultSetImpl.class);
    when(resultSet.getMetaData()).thenReturn(mock(ResultSetMetaData.class));
    when(resultSet.isClosed()).thenReturn(false);
    ResultSet decorated = Decorators.resultSet(resultSet, telemetry);
    decorated.getMetaData();
    decorated.isClosed();
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldNotRecordApiUsageForResultSetGetClobAndVectorArray() throws SQLException {
    SnowflakeResultSetImpl resultSet = mock(SnowflakeResultSetImpl.class);
    when(resultSet.getClob(1)).thenReturn(mock(Clob.class));
    when(resultSet.getClob("c")).thenReturn(mock(Clob.class));
    when(resultSet.getArray(1, Integer.class)).thenReturn(new Integer[] {1});
    ResultSet decorated = Decorators.resultSet(resultSet, telemetry);
    decorated.getClob(1);
    decorated.getClob("c");
    ((SnowflakeResultSet) decorated).getArray(1, Integer.class);
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldNotRecordApiUsageForPreparedStatementBinds() throws SQLException {
    SnowflakePreparedStatementImpl prepared = mock(SnowflakePreparedStatementImpl.class);
    PreparedStatement decorated = (PreparedStatement) Decorators.statement(prepared, telemetry);
    decorated.setInt(1, 7);
    decorated.setString(2, "x");
    decorated.addBatch();
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldRecordApiUsageForPreparedStatementExecuteQuery() throws SQLException {
    SnowflakePreparedStatementImpl prepared = mock(SnowflakePreparedStatementImpl.class);
    when(prepared.executeQuery()).thenReturn(mock(ResultSet.class));
    PreparedStatement decorated = (PreparedStatement) Decorators.statement(prepared, telemetry);
    decorated.executeQuery();
    assertEquals(Arrays.asList("PreparedStatement.executeQuery"), telemetry.apiUsage);
  }

  @Test
  public void shouldRecordApiUsageOnlyForOutermostCall() throws SQLException {
    TestDecorator inner = new TestDecorator("inner", telemetry);
    TestDecorator outer = new TestDecorator("outer", telemetry);
    outer.instrumentedCall(
        "Connection.createStatement",
        () -> {
          // Nested decorated boundary on the same thread must not double-count. Bridge its checked
          // SQLException, as any real re-entrant caller would have to.
          try {
            return inner.instrumentedCall("Statement.execute", () -> "inner-result");
          } catch (SQLException e) {
            throw new IllegalStateException(e);
          }
        });
    assertEquals(Arrays.asList("Connection.createStatement"), telemetry.apiUsage);
  }

  @Test
  public void shouldTranslateRuntimeExceptionFromDelegate() {
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                decorator.instrumentedCall(
                    "Statement.execute",
                    () -> {
                      throw SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED);
                    }));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), thrown.getErrorCode());
    // The decorator forwards the raw caught throwable to the telemetry layer for classification.
    assertEquals(Arrays.asList("SFSQLException"), telemetry.wrapperErrors);
  }

  @Test
  public void shouldRecordWrapperErrorOnlyForOutermostCall() throws SQLException {
    TestDecorator inner = new TestDecorator("inner", telemetry);
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                decorator.instrumentedCall(
                    "Connection.createStatement",
                    () -> {
                      // A nested boundary that fails must not also record — only the outermost
                      // does.
                      try {
                        return inner.instrumentedCall(
                            "Statement.execute",
                            () -> {
                              throw SFSQLException.fromErrorCode(ErrorCode.INVALID_PARAMETER_VALUE);
                            });
                      } catch (SQLException e) {
                        throw SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED);
                      }
                    }));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), thrown.getErrorCode());
    assertEquals(Arrays.asList("SFSQLException"), telemetry.wrapperErrors);
  }

  @Test
  public void shouldReleaseGuardAfterExceptionSoNextOutermostCallRecords() throws SQLException {
    assertThrows(
        SQLException.class,
        () ->
            decorator.instrumentedRun(
                "Statement.execute",
                () -> {
                  throw SFSQLException.fromErrorCode(ErrorCode.INVALID_PARAMETER_VALUE);
                }));
    // A leaked guard would make this second call look nested and record nothing.
    decorator.instrumentedCall("Statement.getResultSet", () -> "ok");
    assertEquals(Arrays.asList("Statement.execute", "Statement.getResultSet"), telemetry.apiUsage);
  }

  @Test
  public void shouldNotLeaveGuardSetAcrossCalls() throws SQLException {
    decorator.instrumentedCall("Statement.execute", () -> "a");
    decorator.instrumentedCall("Statement.getResultSet", () -> "b");
    assertEquals(2, telemetry.apiUsage.size());
  }

  @Test
  public void shouldTranslateHotExceptionAndRecordOnlyWhenOutermost() throws SQLException {
    SQLException direct =
        decorator.hotTranslate(SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), direct.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), direct.getErrorCode());
    assertEquals(Arrays.asList("SFSQLException"), telemetry.wrapperErrors);

    decorator.instrumentedCall(
        "ResultSet.getString",
        () -> {
          SQLException nested =
              decorator.hotTranslate(
                  SFSQLException.fromErrorCode(ErrorCode.INVALID_PARAMETER_VALUE));
          assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), nested.getSQLState());
          return "ok";
        });
    assertEquals(Arrays.asList("SFSQLException"), telemetry.wrapperErrors);
  }

  @Test
  public void shouldCoalesceNullTelemetryToNoopAndSupportUnwrap() throws SQLException {
    TestDecorator nullTelemetryDecorator = new TestDecorator("delegate-val", null);
    nullTelemetryDecorator.uninstrumentedRun(() -> {});
    nullTelemetryDecorator.instrumentedRun("Statement.close", () -> {});
    SQLException translated =
        nullTelemetryDecorator.hotTranslate(
            SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), translated.getErrorCode());

    assertTrue(decorator.isWrapperFor(TestDecorator.class));
    assertTrue(decorator.isWrapperFor(CharSequence.class));
    assertFalse(decorator.isWrapperFor(Integer.class));
    assertSame(decorator, decorator.unwrap(TestDecorator.class));
    assertEquals("d", decorator.unwrap(CharSequence.class));

    SnowflakeSQLException unwrapFailure =
        assertThrows(SnowflakeSQLException.class, () -> decorator.unwrap(Integer.class));
    assertEquals("Cannot unwrap to java.lang.Integer", unwrapFailure.getMessage());
    assertNull(unwrapFailure.getSQLState());
    assertEquals(0, unwrapFailure.getErrorCode());
    assertTrue(telemetry.apiUsage.isEmpty());
  }

  @Test
  public void shouldDecorateStatementsResultSetsAndTelemetryIdempotently() {
    assertNull(Decorators.statement(null, telemetry));
    assertNull(Decorators.resultSet(null, telemetry));
    assertNull(Decorators.connection(null, telemetry));
    assertSame(Telemetry.NOOP, Decorators.telemetryOf((SnowflakeStatementImpl) null));

    SnowflakeCallableStatementImpl callable = mock(SnowflakeCallableStatementImpl.class);
    Statement decoratedCallable = Decorators.statement(callable, telemetry);
    assertInstanceOf(DecoratedSnowflakeCallableStatementImpl.class, decoratedCallable);
    assertSame(decoratedCallable, Decorators.statement(decoratedCallable, telemetry));

    SnowflakePreparedStatementImpl prepared = mock(SnowflakePreparedStatementImpl.class);
    assertInstanceOf(
        DecoratedSnowflakePreparedStatementImpl.class, Decorators.statement(prepared, telemetry));

    SnowflakeStatementImpl plain = mock(SnowflakeStatementImpl.class);
    assertInstanceOf(DecoratedSnowflakeStatementImpl.class, Decorators.statement(plain, telemetry));
    assertSame(Telemetry.NOOP, Decorators.telemetryOf(plain));

    InternalSnowflakeConnection conn = mock(InternalSnowflakeConnection.class);
    when(conn.getTelemetry()).thenReturn(telemetry);
    when(plain.getConnectionInternal()).thenReturn(conn);
    assertSame(telemetry, Decorators.telemetryOf(plain));

    SnowflakeAsyncResultSetImpl asyncRs = mock(SnowflakeAsyncResultSetImpl.class);
    ResultSet decoratedAsync = Decorators.resultSet(asyncRs, telemetry);
    assertInstanceOf(DecoratedSnowflakeAsyncResultSetImpl.class, decoratedAsync);
    assertSame(decoratedAsync, Decorators.resultSet(decoratedAsync, telemetry));

    SnowflakeResultSetImpl syncRs = mock(SnowflakeResultSetImpl.class);
    assertInstanceOf(
        DecoratedSnowflakeResultSetImpl.class, Decorators.resultSet(syncRs, telemetry));
  }
}
