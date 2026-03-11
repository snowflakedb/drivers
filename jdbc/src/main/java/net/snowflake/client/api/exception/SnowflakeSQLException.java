package net.snowflake.client.api.exception;

import java.sql.SQLException;

public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  public SnowflakeSQLException(String message) {
    super(message);
  }

  public SnowflakeSQLException(String message, Throwable cause) {
    super(message, cause);
  }
}
