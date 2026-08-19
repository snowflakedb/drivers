package net.snowflake.client.internal.api.implementation.exception;

import java.sql.SQLException;
import lombok.Getter;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1;

/**
 * Carrier for a core (Rust) failure surfaced through {@link
 * net.snowflake.client.internal.unicore.CoreDriverApi}. Any {@link
 * DatabaseDriverV1.DriverException} payload is retained verbatim in {@link #error} for
 * describe-mode inspection. {@link #toSQLException()} copies SQLState / vendor code / query id from
 * the payload, falling back via {@link StatusCodeMapper} when the payload carries none.
 */
@Getter
public class CoreException extends SnowflakeSQLExceptionCarrier {

  /** The core error payload, or {@code null} for a transport/plain-message failure. */
  private final DatabaseDriverV1.DriverException error;

  public CoreException(String message) {
    super(message);
    this.error = null;
  }

  public CoreException(String message, Throwable cause) {
    super(message, cause);
    this.error = null;
  }

  public CoreException(DatabaseDriverV1.DriverException error, Throwable cause) {
    super(error.hasRootCause() ? error.getRootCause() : error.getMessage(), cause);
    this.error = error;
  }

  /** An explicitly attached query id wins; otherwise the one the core reported in the payload. */
  @Override
  public String getQueryId() {
    String attached = super.getQueryId();
    if (attached != null) {
      return attached;
    }
    return (error != null && error.hasQueryId()) ? error.getQueryId() : null;
  }

  /**
   * Builds from the payload when present (preserving its SQLState / vendor code / query id),
   * otherwise from the carrier message (transport / payload-less failures). Retains {@code this} as
   * the cause. StatusCodes with no server vendor code fall back via {@link StatusCodeMapper}.
   */
  @Override
  public SQLException toSQLException() {
    if (error == null) {
      return new SnowflakeSQLException(getMessage(), this, getQueryId());
    }
    return new SnowflakeSQLException(
        error.hasRootCause() ? error.getRootCause() : error.getMessage(),
        StatusCodeMapper.sqlState(error),
        StatusCodeMapper.vendorCode(error),
        this,
        getQueryId());
  }
}
