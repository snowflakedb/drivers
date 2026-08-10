package net.snowflake.client.api.exception;

import java.sql.SQLException;
import lombok.Getter;
import net.snowflake.client.internal.unicore.ServiceException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1;

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

  public SnowflakeSQLException(DatabaseDriverV1.DriverException error, Throwable cause) {
    super(
        error.hasRootCause() ? error.getRootCause() : error.getMessage(),
        error.hasSqlState() ? error.getSqlState() : null,
        error.hasVendorCode() ? error.getVendorCode() : 0,
        cause);
    this.queryId = error.hasQueryId() ? error.getQueryId() : null;
  }

  public static SnowflakeSQLException fromServiceException(ServiceException exception) {
    if (exception.error instanceof DatabaseDriverV1.DriverException) {
      return new SnowflakeSQLException(
          (DatabaseDriverV1.DriverException) exception.error, exception);
    }
    return new SnowflakeSQLException(exception.getMessage(), exception);
  }
}
