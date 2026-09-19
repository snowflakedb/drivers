package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;

import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class AutoConnectionExceptionMapperTest {

  @Test
  void shouldMapMissingAutoProfileToInvalidParameter() {
    CoreException missingProfileError = missingProfileErrorFor("missing");

    RuntimeException mapped =
        AutoConnectionExceptionMapper.remapMissingProfile(missingProfileError, true);
    SnowflakeSQLException exception =
        assertInstanceOf(
            SnowflakeSQLException.class,
            assertInstanceOf(SnowflakeSQLExceptionCarrier.class, mapped).toSQLException());

    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(), exception.getSQLState());
    assertEquals(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(), exception.getErrorCode());
    assertSame(missingProfileError, exception.getCause());
    assertEquals(
        "The Connection missing not found in connections.toml file.", exception.getMessage());
    assertEquals("query-id", exception.getQueryId());
  }

  @Test
  void shouldPreserveSuppressedCleanupFailuresWhenMappingAMissingProfile() {
    CoreException missingProfileError = missingProfileErrorFor("missing");
    AssertionError cleanupFailure = new AssertionError("cleanup failed");
    missingProfileError.addSuppressed(cleanupFailure);

    RuntimeException mapped =
        AutoConnectionExceptionMapper.remapMissingProfile(missingProfileError, true);
    SnowflakeSQLException exception =
        assertInstanceOf(
            SnowflakeSQLException.class,
            assertInstanceOf(SnowflakeSQLExceptionCarrier.class, mapped).toSQLException());

    assertEquals(1, exception.getSuppressed().length);
    assertSame(cleanupFailure, exception.getSuppressed()[0]);
  }

  @Test
  void shouldPreserveCoreExceptionsWithNoPayload() {
    CoreException otherFailure = new CoreException("login failed");

    assertSame(
        otherFailure, AutoConnectionExceptionMapper.remapMissingProfile(otherFailure, false));
    assertSame(otherFailure, AutoConnectionExceptionMapper.remapMissingProfile(otherFailure, true));
  }

  @Test
  void shouldPreserveMissingProfileOnANonAutoUrl() {
    CoreException missingOnNonAutoUrl = missingProfileErrorFor("missing");

    assertSame(
        missingOnNonAutoUrl,
        AutoConnectionExceptionMapper.remapMissingProfile(missingOnNonAutoUrl, false));
  }

  @Test
  void shouldPreserveNonMissingParameterKinds() {
    CoreException differentKind =
        new CoreException(
            DriverException.newBuilder()
                .setKind(ErrorKind.ERROR_KIND_INVALID_PARAMETER_VALUE)
                .setMessage("bad option")
                .setParameter("connection: missing")
                .build(),
            null);

    assertSame(
        differentKind, AutoConnectionExceptionMapper.remapMissingProfile(differentKind, true));
  }

  @Test
  void shouldPreserveFailuresThatAreNotCoreExceptions() {
    RuntimeException unrelated = new IllegalStateException("boom");

    assertSame(unrelated, AutoConnectionExceptionMapper.remapMissingProfile(unrelated, true));
    assertSame(unrelated, AutoConnectionExceptionMapper.remapMissingProfile(unrelated, false));
  }

  @Test
  void shouldPreserveMissingParameterFailuresThatAreNotAboutTheProfile() {
    CoreException missingUser =
        new CoreException(
            DriverException.newBuilder()
                .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
                .setMessage("Missing required parameter 'user'")
                .setParameter("user")
                .build(),
            null);

    assertSame(missingUser, AutoConnectionExceptionMapper.remapMissingProfile(missingUser, true));
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "connection",
        "connection:readOnly",
        "connection_name",
        "user for connection: readOnly"
      })
  void shouldPreserveParametersThatOnlyResembleTheMissingProfilePrefix(String parameter) {
    CoreException nearMiss =
        new CoreException(
            DriverException.newBuilder()
                .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
                .setMessage("Missing required parameter")
                .setParameter(parameter)
                .build(),
            null);

    assertSame(nearMiss, AutoConnectionExceptionMapper.remapMissingProfile(nearMiss, true));
  }

  @Test
  void shouldPreserveMissingParameterFailuresThatNameNoParameter() {
    CoreException missingParameter =
        new CoreException(
            DriverException.newBuilder()
                .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
                .setMessage("Missing required parameter")
                .build(),
            null);

    assertSame(
        missingParameter,
        AutoConnectionExceptionMapper.remapMissingProfile(missingParameter, true));
  }

  private static CoreException missingProfileErrorFor(String name) {
    return new CoreException(
        DriverException.newBuilder()
            .setKind(ErrorKind.ERROR_KIND_MISSING_PARAMETER)
            .setMessage("Configuration error: Connection '" + name + "' not found in config files")
            .setRootCause("Connection '" + name + "' not found in config files")
            .setParameter("connection: " + name)
            .setQueryId("query-id")
            .build(),
        null);
  }
}
