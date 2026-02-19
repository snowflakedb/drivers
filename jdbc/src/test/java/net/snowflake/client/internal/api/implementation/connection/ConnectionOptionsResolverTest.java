package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;
import java.util.Properties;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

public class ConnectionOptionsResolverTest {

  @Test
  public void buildConnectionOptionsUsesParsedParamsAndDerivesAccount() {
    Properties input = new Properties();
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://globalaccount-12345.global.snowflakecomputing.com?warehouse=TEST_WH&schema=PUBLIC",
            input);

    assertEquals("globalaccount-12345.global.snowflakecomputing.com", resolved.get("host"));
    assertEquals("443", resolved.get("port"));
    assertEquals("https", resolved.get("protocol"));
    assertEquals("globalaccount", resolved.get("account"));
    assertEquals("TEST_WH", resolved.get("warehouse"));
    assertEquals("PUBLIC", resolved.get("schema"));
  }

  @Test
  public void resolveDoesNotOverrideTypedValuesWhenKeyAlreadyExists() {
    Properties input = new Properties();
    input.put("port", 9999);
    input.put("ssl", Boolean.TRUE);

    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://typed.snowflakecomputing.com:443?ssl=off", input);

    assertEquals(9999, resolved.get("port"));
    assertEquals(Boolean.TRUE, resolved.get("ssl"));
  }

  @Test
  public void resolveSkipsBlankQueryParameterKeys() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?=blankkey&warehouse=WH",
            new Properties());

    assertFalse(resolved.containsKey(""));
    assertEquals("WH", resolved.get("warehouse"));
  }

  @Test
  public void parseConnectionStringDecodesEscapedValuesAndForcesHttpWhenSslOff() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://testaccount.com:8080?proxyHost=%3d%2f&proxyPort=777&ssl=off",
            new Properties());
    assertTrue(parsed.isValid());
    assertEquals("http", parsed.getScheme());
    assertEquals("testaccount.com", parsed.getHost());
    assertEquals(8080, parsed.getPort());
    assertEquals("testaccount", parsed.getAccount());

    Map<String, Object> params = parsed.getParameters();
    assertEquals("=/", params.get("PROXYHOST"));
    assertEquals("777", params.get("PROXYPORT"));
    assertEquals("off", params.get("SSL"));
    assertEquals("testaccount", params.get("ACCOUNT"));
  }

  @ParameterizedTest
  @CsvSource({
    "jdbc:snowflake://testaccount.com, https, 443",
    "jdbc:snowflake://testaccount.com?ssl=off, http, 80",
    "jdbc:snowflake://http://testaccount.localhost, http, 80"
  })
  public void parseConnectionStringUsesExpectedDefaultPortForEffectiveScheme(
      String url, String expectedScheme, int expectedPort) {
    ConnectionString parsed = ConnectionString.parse(url, new Properties());

    assertTrue(parsed.isValid());
    assertEquals(expectedScheme, parsed.getScheme());
    assertEquals(expectedPort, parsed.getPort());
  }

  @Test
  public void resolveUsesHttpDefaultPortWhenSslOffAndNoPortProvided() {
    Properties resolved =
        ConnectionOptionsResolver.resolve(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?ssl=off", new Properties());

    assertEquals("http", resolved.get("protocol"));
    assertEquals("80", resolved.get("port"));
  }

  @Test
  public void parseConnectStringKeepsSchemeWhenSslOnOverridesUrlSslOffForJdbcCompatibility() {
    Properties input = new Properties();
    input.setProperty("warehouse", "FROM_PROPERTIES");
    input.setProperty("ssl", "on");
    input.setProperty("account", "from_properties_account");

    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://fromurl.snowflakecomputing.com?warehouse=FROM_URL&ssl=off&account=from_url_account",
            input);

    assertTrue(parsed.isValid());
    assertEquals("http", parsed.getScheme());
    assertEquals("from_properties_account", parsed.getAccount());
    assertEquals("FROM_PROPERTIES", parsed.getParameters().get("WAREHOUSE"));
    assertEquals("on", parsed.getParameters().get("SSL"));
    assertEquals("from_properties_account", parsed.getParameters().get("ACCOUNT"));
  }

  @ParameterizedTest
  @CsvSource({"false,http", "true,https"})
  public void parseConnectionStringInterpretsBooleanSslByValue(
      boolean sslValue, String expectedScheme) {
    Properties input = new Properties();
    input.put("ssl", sslValue);

    ConnectionString parsed =
        ConnectionString.parse("jdbc:snowflake://testaccount.snowflakecomputing.com", input);

    assertTrue(parsed.isValid());
    assertEquals(expectedScheme, parsed.getScheme());
    assertEquals(sslValue, parsed.getParameters().get("SSL"));
  }

  @Test
  public void parseConnectionStringRetainsValueSuffixAndKeepsLegacyInvalidPairBehavior() {
    ConnectionString parsed =
        ConnectionString.parse(
            "jdbc:snowflake://testaccount.snowflakecomputing.com?token=abc==&empty=&novalue&=blankkey",
            new Properties());

    assertTrue(parsed.isValid());
    assertEquals("abc==", parsed.getParameters().get("TOKEN"));
    assertFalse(parsed.getParameters().containsKey("EMPTY"));
    assertFalse(parsed.getParameters().containsKey("NOVALUE"));
    assertEquals("blankkey", parsed.getParameters().get(""));
  }
}
