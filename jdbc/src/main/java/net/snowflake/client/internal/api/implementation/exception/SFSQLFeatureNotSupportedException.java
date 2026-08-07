package net.snowflake.client.internal.api.implementation.exception;

import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;

/**
 * Carrier for a feature unsupported <em>by design</em>, mapping to a checked {@link
 * SQLFeatureNotSupportedException}. Distinct from {@link
 * net.snowflake.client.internal.util.NotImplementedException} (a "not yet" gap): both surface the
 * same JDBC edge type, but are kept separate so a genuine "never" is not confused with a temporary
 * gap. SQLState / vendor code are re-derived in {@link #toSQLException()} from the cause when the
 * original exception is available.
 */
public class SFSQLFeatureNotSupportedException extends DriverRuntimeException {
  private static final long serialVersionUID = 1L;

  public SFSQLFeatureNotSupportedException(String message) {
    super(message);
  }

  public SFSQLFeatureNotSupportedException(SQLFeatureNotSupportedException e) {
    super(e.getMessage(), e);
  }

  @Override
  public SQLException toSQLException() {
    Throwable cause = getCause();
    if (cause instanceof SQLFeatureNotSupportedException) {
      SQLFeatureNotSupportedException original = (SQLFeatureNotSupportedException) cause;
      if (original.getSQLState() != null) {
        return new SQLFeatureNotSupportedException(
            getMessage(), original.getSQLState(), original.getErrorCode());
      }
    }
    return new SQLFeatureNotSupportedException(getMessage());
  }
}
