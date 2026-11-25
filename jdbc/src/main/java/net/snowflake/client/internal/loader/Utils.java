package net.snowflake.client.internal.loader;

import net.snowflake.client.internal.api.implementation.exception.DriverRuntimeException;

/** Utils class for Loader API */
final class Utils {

  private Utils() {}

  /**
   * Find the root cause of the exception.
   *
   * <p>The walk stops before an internal {@link DriverRuntimeException} carrier. A {@code
   * SnowflakeSQLException} raised by this driver keeps its carrier as the cause, so without the
   * stop the result would be the internal carrier instead of the {@code SQLException} that holds
   * the SQLSTATE and vendor code.
   *
   * @param e throwable object
   * @return the throwable cause
   */
  static Throwable getCause(Throwable e) {
    Throwable cause;
    Throwable result = e;

    while (null != (cause = result.getCause())
        && result != cause
        && !(cause instanceof DriverRuntimeException)) {
      result = cause;
    }
    return result;
  }
}
