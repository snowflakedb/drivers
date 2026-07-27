package net.snowflake.client.api.exception;

import java.sql.SQLException;
import lombok.Getter;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
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
    super(message, cause);
    this.queryId = null;
  }

  public SnowflakeSQLException(ErrorCode errorCode, String message) {
    super(message, errorCode.getSqlState(), errorCode.getMessageCode());
    this.queryId = null;
  }

  public SnowflakeSQLException(DatabaseDriverV1.DriverException error, Throwable cause) {
    super(
        error.hasRootCause() ? error.getRootCause() : error.getMessage(),
        error.hasSqlState() ? error.getSqlState() : null,
        error.hasVendorCode() ? error.getVendorCode() : 0,
        cause);
    this.queryId = error.hasQueryId() ? error.getQueryId() : null;
  }

  public static SnowflakeSQLException fromServiceException(
      DatabaseDriverService.ServiceException exception) {
    DatabaseDriverV1.DriverException error = exception.error;
    if (error == null) {
      return new SnowflakeSQLException(exception.getMessage(), exception);
    }
    return new SnowflakeSQLException(error, exception);
  }

  /**
   * Canonical {@code SFException -> SnowflakeSQLException} conversion. The cause is intentionally
   * dropped: the {@code (ErrorCode, message)} constructor already carries the SQLState and vendor
   * code, and legacy parity rendering depends on the absence of a cause chain.
   */
  public static SnowflakeSQLException fromSFException(SFException e) {
    return new SnowflakeSQLException(e.getErrorCode(), e.getMessage());
  }
}
