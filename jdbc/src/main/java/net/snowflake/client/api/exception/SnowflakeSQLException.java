package net.snowflake.client.api.exception;

import java.sql.SQLException;
import lombok.Getter;
import net.snowflake.client.internal.api.implementation.exception.ErrorMessageRenderer;

@Getter
public class SnowflakeSQLException extends SQLException {
  private static final long serialVersionUID = 1L;

  /**
   * @return the Snowflake query ID associated with the failed query, or {@code null} when the
   *     server did not surface one.
   */
  private final String queryId;

  public SnowflakeSQLException(String message) {
    super(message);
    this.queryId = null;
  }

  public SnowflakeSQLException(String message, Throwable cause) {
    this(message, cause, null);
  }

  /** As {@link #SnowflakeSQLException(String, Throwable)} but also carrying a query id. */
  public SnowflakeSQLException(String message, Throwable cause, String queryId) {
    super(message, cause);
    this.queryId = queryId;
  }

  public SnowflakeSQLException(ErrorCode errorCode, String message) {
    this(errorCode, message, null);
  }

  /** As {@link #SnowflakeSQLException(ErrorCode, String)} but also carrying a query id. */
  public SnowflakeSQLException(ErrorCode errorCode, String message, String queryId) {
    super(message, errorCode.getSqlState(), errorCode.getMessageCode());
    this.queryId = queryId;
  }

  /** As {@link SQLException#SQLException(String, String, int, Throwable)} plus query id. */
  public SnowflakeSQLException(
      String reason, String sqlState, int vendorCode, Throwable cause, String queryId) {
    super(reason, sqlState, vendorCode, cause);
    this.queryId = queryId;
  }

  /**
   * Renders the message from the {@link ErrorCode} with this vendor code, or from the code and
   * {@code params} when none matches.
   *
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String sqlState, int vendorCode, Object... params) {
    this((String) null, sqlState, vendorCode, params);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String queryId, String sqlState, int vendorCode) {
    this(ErrorMessageRenderer.render(vendorCode), sqlState, vendorCode, null, queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String queryId, String sqlState, int vendorCode, Object... params) {
    this(ErrorMessageRenderer.render(vendorCode, params), sqlState, vendorCode, null, queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String queryId, String reason, String sqlState) {
    this(reason, sqlState, 0, null, queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String queryId, String reason, String sqlState, int vendorCode) {
    this(reason, sqlState, vendorCode, null, queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(
      String queryId, Throwable cause, String sqlState, int vendorCode, Object... params) {
    this(ErrorMessageRenderer.render(vendorCode, params), sqlState, vendorCode, cause, queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(ErrorCode, String, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(String queryId, ErrorCode errorCode, Object... params) {
    this(
        ErrorMessageRenderer.render(errorCode, params),
        errorCode.getSqlState(),
        errorCode.getMessageCode(),
        null,
        queryId);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, Throwable)}.
   */
  @Deprecated
  public SnowflakeSQLException(Throwable cause, String message) {
    this(message, cause);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(Throwable cause, String sqlState, int vendorCode) {
    this(ErrorMessageRenderer.render(vendorCode), sqlState, vendorCode, cause, null);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(Throwable cause, String sqlState, int vendorCode, Object... params) {
    this(ErrorMessageRenderer.render(vendorCode, params), sqlState, vendorCode, cause, null);
  }

  /**
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(String, String, int, Throwable, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(Throwable cause, ErrorCode errorCode, Object... params) {
    this(
        ErrorMessageRenderer.render(errorCode, params),
        errorCode.getSqlState(),
        errorCode.getMessageCode(),
        cause,
        null);
  }

  /**
   * Renders the message from {@code errorCode}'s template. One or two {@link String} arguments
   * resolve to the constructors that take a message and a query id, not template parameters.
   *
   * @deprecated Retained for source compatibility with snowflake-jdbc. Use {@link
   *     #SnowflakeSQLException(ErrorCode, String)}.
   */
  @Deprecated
  public SnowflakeSQLException(ErrorCode errorCode, Object... params) {
    this((String) null, errorCode, params);
  }
}
