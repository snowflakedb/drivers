package net.snowflake.client.internal.api.implementation.telemetry;

import java.sql.SQLException;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.Set;
import lombok.Getter;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.api.implementation.exception.SFBatchUpdateException;
import net.snowflake.client.internal.api.implementation.exception.SFClientInfoException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.unicore.TransportException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Pre-classified category of a wrapper-caught error, sent to core as the {@code error_source} wire
 * string of {@code telemetrySendWrapperError}. The snake_case {@link #getWireValue()} strings are a
 * contract shared with the Python and ODBC front-ends, so they must stay stable and identical
 * across drivers; the raw error itself never crosses the wire.
 */
@Getter
@RequiredArgsConstructor
public enum ErrorSource {
  CONNECTIVITY("connectivity"),
  SERVER_ERROR("server_error"),
  DATA_CONVERSION("data_conversion"),
  CURSOR_STATE("cursor_state"),
  API_MISUSE("api_misuse"),
  CONFIG_PARSING("config_parsing"),
  INTERNAL_ERROR("internal_error"),
  UNSUPPORTED("unsupported"),
  UNKNOWN("unknown");

  /** The snake_case string sent on the wire (e.g. {@code "server_error"}). */
  private final String wireValue;

  /**
   * Classifies a wrapper-caught throwable into its {@code error_source} category.
   *
   * <p>An {@link SFSQLException} dispatches on its {@link ErrorCode} when it carries one, then on
   * the non-zero vendor code of a surfaced {@link SQLException}, then on a {@link CoreException}
   * anywhere in its cause chain, then on an {@link InterruptedException} in that chain. An {@link
   * SFBatchUpdateException}, {@link SFClientInfoException}, or {@link SnowflakeSQLException}
   * dispatches on its vendor code, and on a {@link CoreException} cause when that code is {@code
   * 0}. A {@link CoreException} with a payload dispatches on its {@link ErrorKind}. One with no
   * payload is {@link #CONNECTIVITY} when the cause is a {@link TransportException}, {@link
   * #UNKNOWN} when the cause is an {@link InterruptedException}, and {@link #INTERNAL_ERROR}
   * otherwise. A non-zero vendor code that is not one of our {@link ErrorCode}s is {@link
   * #SERVER_ERROR}. Anything else is {@link #INTERNAL_ERROR}.
   */
  public static ErrorSource of(Throwable t) {
    if (t instanceof CoreException) {
      return ofCore((CoreException) t);
    }
    if (t instanceof NotImplementedException || t instanceof SFSQLFeatureNotSupportedException) {
      return UNSUPPORTED;
    }
    if (t instanceof SFSQLException) {
      return ofSfSql((SFSQLException) t);
    }
    if (t instanceof SnowflakeSQLException) {
      return ofVendorCodeThenCore(((SnowflakeSQLException) t).getErrorCode(), t);
    }
    if (t instanceof SFBatchUpdateException) {
      return ofVendorCodeThenCore(((SFBatchUpdateException) t).getVendorCode(), t);
    }
    if (t instanceof SFClientInfoException) {
      return ofVendorCodeThenCore(((SFClientInfoException) t).getVendorCode(), t);
    }
    return INTERNAL_ERROR;
  }

  private static ErrorSource ofCore(CoreException exception) {
    if (exception.getError() != null) {
      return ofKind(exception.getError().getKind());
    }
    Throwable cause = exception.getCause();
    if (cause instanceof InterruptedException) {
      return UNKNOWN;
    }
    if (cause instanceof TransportException) {
      return CONNECTIVITY;
    }
    return INTERNAL_ERROR;
  }

  private static ErrorSource ofSfSql(SFSQLException exception) {
    if (exception.getErrorCode() != null) {
      return ofErrorCode(exception.getErrorCode());
    }
    SQLException surfaced = exception.getSurfaced();
    if (surfaced != null && surfaced.getErrorCode() != 0) {
      return ofVendorCode(surfaced.getErrorCode());
    }
    CoreException core = causeOfType(exception, CoreException.class);
    if (core != null) {
      return ofCore(core);
    }
    if (causeOfType(exception, InterruptedException.class) != null) {
      return UNKNOWN;
    }
    return INTERNAL_ERROR;
  }

  /** A vendor code of {@code 0} means the carrier did not copy one, so the cause decides. */
  private static ErrorSource ofVendorCodeThenCore(int vendorCode, Throwable throwable) {
    if (vendorCode != 0) {
      return ofVendorCode(vendorCode);
    }
    CoreException core = causeOfType(throwable, CoreException.class);
    if (core != null) {
      return ofCore(core);
    }
    return INTERNAL_ERROR;
  }

  private static <T extends Throwable> T causeOfType(Throwable throwable, Class<T> type) {
    Throwable cause = throwable.getCause();
    Set<Throwable> seen = Collections.newSetFromMap(new IdentityHashMap<Throwable, Boolean>());
    while (cause != null && seen.add(cause)) {
      if (type.isInstance(cause)) {
        return type.cast(cause);
      }
      cause = cause.getCause();
    }
    return null;
  }

  /** A non-zero vendor code outside {@link ErrorCode} is a code the server assigned. */
  private static ErrorSource ofVendorCode(int vendorCode) {
    if (vendorCode == 0) {
      return INTERNAL_ERROR;
    }
    for (ErrorCode code : ErrorCode.values()) {
      if (code.getMessageCode() == vendorCode) {
        return ofErrorCode(code);
      }
    }
    return SERVER_ERROR;
  }

  private static ErrorSource ofErrorCode(ErrorCode errorCode) {
    switch (errorCode) {
      case INVALID_VALUE_CONVERT:
        return DATA_CONVERSION;
      case INVALID_CONNECTION_STRING:
      case INVALID_PROXY_PROPERTIES:
        return CONFIG_PARSING;
      case CONNECTION_CLOSED:
      case RESULTSET_ALREADY_CLOSED:
        return CURSOR_STATE;
      case COLUMN_DOES_NOT_EXIST:
      case INVALID_PARAMETER_VALUE:
      case INVALID_PARAMETER_TYPE:
        return API_MISUSE;
      case FEATURE_UNSUPPORTED:
      case ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED:
      case COMPRESSION_TYPE_NOT_SUPPORTED:
        return UNSUPPORTED;
      case CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP:
      case FILE_NOT_FOUND:
        return SERVER_ERROR;
      case NETWORK_ERROR:
      case IO_ERROR:
      case FILE_OPERATION_UPLOAD_ERROR:
      case FILE_OPERATION_DOWNLOAD_ERROR:
        // These codes are the vendor-code form of ERROR_KIND_IO.
        return CONNECTIVITY;
      case INTERNAL_ERROR:
      default:
        return INTERNAL_ERROR;
    }
  }

  private static ErrorSource ofKind(ErrorKind kind) {
    switch (kind) {
      case ERROR_KIND_IO:
      case ERROR_KIND_TIMEOUT:
        return CONNECTIVITY;
      case ERROR_KIND_QUERY_FAILED:
      case ERROR_KIND_AUTHENTICATION_ERROR:
      case ERROR_KIND_LOGIN_ERROR:
      case ERROR_KIND_STAGE_BINDING:
      case ERROR_KIND_REMOTE_FILE_NOT_FOUND:
        return SERVER_ERROR;
      case ERROR_KIND_LOCAL_FILE_NOT_FOUND:
        // ErrorKindMapper surfaces this as ErrorCode.FILE_NOT_FOUND, the same vendor code as a
        // missing remote file.
        return SERVER_ERROR;
      case ERROR_KIND_INVALID_ARGUMENT:
      case ERROR_KIND_INVALID_PARAMETER_VALUE:
      case ERROR_KIND_MISSING_PARAMETER:
        return API_MISUSE;
      case ERROR_KIND_NOT_IMPLEMENTED:
      case ERROR_KIND_UNSUPPORTED_COMPRESSION:
        return UNSUPPORTED;
      case ERROR_KIND_INTERNAL_ERROR:
        return INTERNAL_ERROR;
      case ERROR_KIND_CANCELLED:
        // Caller cancellation is an application core error.
        return SERVER_ERROR;
      case ERROR_KIND_UNSPECIFIED:
      default:
        return UNKNOWN;
    }
  }
}
