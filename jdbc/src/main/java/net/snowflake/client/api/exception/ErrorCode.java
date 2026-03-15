package net.snowflake.client.api.exception;

public enum ErrorCode {
  INTERNAL_ERROR(200001, "HY000"),
  INVALID_VALUE_CONVERT(200038, "0A000"),
  CONNECTION_CLOSED(200052, "08003");

  private final Integer messageCode;
  private final String sqlState;

  ErrorCode(Integer messageCode, String sqlState) {
    this.messageCode = messageCode;
    this.sqlState = sqlState;
  }

  public Integer getMessageCode() {
    return messageCode;
  }

  public String getSqlState() {
    return sqlState;
  }
}
