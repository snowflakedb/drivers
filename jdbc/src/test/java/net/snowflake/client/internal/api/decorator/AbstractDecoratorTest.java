package net.snowflake.client.internal.api.decorator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.Supplier;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
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
    public void recordWrapperError(String exceptionType, String errorSource) {
      wrapperErrors.add(exceptionType + ":" + errorSource);
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
                      throw new SFException(ErrorCode.CONNECTION_CLOSED);
                    }));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), thrown.getErrorCode());
    // The wrapper-error signal is wired by the real-emitter PR, not PR 0.
    assertTrue(telemetry.wrapperErrors.isEmpty());
  }

  @Test
  public void shouldReleaseGuardAfterExceptionSoNextOutermostCallRecords() throws SQLException {
    assertThrows(
        SQLException.class,
        () ->
            decorator.instrumentedRun(
                "Statement.execute",
                () -> {
                  throw new SFException(ErrorCode.INVALID_PARAMETER_VALUE);
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
}
