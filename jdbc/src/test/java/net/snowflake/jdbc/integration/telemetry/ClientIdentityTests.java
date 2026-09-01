package net.snowflake.jdbc.integration.telemetry;

import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import java.sql.Connection;
import java.sql.DriverManager;
import java.util.List;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/**
 * WireMock integration tests for JDBC client identity on the login request. Mirrors {@code
 * python/tests/integ/telemetry/test_client_identity.py} and the ODBC wire-level tests in {@code
 * sf_core/tests/integration/authentication/odbc_client_identity.rs}.
 */
class ClientIdentityTests extends BaseWiremockTest {

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  @SkipOldDriver("New-driver-only: client identity fields via WireMock")
  void shouldSendJdbcClientIdentityInLoginRequest() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");

    try (Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps())) {
      assertFalse(conn.isClosed());
    }

    List<JsonNode> loginRequests = wiremock.getRequests("/session/v1/login-request.*");
    assertTrue(loginRequests.size() >= 1, "Expected at least one login-request");

    JsonNode data = parseJson(loginRequests.get(0).get("body").asText()).get("data");

    assertEquals("JDBC", data.get("CLIENT_APP_ID").asText());
    // CLIENT_APP_VERSION is sent unstripped from wrapper identity (no FULL field).
    assertEquals(SnowflakeDriver.CLIENT_APP_VERSION, data.get("CLIENT_APP_VERSION").asText());

    JsonNode env = data.get("CLIENT_ENVIRONMENT");
    assertEquals("JDBC", env.get("APPLICATION").asText());
    assertFalse(env.get("OS").asText().isEmpty(), "OS must not be empty");
    assertFalse(env.get("OS_VERSION").asText().isEmpty(), "OS_VERSION must not be empty");
    assertEquals(System.getProperty("java.vm.name").trim(), env.get("RUNTIME_NAME").asText());
    assertEquals(System.getProperty("java.version").trim(), env.get("RUNTIME_VERSION").asText());
    assertEquals("prpr2", env.get("RELEASE_TYPE").asText());
  }

  private Properties connectionProps() {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");
    return props;
  }
}
