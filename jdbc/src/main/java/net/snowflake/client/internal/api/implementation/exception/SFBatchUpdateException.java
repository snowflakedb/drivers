package net.snowflake.client.internal.api.implementation.exception;

import java.sql.BatchUpdateException;
import java.sql.SQLException;
import lombok.Getter;

/**
 * Carrier for a batch failure, mapping to a checked {@link BatchUpdateException} and preserving the
 * exact {@code (message, sqlState, vendorCode, updateCounts, cause)} tuple. Thrown by impl-tier
 * batch code ({@code StatementBatch}, {@code PreparedBatch}).
 */
@Getter
public class SFBatchUpdateException extends DriverRuntimeException {
  private static final long serialVersionUID = 1L;

  private final String sqlState;
  private final int vendorCode;
  private final int[] updateCounts;

  public SFBatchUpdateException(
      String message, String sqlState, int vendorCode, int[] updateCounts, Throwable cause) {
    super(message, cause);
    this.sqlState = sqlState;
    this.vendorCode = vendorCode;
    this.updateCounts = updateCounts;
  }

  @Override
  public SQLException toSQLException() {
    return new BatchUpdateException(getMessage(), sqlState, vendorCode, updateCounts, getCause());
  }
}
