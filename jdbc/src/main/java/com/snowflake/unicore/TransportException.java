package com.snowflake.unicore;

/** Exception thrown when transport-level error occurs */
public class TransportException extends RuntimeException {
  public TransportException(String message) {
    super(message);
  }

  public TransportException(String message, Throwable cause) {
    super(message, cause);
  }
}
