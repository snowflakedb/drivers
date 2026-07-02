package net.snowflake.jdbc.wiremock;

import static org.junit.jupiter.api.Assertions.assertFalse;

import java.sql.Connection;
import java.sql.DriverManager;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.Test;

/**
 * End-to-end sanity test: proves that a JDBC connection routed at a wiremock-backed URL reaches
 * sf_core, which then issues the expected {@code /session/v1/login-request} HTTP call. Intended as
 * a baseline for more elaborate wiremock-driven JDBC tests (session renewal, telemetry, etc.).
 */
public class WiremockSanityTest extends BaseWiremockTest {

  @Test
  public void opensConnectionAndSendsLoginRequest() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");

    // ConnectionOptionsResolver always derives "protocol" from the JDBC URL scheme. sf_core
    // rejects setting both "ssl" and "protocol" as conflicting, so we set protocol=http here
    // and avoid also passing ssl=off.
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");

    Class.forName(SnowflakeDriver.class.getName());
    try (Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), props)) {
      assertFalse(conn.isClosed());
    }

    wiremock.verifyRequestCount(1, "/session/v1/login-request");
  }
}
