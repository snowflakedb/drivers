package net.snowflake.client.internal.api.implementation.telemetry;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.sql.ClientInfoStatus;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Collections;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;
import net.snowflake.client.internal.api.implementation.exception.SFClientInfoException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.unicore.TransportException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import net.snowflake.client.internal.util.NotImplementedException;
import org.junit.jupiter.api.Test;

/** Covers {@link ErrorSource#of(Throwable)}, the wrapper-error classification. */
public class ErrorSourceTest {

  private static final Map<ErrorCode, ErrorSource> BY_ERROR_CODE = errorCodes();
  private static final Map<ErrorKind, ErrorSource> BY_ERROR_KIND = errorKinds();

  @Test
  public void shouldClassifyEachErrorCode() {
    assertEquals(EnumSet.allOf(ErrorCode.class), BY_ERROR_CODE.keySet());
    for (Map.Entry<ErrorCode, ErrorSource> entry : BY_ERROR_CODE.entrySet()) {
      assertEquals(
          entry.getValue(),
          ErrorSource.of(SFSQLException.fromErrorCode(entry.getKey(), "a", "b", "c")),
          entry.getKey().name());
    }
  }

  @Test
  public void shouldClassifyEachErrorKind() {
    Set<ErrorKind> classifiable =
        EnumSet.allOf(ErrorKind.class).stream()
            .filter(kind -> !"UNRECOGNIZED".equals(kind.name()))
            .collect(Collectors.toCollection(() -> EnumSet.noneOf(ErrorKind.class)));
    assertEquals(classifiable, BY_ERROR_KIND.keySet());
    for (Map.Entry<ErrorKind, ErrorSource> entry : BY_ERROR_KIND.entrySet()) {
      DriverException payload =
          DriverException.newBuilder().setMessage("failure").setKind(entry.getKey()).build();
      assertEquals(
          entry.getValue(),
          ErrorSource.of(new CoreException(payload, null)),
          entry.getKey().name());
    }
  }

  @Test
  public void shouldClassifyTransportCoreFailureAsConnectivity() {
    assertEquals(
        ErrorSource.CONNECTIVITY,
        ErrorSource.of(
            new CoreException(
                "Driver communication error: reset", new TransportException("reset"))));
  }

  @Test
  public void shouldClassifyPayloadLessCoreFailureWithoutTransportCauseAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new CoreException("connection reset")));
    assertEquals(
        ErrorSource.INTERNAL_ERROR,
        ErrorSource.of(new CoreException("Driver error: boom", new IOException("boom"))));
  }

  @Test
  public void shouldClassifyInterruptedCoreFailureAsUnknown() {
    assertEquals(
        ErrorSource.UNKNOWN,
        ErrorSource.of(new CoreException("interrupted", new InterruptedException("stop"))));
  }

  @Test
  public void shouldClassifyFeatureNotSupportedAsUnsupported() {
    assertEquals(
        ErrorSource.UNSUPPORTED,
        ErrorSource.of(
            new SFSQLFeatureNotSupportedException("getBestRowIdentifier not supported")));
  }

  @Test
  public void shouldClassifyNotImplementedAsUnsupported() {
    assertEquals(ErrorSource.UNSUPPORTED, ErrorSource.of(new NotImplementedException("nope")));
  }

  @Test
  public void shouldClassifyMessageOnlySfExceptionAsInternalError() {
    assertEquals(
        ErrorSource.INTERNAL_ERROR, ErrorSource.of(new SFSQLException("Statement is closed")));
  }

  @Test
  public void shouldClassifyMessageOnlySnowflakeSqlExceptionAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new SnowflakeSQLException("boom")));
  }

  @Test
  public void shouldClassifyForeignVendorCodeAsServerError() {
    assertEquals(
        ErrorSource.SERVER_ERROR,
        ErrorSource.of(new SnowflakeSQLException("syntax error", "42000", 1003, null, null)));
  }

  @Test
  public void shouldClassifyRemappedRemoteFileByItsCoreKind() {
    DriverException payload =
        DriverException.newBuilder()
            .setMessage("missing")
            .setKind(ErrorKind.ERROR_KIND_REMOTE_FILE_NOT_FOUND)
            .build();
    CoreException core = new CoreException(payload, null);

    assertEquals(
        ErrorSource.of(core),
        ErrorSource.of(SFSQLException.remoteFileNotFound("stage/file", core)));
  }

  @Test
  public void shouldClassifyInterruptedSfExceptionAsUnknown() {
    assertEquals(
        ErrorSource.UNKNOWN,
        ErrorSource.of(
            new SFSQLException(
                "Interrupted while waiting for async query to complete",
                new InterruptedException("stop"))));
  }

  @Test
  public void shouldPreferSurfacedVendorCodeOverADifferentCoreKind() {
    CoreException core = coreOf(ErrorKind.ERROR_KIND_IO);
    SnowflakeSQLException surfaced =
        new SnowflakeSQLException(
            "remapped",
            ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
            ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(),
            core,
            null);

    assertEquals(ErrorSource.API_MISUSE, ErrorSource.of(SFSQLException.surfacing(surfaced)));
  }

  @Test
  public void shouldClassifyCoreCauseNestedPastTheDirectCause() {
    CoreException core = coreOf(ErrorKind.ERROR_KIND_IO);
    IOException wrapped = new IOException("wrapper", new RuntimeException("mid", core));

    assertEquals(
        ErrorSource.CONNECTIVITY, ErrorSource.of(new SFSQLException("failed to read", wrapped)));
  }

  @Test
  public void shouldClassifyKindOnlyBatchFailureByItsCoreCause() {
    CoreException core = coreOf(ErrorKind.ERROR_KIND_QUERY_FAILED);

    assertEquals(ErrorSource.SERVER_ERROR, ErrorSource.of(batchFailure(0, null, core)));
  }

  @Test
  public void shouldClassifyBatchVendorCodeBeforeItsCoreCause() {
    CoreException core = coreOf(ErrorKind.ERROR_KIND_IO);

    assertEquals(
        ErrorSource.CURSOR_STATE,
        ErrorSource.of(
            batchFailure(
                ErrorCode.CONNECTION_CLOSED.getMessageCode(),
                ErrorCode.CONNECTION_CLOSED.getSqlState(),
                core)));
  }

  @Test
  public void shouldClassifyClosedConnectionClientInfoAsCursorState() {
    assertEquals(
        ErrorSource.CURSOR_STATE,
        ErrorSource.of(
            clientInfo(
                ErrorCode.CONNECTION_CLOSED.getMessageCode(),
                ErrorCode.CONNECTION_CLOSED.getSqlState())));
  }

  @Test
  public void shouldClassifyUnknownClientInfoPropertyAsApiMisuse() {
    assertEquals(
        ErrorSource.API_MISUSE,
        ErrorSource.of(
            clientInfo(
                ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(),
                ErrorCode.INVALID_PARAMETER_VALUE.getSqlState())));
  }

  @Test
  public void shouldClassifyMessageOnlySfExceptionByCoreCause() {
    DriverException payload =
        DriverException.newBuilder().setMessage("reset").setKind(ErrorKind.ERROR_KIND_IO).build();
    CoreException core = new CoreException(payload, null);

    assertEquals(
        ErrorSource.CONNECTIVITY,
        ErrorSource.of(new SFSQLException("Failed to read input stream: reset", core)));
  }

  @Test
  public void shouldClassifySnowflakeSqlExceptionByVendorCode() {
    assertEquals(
        ErrorSource.CONFIG_PARSING,
        ErrorSource.of(
            new SnowflakeSQLException(ErrorCode.INVALID_CONNECTION_STRING, "bad connection")));
  }

  @Test
  public void shouldClassifyCheckedSqlExceptionAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new SQLException("control flow")));
  }

  @Test
  public void shouldClassifyArbitraryThrowableAsInternalError() {
    assertEquals(ErrorSource.INTERNAL_ERROR, ErrorSource.of(new IOException("disk gone")));
  }

  private static CoreException coreOf(ErrorKind kind) {
    DriverException payload =
        DriverException.newBuilder().setMessage("failure").setKind(kind).build();
    return new CoreException(payload, null);
  }

  private static SFBatchUpdateException batchFailure(
      int vendorCode, String sqlState, CoreException cause) {
    return new SFBatchUpdateException(
        cause.getLocalizedMessage(),
        sqlState,
        vendorCode,
        new int[] {Statement.EXECUTE_FAILED},
        cause);
  }

  private static SFClientInfoException clientInfo(int vendorCode, String sqlState) {
    return new SFClientInfoException(
        "client info",
        sqlState,
        vendorCode,
        Collections.singletonMap("name", ClientInfoStatus.REASON_UNKNOWN_PROPERTY));
  }

  private static Map<ErrorCode, ErrorSource> errorCodes() {
    EnumMap<ErrorCode, ErrorSource> mapped = new EnumMap<ErrorCode, ErrorSource>(ErrorCode.class);
    mapped.put(ErrorCode.INVALID_VALUE_CONVERT, ErrorSource.DATA_CONVERSION);
    mapped.put(ErrorCode.INVALID_CONNECTION_STRING, ErrorSource.CONFIG_PARSING);
    mapped.put(ErrorCode.INVALID_PROXY_PROPERTIES, ErrorSource.CONFIG_PARSING);
    mapped.put(ErrorCode.CONNECTION_CLOSED, ErrorSource.CURSOR_STATE);
    mapped.put(ErrorCode.RESULTSET_ALREADY_CLOSED, ErrorSource.CURSOR_STATE);
    mapped.put(ErrorCode.COLUMN_DOES_NOT_EXIST, ErrorSource.API_MISUSE);
    mapped.put(ErrorCode.INVALID_PARAMETER_VALUE, ErrorSource.API_MISUSE);
    mapped.put(ErrorCode.INVALID_PARAMETER_TYPE, ErrorSource.API_MISUSE);
    mapped.put(ErrorCode.FEATURE_UNSUPPORTED, ErrorSource.UNSUPPORTED);
    mapped.put(ErrorCode.ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED, ErrorSource.UNSUPPORTED);
    mapped.put(ErrorCode.COMPRESSION_TYPE_NOT_SUPPORTED, ErrorSource.UNSUPPORTED);
    mapped.put(ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorCode.INTERNAL_ERROR, ErrorSource.INTERNAL_ERROR);
    mapped.put(ErrorCode.FILE_NOT_FOUND, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorCode.NETWORK_ERROR, ErrorSource.CONNECTIVITY);
    mapped.put(ErrorCode.IO_ERROR, ErrorSource.CONNECTIVITY);
    mapped.put(ErrorCode.FILE_OPERATION_UPLOAD_ERROR, ErrorSource.CONNECTIVITY);
    mapped.put(ErrorCode.FILE_OPERATION_DOWNLOAD_ERROR, ErrorSource.CONNECTIVITY);
    return mapped;
  }

  private static Map<ErrorKind, ErrorSource> errorKinds() {
    EnumMap<ErrorKind, ErrorSource> mapped = new EnumMap<ErrorKind, ErrorSource>(ErrorKind.class);
    mapped.put(ErrorKind.ERROR_KIND_IO, ErrorSource.CONNECTIVITY);
    mapped.put(ErrorKind.ERROR_KIND_TIMEOUT, ErrorSource.CONNECTIVITY);
    mapped.put(ErrorKind.ERROR_KIND_QUERY_FAILED, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_AUTHENTICATION_ERROR, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_LOGIN_ERROR, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_STAGE_BINDING, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_REMOTE_FILE_NOT_FOUND, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_LOCAL_FILE_NOT_FOUND, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_INVALID_ARGUMENT, ErrorSource.API_MISUSE);
    mapped.put(ErrorKind.ERROR_KIND_INVALID_PARAMETER_VALUE, ErrorSource.API_MISUSE);
    mapped.put(ErrorKind.ERROR_KIND_MISSING_PARAMETER, ErrorSource.API_MISUSE);
    mapped.put(ErrorKind.ERROR_KIND_NOT_IMPLEMENTED, ErrorSource.UNSUPPORTED);
    mapped.put(ErrorKind.ERROR_KIND_UNSUPPORTED_COMPRESSION, ErrorSource.UNSUPPORTED);
    mapped.put(ErrorKind.ERROR_KIND_INTERNAL_ERROR, ErrorSource.INTERNAL_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_CANCELLED, ErrorSource.SERVER_ERROR);
    mapped.put(ErrorKind.ERROR_KIND_UNSPECIFIED, ErrorSource.UNKNOWN);
    return mapped;
  }
}
