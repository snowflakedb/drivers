package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import org.junit.jupiter.api.Test;

/**
 * Covers the query-id carrying contract added to the {@link SnowflakeSQLExceptionCarrier} family
 * and the byte-identical rendering preserved when {@code SFException} was folded into {@link
 * SFSQLException}.
 */
public class ExceptionCarrierQueryIdTest {

  @Test
  public void shouldFlowAttachedQueryIdIntoSurfacedSnowflakeSqlExceptionOnMessagePath() {
    SnowflakeSQLExceptionCarrier carrier = new SFSQLException("boom").withQueryId("01a-b2c");

    SQLException surfaced = carrier.toSQLException();

    SnowflakeSQLException sf = assertInstanceOf(SnowflakeSQLException.class, surfaced);
    assertEquals("01a-b2c", sf.getQueryId());
    assertEquals("boom", sf.getMessage());
  }

  @Test
  public void shouldFlowAttachedQueryIdIntoSurfacedSnowflakeSqlExceptionOnErrorCodePath() {
    SnowflakeSQLExceptionCarrier carrier =
        SFSQLException.fromErrorCode(ErrorCode.CONNECTION_CLOSED).withQueryId("01a-b2c");

    SnowflakeSQLException sf =
        assertInstanceOf(SnowflakeSQLException.class, carrier.toSQLException());

    assertEquals("01a-b2c", sf.getQueryId());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getSqlState(), sf.getSQLState());
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), sf.getErrorCode());
    // The errorCode path drops the cause for legacy parity.
    assertNull(sf.getCause());
  }

  @Test
  public void shouldLeaveQueryIdNullWhenNoneAttached() {
    SFSQLException carrier = new SFSQLException("boom");

    SnowflakeSQLException sf =
        assertInstanceOf(SnowflakeSQLException.class, carrier.toSQLException());

    assertNull(sf.getQueryId());
  }

  @Test
  public void shouldRenderNullTemplateErrorCodeAsNameWhenNoArgs() {
    // INTERNAL_ERROR has a null MessageFormat template — the no-arg fallback is the code name.
    SFSQLException carrier = SFSQLException.fromErrorCode(ErrorCode.INTERNAL_ERROR);

    assertEquals(String.valueOf(ErrorCode.INTERNAL_ERROR), carrier.getMessage());
  }

  @Test
  public void shouldRenderNullTemplateErrorCodeWithArgsAppended() {
    SFSQLException carrier =
        SFSQLException.fromErrorCode(ErrorCode.INTERNAL_ERROR, "Missing column type for column 3");

    assertEquals("INTERNAL_ERROR: [Missing column type for column 3]", carrier.getMessage());
  }

  @Test
  public void shouldPreferAttachedQueryIdOverCorePayloadQueryId() {
    DriverException payload =
        DriverException.newBuilder().setMessage("server failure").setQueryId("payload-id").build();
    CoreException carrier =
        (CoreException) new CoreException(payload, null).withQueryId("attached-id");

    assertEquals("attached-id", carrier.getQueryId());
    SnowflakeSQLException sf =
        assertInstanceOf(SnowflakeSQLException.class, carrier.toSQLException());
    assertEquals("attached-id", sf.getQueryId());
  }

  @Test
  public void shouldFallBackToCorePayloadQueryIdWhenNoneAttached() {
    DriverException payload =
        DriverException.newBuilder().setMessage("server failure").setQueryId("payload-id").build();
    CoreException carrier = new CoreException(payload, null);

    assertEquals("payload-id", carrier.getQueryId());
    SnowflakeSQLException sf =
        assertInstanceOf(SnowflakeSQLException.class, carrier.toSQLException());
    assertEquals("payload-id", sf.getQueryId());
  }

  @Test
  public void shouldReDeriveSqlStateAndVendorCodeFromCauseForFeatureNotSupported() {
    SQLFeatureNotSupportedException cause =
        new SQLFeatureNotSupportedException("no can do", "0A000", 200035);
    SFSQLFeatureNotSupportedException carrier = new SFSQLFeatureNotSupportedException(cause);

    SQLException surfaced = carrier.toSQLException();

    SQLFeatureNotSupportedException fnse =
        assertInstanceOf(SQLFeatureNotSupportedException.class, surfaced);
    assertEquals("no can do", fnse.getMessage());
    assertEquals("0A000", fnse.getSQLState());
    assertEquals(200035, fnse.getErrorCode());
  }

  @Test
  public void shouldSurfaceMessageOnlyFeatureNotSupportedWithoutSqlState() {
    SFSQLFeatureNotSupportedException carrier =
        new SFSQLFeatureNotSupportedException("unsupported feature");

    SQLFeatureNotSupportedException fnse =
        assertInstanceOf(SQLFeatureNotSupportedException.class, carrier.toSQLException());
    assertEquals("unsupported feature", fnse.getMessage());
    assertNull(fnse.getSQLState());
    assertEquals(0, fnse.getErrorCode());
  }

  @Test
  public void shouldReturnSameCarrierInstanceFromWithQueryId() {
    SFSQLException carrier = new SFSQLException("boom");

    assertSame(carrier, carrier.withQueryId("01a-b2c"));
  }
}
