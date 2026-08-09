package net.snowflake.client.internal.util;

import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;

/**
 * Carrier for a method that is <em>not yet</em> implemented but is expected to be, mapping to a
 * checked {@link SQLFeatureNotSupportedException}. Distinct from {@link
 * net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException} (a
 * by-design "never"): same JDBC edge type, kept separate so a temporary gap is not confused with a
 * permanent one.
 */
public class NotImplementedException extends DriverRuntimeException {
  private static final long serialVersionUID = 1L;

  public NotImplementedException() {
    super((String) null);
  }

  public NotImplementedException(String message) {
    super(message);
  }

  @Override
  public SQLException toSQLException() {
    return new SQLFeatureNotSupportedException(getMessage());
  }
}
