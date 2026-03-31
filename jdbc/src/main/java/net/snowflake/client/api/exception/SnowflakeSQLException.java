package net.snowflake.client.api.exception;

import java.sql.SQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1;

public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  public SnowflakeSQLException(String message) {
    super(message);
  }

  public SnowflakeSQLException(String message, Throwable cause) {
    super(message, cause);
  }

  public SnowflakeSQLException(DatabaseDriverV1.DriverException error, Throwable cause) {
    super(
        error.hasRootCause() ? error.getRootCause() : error.getMessage(),
        error.hasSqlState() ? error.getSqlState() : null,
        error.hasVendorCode() ? error.getVendorCode() : 0,
        cause);
  }

  public static SnowflakeSQLException fromServiceException(
      String fallbackMessage, DatabaseDriverService.ServiceException exception) {
    DatabaseDriverV1.DriverException error = exception.error;
    if (error == null) {
      return new SnowflakeSQLException(fallbackMessage, exception);
    }
    return new SnowflakeSQLException(error, exception);
  }
}
