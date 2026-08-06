package net.snowflake.client.internal.api.implementation.exception;

import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.function.Supplier;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.decorator.AbstractDecorator;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Translates the driver's internal runtime-exception model into the checked {@link SQLException}s
 * the public JDBC API must throw. Impl classes throw only runtime exceptions; the decorator layer
 * routes every call through {@link #call(Supplier)} / {@link #run(Runnable)} (or {@link
 * AbstractDecorator} when telemetry is also recorded), and this is the only place a {@link
 * SnowflakeSQLException} is created. Expressing the delegate call as a {@link Supplier}/{@link
 * Runnable} means the boundary only ever sees unchecked throwables, which is what lets the impl
 * side drop {@code throws SQLException}. See {@link #translate(Throwable)} for the mapping.
 */
public final class SqlExceptionMapper {

  private SqlExceptionMapper() {}

  /**
   * Runs {@code action}, translating any thrown runtime exception to a checked {@link
   * SQLException}.
   */
  public static <T> T call(Supplier<T> action) throws SQLException {
    try {
      return action.get();
    } catch (RuntimeException e) {
      throw translate(e);
    }
  }

  /** Void counterpart of {@link #call(Supplier)}. */
  public static void run(Runnable action) throws SQLException {
    try {
      action.run();
    } catch (RuntimeException e) {
      throw translate(e);
    }
  }

  /**
   * The catch ladder as one pure function, shared by both entry points and {@link
   * AbstractDecorator}'s telemetry hook. {@link SFException} maps via {@link
   * SnowflakeSQLException#fromSFException} (SQLState + vendor code, cause dropped for legacy
   * parity); anything else keeps its cause for diagnostics.
   */
  public static SQLException translate(Throwable t) {
    if (t instanceof SnowflakeSQLException) {
      return (SnowflakeSQLException) t;
    }
    if (t instanceof SQLException) {
      return (SQLException) t;
    }
    if (t instanceof SFException) {
      return SnowflakeSQLException.fromSFException((SFException) t);
    }
    if (t instanceof NotImplementedException) {
      return new SQLFeatureNotSupportedException(t.getMessage());
    }
    return new SnowflakeSQLException(t.getMessage(), t);
  }
}
