package net.snowflake.client.internal.api.implementation.exception;

import java.util.Collections;
import java.util.EnumMap;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;

/**
 * Maps a core {@link ErrorKind} onto a legacy JDBC {@link ErrorCode} when the payload does not
 * already carry a vendor code or SQLSTATE.
 */
final class ErrorKindMapper {

  private static final Map<ErrorKind, ErrorCode> KIND_TO_ERROR_CODE;

  static {
    EnumMap<ErrorKind, ErrorCode> mapped = new EnumMap<ErrorKind, ErrorCode>(ErrorKind.class);
    mapped.put(ErrorKind.ERROR_KIND_LOCAL_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND);
    mapped.put(ErrorKind.ERROR_KIND_REMOTE_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND);
    mapped.put(
        ErrorKind.ERROR_KIND_UNSUPPORTED_COMPRESSION, ErrorCode.COMPRESSION_TYPE_NOT_SUPPORTED);
    KIND_TO_ERROR_CODE = Collections.unmodifiableMap(mapped);
  }

  private ErrorKindMapper() {}

  static int vendorCode(DriverException error) {
    if (error.hasVendorCode()) {
      return error.getVendorCode();
    }
    ErrorCode fallback = toErrorCode(error.getKind());
    return fallback != null ? fallback.getMessageCode() : 0;
  }

  static String sqlState(DriverException error) {
    if (error.hasSqlState()) {
      return error.getSqlState();
    }
    ErrorCode fallback = toErrorCode(error.getKind());
    return fallback != null ? fallback.getSqlState() : null;
  }

  static ErrorCode toErrorCode(ErrorKind kind) {
    return KIND_TO_ERROR_CODE.get(kind);
  }
}
