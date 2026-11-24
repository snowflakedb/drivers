package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.telemetry.ErrorSource;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.IoClass;
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

  static Stream<Arguments> ioClassFallbacks() {
    return Stream.of(
        Arguments.of(
            IoClass.IO_CLASS_FILE_UPLOAD, ErrorCode.FILE_OPERATION_UPLOAD_ERROR, 200066, "XX000"),
        Arguments.of(
            IoClass.IO_CLASS_FILE_DOWNLOAD,
            ErrorCode.FILE_OPERATION_DOWNLOAD_ERROR,
            200067,
            "XX000"),
        Arguments.of(IoClass.IO_CLASS_NETWORK, ErrorCode.NETWORK_ERROR, 200015, "58030"),
        Arguments.of(IoClass.IO_CLASS_NO_SPACE_LEFT, ErrorCode.IO_ERROR, 200016, "58000"),
        Arguments.of(IoClass.IO_CLASS_GENERIC, ErrorCode.IO_ERROR, 200016, "58000"));
  }

  @ParameterizedTest(name = "shouldMap {0} to {1}")
  @MethodSource("errorKindFallbacks")
  public void shouldMapErrorKindToLegacyErrorCode(ErrorKind kind, ErrorCode expected) {
    DriverException payload =
        DriverException.newBuilder().setMessage("client-side failure").setKind(kind).build();
    assertEquals(expected, ErrorKindMapper.toErrorCode(payload));

    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(expected.getMessageCode(), thrown.getErrorCode());
    assertEquals(expected.getSqlState(), thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
    assertEquals(
        ErrorSource.of(SFSQLException.fromErrorCode(expected)),
        ErrorSource.of(carrier),
        kind.name());
  }

  @ParameterizedTest(name = "shouldMap {0} to {1}")
  @MethodSource("ioClassFallbacks")
  public void shouldMapIoClassToLegacyErrorCode(
      IoClass ioClass, ErrorCode expected, int legacyVendorCode, String legacySqlState) {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("client-side failure")
            .setKind(ErrorKind.ERROR_KIND_IO)
            .setIoClass(ioClass)
            .build();
    assertEquals(expected, ErrorKindMapper.toErrorCode(payload));

    CoreException carrier = new CoreException(payload, null);
    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(legacyVendorCode, thrown.getErrorCode());
    assertEquals(legacySqlState, thrown.getSQLState());
    assertSame(carrier, thrown.getCause());
  }

  @Test
  public void shouldReportConnectionNotEstablishedSqlStateForLoginError() {
    DriverException withCredentialSqlState =
        DriverException.newBuilder()
            .setMessage("Login error: Incorrect username or password was specified., code: 390100")
            .setKind(ErrorKind.ERROR_KIND_LOGIN_ERROR)
            .setVendorCode(390100)
            .setSqlState("28000")
            .build();
    SnowflakeSQLException credentialRejection =
        surface(new CoreException(withCredentialSqlState, null));
    assertEquals(390100, credentialRejection.getErrorCode());
    assertEquals(ErrorKindMapper.LOGIN_REJECTION_SQL_STATE, credentialRejection.getSQLState());

    DriverException withoutSqlState =
        DriverException.newBuilder()
            .setMessage(
                "Login error: The user you were trying to authenticate as differs from the user"
                    + " currently logged in at the IDP., code: 390191")
            .setKind(ErrorKind.ERROR_KIND_LOGIN_ERROR)
            .setVendorCode(390191)
            .build();
    SnowflakeSQLException idpMismatch = surface(new CoreException(withoutSqlState, null));
    assertEquals(390191, idpMismatch.getErrorCode());
    assertEquals(ErrorKindMapper.LOGIN_REJECTION_SQL_STATE, idpMismatch.getSQLState());
  }

  @Test
  public void shouldKeepCoreSqlStateForAuthenticationError() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("token expired")
            .setKind(ErrorKind.ERROR_KIND_AUTHENTICATION_ERROR)
            .setVendorCode(390112)
            .setSqlState("08001")
            .build();
    SnowflakeSQLException thrown = surface(new CoreException(payload, null));
    assertEquals(390112, thrown.getErrorCode());
    assertEquals("08001", thrown.getSQLState());
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
  public void shouldDetectStageBindingDisabledFromErrorKind() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("SYSTEM$BIND stage is disabled")
            .setKind(ErrorKind.ERROR_KIND_STAGE_BINDING)
            .build();
    CoreException carrier = new CoreException(payload, null);

    assertTrue(carrier.isStageBindingDisabled());
  }

  @Test
  public void shouldNotTreatOtherErrorKindsAsStageBindingDisabled() {
    CoreException messageOnly = new CoreException("transport failure");
    assertFalse(messageOnly.isStageBindingDisabled());

    DriverException payload =
        DriverException.newBuilder()
            .setMessage("cancelled")
            .setKind(ErrorKind.ERROR_KIND_CANCELLED)
            .build();
    assertFalse(new CoreException(payload, null).isStageBindingDisabled());
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
    assertNull(ErrorKindMapper.toErrorCode(payload));
  }

  @Test
  public void shouldLeaveVendorCodeUnsetWhenIoHasNoClass() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("File transfers have been disabled.")
            .setKind(ErrorKind.ERROR_KIND_IO)
            .build();
    CoreException carrier = new CoreException(payload, null);

    SnowflakeSQLException thrown = surface(carrier);

    assertEquals(0, thrown.getErrorCode());
    assertNull(thrown.getSQLState());
    assertNull(ErrorKindMapper.toErrorCode(payload));
  }

  private static SnowflakeSQLException surface(CoreException carrier) {
    return (SnowflakeSQLException) carrier.toSQLException();
  }
}
