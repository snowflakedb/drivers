package net.snowflake.jdbc.e2e.session;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class ClientOptionsTests {

  @Test
  void shouldSetClientPrefetchThreadsViaConnectionProperty() throws Exception {
    // Given Snowflake client is logged in with CLIENT_PREFETCH_THREADS set to 8
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    props.setProperty("CLIENT_PREFETCH_THREADS", "8");
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        Statement stmt = conn.createStatement();
        // When Query "SHOW PARAMETERS LIKE 'CLIENT_PREFETCH_THREADS'" is executed
        ResultSet rs = stmt.executeQuery("SHOW PARAMETERS LIKE 'CLIENT_PREFETCH_THREADS'")) {
      // Then the session parameter value should be "8"
      assertTrue(rs.next(), "Expected one row");
      assertEquals("8", rs.getString("value"));
    }
  }
}
