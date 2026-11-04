package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import java.util.logging.Handler;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.Isolated;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

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
  void shouldParseExplicitPortDecodedQueryAndSslOverrideFromProperties() {
    Properties info = new Properties();
    info.put("ssl", Boolean.FALSE);

    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://my_acct.snowflakecomputing.com:8080/?warehouse=WH%201&emptyKey=",
            info);

    assertTrue(parsed.isValid());
    assertEquals("http", parsed.getScheme());
    assertEquals(8080, parsed.getPort());
    assertEquals("my_acct", parsed.getAccount());
    assertEquals("my-acct.snowflakecomputing.com", parsed.getHost());
    assertEquals("WH 1", parsed.getParameters().get("WAREHOUSE"));
    assertFalse(parsed.getParameters().containsKey("EMPTYKEY"));
  }

  @Test
  void shouldPreserveUnderscoreHostWhenAllowUnderscoresInHostIsTrue() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://my_acct.snowflakecomputing.com?allowUnderscoresInHost=true");

    assertTrue(parsed.isValid());
    assertEquals("my_acct", parsed.getAccount());
    assertEquals("my_acct.snowflakecomputing.com", parsed.getHost());
    assertEquals("https", parsed.getScheme());
    assertEquals(443, parsed.getPort());
  }

  @Test
  void shouldDeriveGlobalAccountAndDefaultHttpPortWhenSslIsOffInQuery() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://org-acct-locator123.global.snowflakecomputing.com?ssl=off");

    assertTrue(parsed.isValid());
    assertEquals("org-acct", parsed.getAccount());
    assertEquals("http", parsed.getScheme());
    assertEquals(80, parsed.getPort());
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "jdbc:postgresql://localhost/db",
        "jdbc:snowflake://",
        "jdbc:snowflake://host.snowflakecomputing.com/extra/path",
        "jdbc:snowflake://localhost:8080",
        "jdbc:snowflake://[invalid-uri"
      })
  void shouldReturnInvalidConnectionStringForMalformedUrl(String url) {
    ConnectionString parsed = ConnectionString.parse(url);

    assertFalse(parsed.isValid());
    assertEquals("", parsed.getHost());
    assertEquals(-1, parsed.getPort());
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
