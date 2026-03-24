package net.snowflake.client.api.exception;


import lombok.Getter;

@Getter
public enum ErrorCode {
  INTERNAL_ERROR(200001, "HY000"),
  INVALID_VALUE_CONVERT(200038, "0A000"),
  CONNECTION_CLOSED(200052, "08003");

  private final int messageCode;
  private final String sqlState;

  ErrorCode(Integer messageCode, String sqlState) {
    this.messageCode = messageCode;
    this.sqlState = sqlState;
  }
}
