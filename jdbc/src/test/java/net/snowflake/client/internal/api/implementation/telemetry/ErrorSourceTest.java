package net.snowflake.client.internal.api.implementation.telemetry;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.sql.SQLException;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.util.NotImplementedException;
import org.junit.jupiter.api.Test;

/** Covers {@link ErrorSource#of(Throwable)}, the wrapper-error classification. */
public class ErrorSourceTest {

  @Test
  public void shouldClassifySfExceptionAsServerError() {
    assertEquals(
        ErrorSource.SERVER_ERROR,
        ErrorSource.of(SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED)));
  }

  @Test
  public void shouldClassifySnowflakeSqlExceptionAsServerError() {
    assertEquals(ErrorSource.SERVER_ERROR, ErrorSource.of(new SnowflakeSQLException("boom")));
  }

  @Test
  public void shouldClassifyNotImplementedAsUnsupported() {
    assertEquals(ErrorSource.UNSUPPORTED, ErrorSource.of(new NotImplementedException("nope")));
  }

  @Test
  public void shouldClassifyCheckedSqlExceptionAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new SQLException("control flow")));
  }

  @Test
  public void shouldClassifyArbitraryThrowableAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new IOException("disk gone")));
  }
}
