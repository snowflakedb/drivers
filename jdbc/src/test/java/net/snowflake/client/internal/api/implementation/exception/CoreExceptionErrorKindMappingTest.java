package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/** Covers {@link ErrorKindMapper} and its use from {@link CoreException#toSQLException()}. */
public class CoreExceptionErrorKindMappingTest {

  static Stream<Arguments> errorKindFallbacks() {
    return Stream.of(
        Arguments.of(ErrorKind.ERROR_KIND_LOCAL_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND),
        Arguments.of(ErrorKind.ERROR_KIND_REMOTE_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND),
        Arguments.of(
            ErrorKind.ERROR_KIND_UNSUPPORTED_COMPRESSION,
            ErrorCode.COMPRESSION_TYPE_NOT_SUPPORTED));
  }

  @ParameterizedTest(name = "shouldMap {0} to {1}")
  @MethodSource("errorKindFallbacks")
  public void shouldMapErrorKindToLegacyErrorCode(ErrorKind kind, ErrorCode expected) {
    assertEquals(expected, ErrorKindMapper.toErrorCode(kind));

    DriverException payload =
        DriverException.newBuilder().setMessage("client-side failure").setKind(kind).build();
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
            .setKind(ErrorKind.ERROR_KIND_LOCAL_FILE_NOT_FOUND)
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
  public void shouldLeaveVendorCodeAndSqlStateUnsetForUnrelatedErrorKinds() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("cancelled")
            .setKind(ErrorKind.ERROR_KIND_CANCELLED)
            .build();
    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(0, thrown.getErrorCode());
    assertNull(thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
    assertNull(ErrorKindMapper.toErrorCode(ErrorKind.ERROR_KIND_CANCELLED));
  }

  private static SnowflakeSQLException surface(CoreException carrier) {
    return (SnowflakeSQLException) carrier.toSQLException();
  }
}
