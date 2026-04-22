package net.snowflake.jdbc.e2e.session;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class SessionParametersTests extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldForwardUnrecognizedConnectionOptionAsSessionParameter() throws Exception {
    // Given Snowflake client is logged in with connection option QUERY_TAG set to
    // "session_param_e2e_test"
    Properties props = loadConnectionProperties();
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
}
