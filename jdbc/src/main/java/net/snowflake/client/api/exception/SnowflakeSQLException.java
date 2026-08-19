package net.snowflake.client.api.exception;

import java.sql.SQLException;
import lombok.Getter;

@Getter
public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  /**
   * @return the Snowflake query ID associated with the failed query, or {@code null} when the
   *     server did not surface one.
   */
  private final String queryId;

  public SnowflakeSQLException(String message) {
    super(message);
    this.queryId = null;
  }

  public SnowflakeSQLException(String message, Throwable cause) {
    this(message, cause, null);
  }

  /** As {@link #SnowflakeSQLException(String, Throwable)} but also carrying a query id. */
  public SnowflakeSQLException(String message, Throwable cause, String queryId) {
    super(message, cause);
    this.queryId = queryId;
  }

  public SnowflakeSQLException(ErrorCode errorCode, String message) {
    this(errorCode, message, null);
  }

  /** As {@link #SnowflakeSQLException(ErrorCode, String)} but also carrying a query id. */
  public SnowflakeSQLException(ErrorCode errorCode, String message, String queryId) {
    super(message, errorCode.getSqlState(), errorCode.getMessageCode());
    this.queryId = queryId;
  }

  /** As {@link SQLException#SQLException(String, String, int, Throwable)} plus query id. */
  public SnowflakeSQLException(
      String reason, String sqlState, int vendorCode, Throwable cause, String queryId) {
    super(reason, sqlState, vendorCode, cause);
    this.queryId = queryId;
  }
}
