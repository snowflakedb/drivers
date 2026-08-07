package net.snowflake.client.internal.api.implementation.exception;

import java.sql.ClientInfoStatus;
import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.util.Map;
import lombok.Getter;

/**
 * Carrier for a client-info failure, mapping to a checked {@link SQLClientInfoException} and
 * preserving the exact {@code (message, sqlState, vendorCode, failedProperties)} tuple. Thrown by
 * impl-tier connection code ({@code SnowflakeConnectionImpl#setClientInfo}).
 */
@Getter
public class SFClientInfoException extends DriverRuntimeException {
  private static final long serialVersionUID = 1L;

  private final String sqlState;
  private final int vendorCode;
  private final Map<String, ClientInfoStatus> failedProperties;

  public SFClientInfoException(
      String message,
      String sqlState,
      int vendorCode,
      Map<String, ClientInfoStatus> failedProperties) {
    super(message);
    this.sqlState = sqlState;
    this.vendorCode = vendorCode;
    this.failedProperties = failedProperties;
  }

  @Override
  public SQLException toSQLException() {
    return new SQLClientInfoException(getMessage(), sqlState, vendorCode, failedProperties);
  }
}
