package net.snowflake.client.internal.api.implementation.exception;

import java.sql.SQLException;
import java.text.MessageFormat;
import java.util.Arrays;
import lombok.Getter;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;

/**
 * Carrier for an exact {@link SnowflakeSQLException}, preserving the {@code (errorCode, message,
 * cause)} arguments so the mapper can rebuild it byte-identically.
 *
 * <p>The message is either verbatim (constructors) or rendered through the {@link ErrorCode} {@code
 * MessageFormat} template via {@link #fromErrorCode(ErrorCode, Object...)} (the path formerly on
 * {@code SFException}); a {@code null} template (e.g. {@link ErrorCode#INTERNAL_ERROR}) falls back
 * to {@code "CODE: [args]"}. The {@code errorCode} branch of {@link #toSQLException()} drops the
 * cause to match legacy parity.
 */
@Getter
public class SFSQLException extends SnowflakeSQLExceptionCarrier {
  private static final long serialVersionUID = 1L;

  private final ErrorCode errorCode;

  /**
   * An already-formed {@link SQLException} to re-surface unchanged (see {@link #surfacing}); {@code
   * null} for carriers that {@link #toSQLException()} rebuilds from {@code (errorCode, message,
   * cause)}.
   */
  private final SQLException surfaced;

  public SFSQLException(String message) {
    super(message);
    this.errorCode = null;
    this.surfaced = null;
  }

  public SFSQLException(String message, Throwable cause) {
    super(message, cause);
    this.errorCode = null;
    this.surfaced = null;
  }

  public SFSQLException(ErrorCode errorCode, String message) {
    super(message);
    this.errorCode = errorCode;
    this.surfaced = null;
  }

  private SFSQLException(SQLException surfaced) {
    super(surfaced.getMessage(), surfaced);
    this.errorCode = null;
    this.surfaced = surfaced;
  }

  /**
   * Carrier that re-surfaces an already-formed {@link SQLException} unchanged when it crosses this
   * impl's decorator boundary. Used to rethrow a checked exception caught from an already-decorated
   * delegate (e.g. a pooled physical connection) without losing its vendor code, SQL state, cause,
   * or query id — a plain {@code new SFSQLException(message, cause)} would rebuild it with vendor
   * code 0.
   */
  public static SFSQLException surfacing(SQLException surfaced) {
    return new SFSQLException(surfaced);
  }

  /** Renders the message through the {@link ErrorCode} template (with null-template fallback). */
  public static SFSQLException fromErrorCode(ErrorCode errorCode, Object... params) {
    return new SFSQLException(errorCode, buildMessage(errorCode, params));
  }

  private static String buildMessage(ErrorCode errorCode, Object... params) {
    Object[] args = params == null ? new Object[0] : params;
    String template = errorCode.getMessageTemplate();
    if (template != null) {
      return MessageFormat.format(template, args);
    }
    if (args.length == 0) {
      return String.valueOf(errorCode);
    }
    return String.format("%s: %s", errorCode, Arrays.toString(args));
  }

  @Override
  public SQLException toSQLException() {
    if (surfaced != null) {
      return surfaced;
    }
    if (errorCode != null) {
      return new SnowflakeSQLException(errorCode, getMessage(), getQueryId());
    }
    if (getCause() != null || getQueryId() != null) {
      return new SnowflakeSQLException(getMessage(), getCause(), getQueryId());
    }
    return new SnowflakeSQLException(getMessage());
  }
}
