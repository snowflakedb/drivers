package net.snowflake.client.internal.api.implementation.exception;

import lombok.Getter;
import net.snowflake.client.api.exception.SnowflakeSQLException;

/**
 * Base for carriers that map to a {@link SnowflakeSQLException}, as opposed to those mapping to a
 * more specific JDBC edge type ({@link SFBatchUpdateException}, {@link SFClientInfoException},
 * {@link SFSQLFeatureNotSupportedException}). Those edge types have no query-id slot, so {@code
 * queryId} lives here rather than on {@link DriverRuntimeException}. It is optional and set
 * fluently at the throw site — {@code throw new SFSQLException(msg).withQueryId(qid)} — and flows
 * into the surfaced {@link SnowflakeSQLException#getQueryId()}.
 */
@Getter
public abstract class SnowflakeSQLExceptionCarrier extends DriverRuntimeException {
  private static final long serialVersionUID = 1L;

  private String queryId;

  protected SnowflakeSQLExceptionCarrier(String message) {
    super(message);
  }

  protected SnowflakeSQLExceptionCarrier(String message, Throwable cause) {
    super(message, cause);
  }

  public SnowflakeSQLExceptionCarrier withQueryId(String queryId) {
    this.queryId = queryId;
    return this;
  }
}
