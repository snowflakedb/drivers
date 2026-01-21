package net.snowflake.client.jdbc;

import java.sql.SQLException;

public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  public SnowflakeSQLException(String message) {
    super(message);
  }
}
