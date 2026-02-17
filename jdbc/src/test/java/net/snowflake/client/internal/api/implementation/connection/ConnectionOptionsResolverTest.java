package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;
import java.util.Properties;
import org.junit.jupiter.api.Test;

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

  @Test
  public void parseConnectStringPrefersPropertiesOverUrlOnConflicts() {
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
}
