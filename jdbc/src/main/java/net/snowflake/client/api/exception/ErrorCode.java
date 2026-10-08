package net.snowflake.client.api.exception;

import lombok.AllArgsConstructor;
import lombok.Getter;

@Getter
@AllArgsConstructor
public enum ErrorCode {
  INTERNAL_ERROR(200001, null, null),
  INVALID_VALUE_CONVERT(
      200038, null, "Cannot convert value in the driver from type:{0} to type:{1}, value={2}."),
  COLUMN_DOES_NOT_EXIST(200032, "22000", null),
  CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP(
      200041,
      "01000",
      "Connection property value {0} is invalid. Value specified by user: {1}, returned by server:"
          + " {2}."),
  RESULTSET_ALREADY_CLOSED(200037, "0A000", null),
  ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED(200023, "0A000", null),
  FEATURE_UNSUPPORTED(200035, "0A000", null),
  INVALID_PARAMETER_VALUE(200047, "22023", null),
  INVALID_PARAMETER_TYPE(200033, "22023", "Invalid parameter value type: {0}, expected type: {1}."),
  INVALID_CONNECTION_STRING(200059, "08000", "Connection string is invalid. Unable to parse."),
  INVALID_PROXY_PROPERTIES(200051, "08000", null),
  CONNECTION_CLOSED(200052, "08003", null),
  COMPRESSION_TYPE_NOT_SUPPORTED(200004, "0A000", null),
  FILE_NOT_FOUND(200008, "22000", null),
  NETWORK_ERROR(200015, "58030", "JDBC driver encountered communication error. Message: {0}."),
  IO_ERROR(200016, "58000", "JDBC driver encountered IO error. Message: {0}."),
  FILE_OPERATION_UPLOAD_ERROR(
      200066, "XX000", "JDBC driver file operation error while performing stage upload."),
  FILE_OPERATION_DOWNLOAD_ERROR(
      200067, "XX000", "JDBC driver file operation error while performing stage download.");

  private final int messageCode;
  private final String sqlState;
  private final String messageTemplate;
}
