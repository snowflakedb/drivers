package net.snowflake.client.api.exception;

public enum ErrorCode {
  INTERNAL_ERROR(200001),
  INVALID_VALUE_CONVERT(200038);

  private final int messageCode;

  ErrorCode(int messageCode) {
    this.messageCode = messageCode;
  }

  public int getMessageCode() {
    return messageCode;
  }
}
