package net.snowflake.client.api.exception;

import java.sql.SQLException;

public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  private SnowflakeSQLException(String message, ErrorCode errorCode) {
    super(message, errorCode.getSqlState(), errorCode.getMessageCode());
  }

  public SnowflakeSQLException(String message) {
    super(message);
  }

  public SnowflakeSQLException(String message, Throwable cause) {
    super(message, cause);
  }

  public static SnowflakeSQLException unsupportedStatementTypeInExecutionApi(String sqlPreview) {
    return new SnowflakeSQLException(
        "Statement '" + sqlPreview + "' cannot be executed using current API.",
        ErrorCode.UNSUPPORTED_STATEMENT_TYPE_IN_EXECUTION_API);
  }
}
