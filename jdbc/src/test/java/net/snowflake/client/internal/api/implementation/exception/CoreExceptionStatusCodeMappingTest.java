package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatusCode;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/** Covers {@link StatusCodeMapper} and its use from {@link CoreException#toSQLException()}. */
public class CoreExceptionStatusCodeMappingTest {

  static Stream<Arguments> statusCodeFallbacks() {
    return Stream.of(
        Arguments.of(StatusCode.STATUS_CODE_LOCAL_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND),
        Arguments.of(StatusCode.STATUS_CODE_REMOTE_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND),
        Arguments.of(
            StatusCode.STATUS_CODE_UNSUPPORTED_COMPRESSION,
            ErrorCode.COMPRESSION_TYPE_NOT_SUPPORTED));
  }

  @ParameterizedTest(name = "shouldMap {0} to {1}")
  @MethodSource("statusCodeFallbacks")
  public void shouldMapStatusCodeToLegacyErrorCode(StatusCode statusCode, ErrorCode expected) {
    assertEquals(expected, StatusCodeMapper.toErrorCode(statusCode));

    DriverException payload =
        DriverException.newBuilder()
            .setMessage("client-side failure")
            .setStatusCode(statusCode)
            .build();
    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(expected.getMessageCode(), thrown.getErrorCode());
    assertEquals(expected.getSqlState(), thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
  }

  @Test
  public void shouldPreferServerVendorCodeAndSqlStateWhenPresent() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("server failure with status")
            .setStatusCode(StatusCode.STATUS_CODE_LOCAL_FILE_NOT_FOUND)
            .setVendorCode(123456)
            .setSqlState("42S02")
            .build();
    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(123456, thrown.getErrorCode());
    assertEquals("42S02", thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
  }

  @Test
  public void shouldLeaveVendorCodeAndSqlStateUnsetForUnrelatedStatusCodes() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("cancelled")
            .setStatusCode(StatusCode.STATUS_CODE_CANCELLED)
            .build();
    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(0, thrown.getErrorCode());
    assertNull(thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
    assertNull(StatusCodeMapper.toErrorCode(StatusCode.STATUS_CODE_CANCELLED));
  }

  private static SnowflakeSQLException surface(CoreException carrier) {
    return (SnowflakeSQLException) carrier.toSQLException();
  }
}
