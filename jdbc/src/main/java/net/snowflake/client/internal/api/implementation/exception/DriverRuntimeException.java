package net.snowflake.client.internal.api.implementation.exception;

import java.sql.SQLException;

/**
 * Base for every unchecked carrier the impl tier throws in place of a checked {@link SQLException},
 * so impl code need not declare {@code throws SQLException}. {@link SqlExceptionMapper#translate}
 * converts a carrier to its checked counterpart at exactly one boundary (the decorator).
 *
 * <p>Each subclass owns its edge mapping via the abstract {@link #toSQLException()} — a polymorphic
 * dispatch, not an {@code instanceof} ladder, so a new carrier is a compile error until its mapping
 * is supplied. Byte-exact construction still funnels through {@link
 * net.snowflake.client.api.exception.SnowflakeSQLException}'s own constructors, keeping
 * legacy-parity rendering in one place.
 */
public abstract class DriverRuntimeException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  protected DriverRuntimeException(String message) {
    super(message);
  }

  protected DriverRuntimeException(String message, Throwable cause) {
    super(message, cause);
  }

  /** The checked {@link SQLException} this carrier becomes at the boundary. */
  public abstract SQLException toSQLException();
}
