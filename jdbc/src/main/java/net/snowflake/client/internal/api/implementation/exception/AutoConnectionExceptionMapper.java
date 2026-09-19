package net.snowflake.client.internal.api.implementation.exception;

import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;

/** Applies JDBC-specific error compatibility to failures raised while opening an auto URL. */
public final class AutoConnectionExceptionMapper {

  /**
   * sf_core reports an absent auto-config profile as {@code MISSING_PARAMETER} carrying {@code
   * "connection: <name>"} in {@code parameter}. That prefix is the only discriminant that separates
   * it from the other {@code MISSING_PARAMETER} failures (an absent {@code user}, {@code token},
   * ...), which keep their own error identity.
   *
   * <p>The prefix is built in sf_core and matched here, with no shared constant and no compile-time
   * link between the two. {@code DriverException} does carry a typed {@code validation_code}, but
   * no member of it denotes this error and {@code ConnectionNotFound} leaves the field unset.
   *
   * <p>TODO(SNOW-3887958): carry a discriminant across the boundary — a {@code ValidationCode}
   * member or an {@code ErrorKind} for a missing profile — so this stops depending on message
   * shape. {@code connection_not_found_prefixes_parameter} in sf_core locks the prefix today.
   */
  private static final String MISSING_PROFILE_PARAMETER_PREFIX = "connection: ";

  private AutoConnectionExceptionMapper() {}

  public static RuntimeException remapMissingProfile(
      RuntimeException exception, boolean autoConnectionUrl) {
    if (!(exception instanceof CoreException) || !autoConnectionUrl) {
      return exception;
    }

    CoreException coreException = (CoreException) exception;
    if (!isMissingProfile(coreException.getError())) {
      return exception;
    }

    SnowflakeSQLException mapped =
        new SnowflakeSQLException(
            missingProfileMessage(coreException.getError().getParameter()),
            ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
            ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode(),
            coreException,
            coreException.getQueryId());
    for (Throwable suppressed : coreException.getSuppressed()) {
      mapped.addSuppressed(suppressed);
    }
    return SFSQLException.surfacing(mapped);
  }

  private static boolean isMissingProfile(DriverException error) {
    return error != null
        && error.getKind() == ErrorKind.ERROR_KIND_MISSING_PARAMETER
        && error.getParameter().startsWith(MISSING_PROFILE_PARAMETER_PREFIX);
  }

  /**
   * The reference JDBC driver reports this wording from {@code SFConnectionConfigParser}. Keep the
   * public JDBC message aligned with that driver; SQLState and vendor code stay {@link
   * ErrorCode#INVALID_PARAMETER_VALUE} so clients can classify the failure.
   */
  private static String missingProfileMessage(String parameter) {
    String profileName = parameter.substring(MISSING_PROFILE_PARAMETER_PREFIX.length());
    return "The Connection " + profileName + " not found in connections.toml file.";
  }
}
