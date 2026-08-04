package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.util.NotImplementedException;
import org.junit.jupiter.api.Test;

/** Branch coverage for {@link SqlExceptionMapper}, the JDBC exception-translation boundary. */
public class SqlExceptionMapperTest {

  @Test
  public void shouldReturnValueOnSuccess() throws SQLException {
    assertEquals("ok", SqlExceptionMapper.call(() -> "ok"));
  }

  @Test
  public void shouldExecuteActionOnSuccess() throws SQLException {
    AtomicBoolean ran = new AtomicBoolean(false);
    SqlExceptionMapper.run(() -> ran.set(true));
    assertTrue(ran.get());
  }

  @Test
  public void shouldTranslateSfExceptionThroughCallPreservingCodeAndState() {
    SFException sf = new SFException(ErrorCode.CONNECTION_CLOSED);
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                SqlExceptionMapper.call(
                    () -> {
                      throw sf;
                    }));
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), thrown.getErrorCode());
    assertEquals(sf.getMessage(), thrown.getMessage());
    // Cause is intentionally dropped to match legacy parity rendering.
    assertNull(thrown.getCause());
  }

  @Test
  public void shouldRenderFormattedMessageForSfExceptionWithTemplate() {
    SFException sf = new SFException(ErrorCode.INVALID_VALUE_CONVERT, "VARIANT", "INT", "abc");
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                SqlExceptionMapper.call(
                    () -> {
                      throw sf;
                    }));
    assertEquals(sf.getMessage(), thrown.getMessage());
    // INVALID_VALUE_CONVERT has a null SQLState in the enum.
    assertEquals(ErrorCode.INVALID_VALUE_CONVERT.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.INVALID_VALUE_CONVERT.getMessageCode(), thrown.getErrorCode());
  }

  @Test
  public void shouldTranslateNotImplementedThroughCallPreservingMessage() {
    SQLException thrown =
        assertThrows(
            SQLFeatureNotSupportedException.class,
            () ->
                SqlExceptionMapper.call(
                    () -> {
                      throw new NotImplementedException("no such feature");
                    }));
    assertEquals("no such feature", thrown.getMessage());
  }

  @Test
  public void shouldWrapArbitraryRuntimeExceptionThroughCallKeepingCause() {
    IllegalArgumentException cause = new IllegalArgumentException("timeout is less than 0");
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                SqlExceptionMapper.call(
                    () -> {
                      throw cause;
                    }));
    assertEquals("timeout is less than 0", thrown.getMessage());
    assertSame(cause, thrown.getCause());
  }

  @Test
  public void shouldTranslateExceptionsInRun() {
    SQLException thrown =
        assertThrows(
            SnowflakeSQLException.class,
            () ->
                SqlExceptionMapper.run(
                    () -> {
                      throw new SFException(ErrorCode.INVALID_PARAMETER_VALUE);
                    }));
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), thrown.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), thrown.getErrorCode());
  }

  // translate() is exercised directly for the checked-exception arms: a Supplier/Runnable lambda
  // cannot throw a checked exception, so those arms are only reachable through the pure function.

  @Test
  public void shouldReturnSnowflakeSqlExceptionUnchanged() {
    SnowflakeSQLException original = new SnowflakeSQLException("boom");
    assertSame(original, SqlExceptionMapper.translate(original));
  }

  @Test
  public void shouldReturnPlainSqlExceptionUnchanged() {
    SQLException original = new SQLException("control flow", "08003", 42);
    assertSame(original, SqlExceptionMapper.translate(original));
  }

  @Test
  public void shouldWrapCheckedExceptionKeepingCause() {
    IOException cause = new IOException("disk gone");
    SQLException thrown = SqlExceptionMapper.translate(cause);
    assertEquals("disk gone", thrown.getMessage());
    assertSame(cause, thrown.getCause());
  }
}
