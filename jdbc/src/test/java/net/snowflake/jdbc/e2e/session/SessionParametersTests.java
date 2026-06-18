package net.snowflake.jdbc.e2e.session;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withSnowflakeAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class SessionParametersTests {

  @Test
  void shouldForwardUnrecognizedConnectionOptionAsSessionParameter() throws Exception {
    // Given Snowflake client is logged in with connection option QUERY_TAG set to
    // "session_param_e2e_test"
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());
    props.setProperty("QUERY_TAG", "session_param_e2e_test");
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        // When Query "SELECT CURRENT_QUERY_TAG()" is executed
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_QUERY_TAG()")) {
      // Then the result should contain value "session_param_e2e_test"
      assertTrue(rs.next(), "Expected one row");
      assertEquals("session_param_e2e_test", rs.getString(1));
    }
  }

  @Test
  void shouldEnableSessionKeepAliveViaConnectionString() throws Exception {
    // Given Snowflake client is logged in with connection option CLIENT_SESSION_KEEP_ALIVE set to
    // "true"
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());
    props.setProperty("CLIENT_SESSION_KEEP_ALIVE", "true");
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        Statement stmt = conn.createStatement();
        // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'" is executed
        ResultSet rs = stmt.executeQuery("SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE'")) {
      // Then the session parameter value should be "true"
      assertTrue(rs.next(), "Expected one row");
      assertEquals("true", rs.getString("value"));
    }
  }

  @Test
  void shouldSetHeartbeatFrequencyViaConnectionString() throws Exception {
    // Given Snowflake client is logged in with CLIENT_SESSION_KEEP_ALIVE=true and
    // CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY=1800
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());
    props.setProperty("CLIENT_SESSION_KEEP_ALIVE", "true");
    props.setProperty("CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY", "1800");
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        Statement stmt = conn.createStatement();
        // When Query "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'" is
        // executed
        ResultSet rs =
            stmt.executeQuery(
                "SHOW PARAMETERS LIKE 'CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY'")) {
      // Then the session parameter value reflects the configured frequency
      assertTrue(rs.next(), "Expected one row");
      assertEquals("1800", rs.getString("value"));
    }
  }
}
