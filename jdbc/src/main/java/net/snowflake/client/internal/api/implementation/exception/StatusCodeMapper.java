package net.snowflake.client.internal.api.implementation.exception;

import java.util.Collections;
import java.util.EnumMap;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatusCode;

/**
 * Maps a core {@link StatusCode} onto a legacy JDBC {@link ErrorCode} when the payload does not
 * already carry a vendor code or SQLSTATE.
 */
final class StatusCodeMapper {

  private static final Map<StatusCode, ErrorCode> STATUS_TO_ERROR_CODE;

  static {
    EnumMap<StatusCode, ErrorCode> mapped = new EnumMap<StatusCode, ErrorCode>(StatusCode.class);
    mapped.put(StatusCode.STATUS_CODE_LOCAL_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND);
    mapped.put(StatusCode.STATUS_CODE_REMOTE_FILE_NOT_FOUND, ErrorCode.FILE_NOT_FOUND);
    mapped.put(
        StatusCode.STATUS_CODE_UNSUPPORTED_COMPRESSION, ErrorCode.COMPRESSION_TYPE_NOT_SUPPORTED);
    STATUS_TO_ERROR_CODE = Collections.unmodifiableMap(mapped);
  }

  private StatusCodeMapper() {}

  static int vendorCode(DriverException error) {
    if (error.hasVendorCode()) {
      return error.getVendorCode();
    }
    ErrorCode fallback = toErrorCode(error.getStatusCode());
    return fallback != null ? fallback.getMessageCode() : 0;
  }

  static String sqlState(DriverException error) {
    if (error.hasSqlState()) {
      return error.getSqlState();
    }
    ErrorCode fallback = toErrorCode(error.getStatusCode());
    return fallback != null ? fallback.getSqlState() : null;
  }

  static ErrorCode toErrorCode(StatusCode statusCode) {
    return STATUS_TO_ERROR_CODE.get(statusCode);
  }
}
