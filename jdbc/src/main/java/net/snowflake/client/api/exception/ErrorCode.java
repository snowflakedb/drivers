package net.snowflake.client.api.exception;

public enum ErrorCode {
  INTERNAL_ERROR(200001),
  INVALID_VALUE_CONVERT(200038),
  UNSUPPORTED_STATEMENT_TYPE_IN_EXECUTION_API(200042, "0A000");

  private final int messageCode;
  // Nullable by design: only SQL-standardized errors should provide SQLSTATE explicitly.
  private final String sqlState;

  ErrorCode(int messageCode) {
    this(messageCode, null);
  }

  ErrorCode(int messageCode, String sqlState) {
    this.messageCode = messageCode;
    this.sqlState = sqlState;
  }

  public int getMessageCode() {
    return messageCode;
  }

  public String getSqlState() {
    return sqlState;
  }
}
