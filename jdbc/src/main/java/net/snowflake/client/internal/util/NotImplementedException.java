package net.snowflake.client.internal.util;

public class NotImplementedException extends RuntimeException {

  public NotImplementedException() {}

  public NotImplementedException(String message) {
    super(message);
  }
}
