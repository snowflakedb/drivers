package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertFalse;

import java.util.ArrayList;
import java.util.List;
import java.util.logging.Handler;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;

@Isolated("mutates the ConnectionString JUL logger handlers")
class ConnectionStringTest {

  @Test
  void shouldRejectGlobalHostWithoutAccountLocator() {
    assertFalse(
        ConnectionString.parse("jdbc:snowflake://account.global.snowflakecomputing.com").isValid());
  }

  @Test
  void shouldRejectAccountQueryValueContainingEquals() {
    assertFalse(
        ConnectionString.parse("jdbc:snowflake://foo.snowflakecomputing.com?account=foo=bar")
            .isValid());
    assertFalse(
        ConnectionString.parse("jdbc:snowflake://foo.snowflakecomputing.com?account=foo%3Dbar")
            .isValid());
  }

  @Test
  void shouldNotLogMalformedUrlContents() {
    String credentialMarker = "test_password";
    List<String> messages = new ArrayList<>();
    Handler handler =
        new Handler() {
          @Override
          public void publish(LogRecord record) {
            messages.add(record.getMessage());
            Object[] parameters = record.getParameters();
            if (parameters != null) {
              for (Object parameter : parameters) {
                if (parameter != null) {
                  messages.add(parameter.toString());
                }
              }
            }
            if (record.getThrown() != null) {
              messages.add(String.valueOf(record.getThrown()));
            }
          }

          @Override
          public void flush() {}

          @Override
          public void close() {}
        };
    Logger logger = Logger.getLogger(ConnectionString.class.getName());
    logger.addHandler(handler);
    try {
      ConnectionString.parse(
          "jdbc:snowflake://account.snowflakecomputing.com?password=" + credentialMarker + "|");
    } finally {
      logger.removeHandler(handler);
    }

    assertFalse(messages.isEmpty());
    assertFalse(messages.stream().anyMatch(message -> message.contains(credentialMarker)));
  }
}
