package net.snowflake.jdbc.e2e.query;

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
import net.snowflake.client.api.statement.SnowflakeStatement;
import org.junit.jupiter.api.Test;

class QueryTagTests {

  @Test
  void shouldTagQueriesWhenQueryTagIsSetAtConnectionLevel() throws Exception {
    // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    props.setProperty("QUERY_TAG", "conn_tag_e2e");
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        // When Query "SELECT CURRENT_QUERY_TAG()" is executed
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_QUERY_TAG()")) {
      // Then the result should contain value "conn_tag_e2e"
      assertTrue(rs.next(), "Expected one row");
      assertEquals("conn_tag_e2e", rs.getString(1));
    }
  }

  @Test
  void shouldTagASingleQueryViaStatementLevelQueryTag() throws Exception {
    // Given Snowflake client is logged in
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props);
        Statement stmt = conn.createStatement()) {
      // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG
      // "stmt_tag_e2e"
      stmt.unwrap(SnowflakeStatement.class).setParameter("QUERY_TAG", "stmt_tag_e2e");
      try (ResultSet rs = stmt.executeQuery("SELECT CURRENT_QUERY_TAG()")) {
        // Then the result should contain value "stmt_tag_e2e"
        assertTrue(rs.next(), "Expected one row");
        assertEquals("stmt_tag_e2e", rs.getString(1));
      }
    }
  }

  @Test
  void shouldNotLeakStatementLevelQueryTagIntoSessionState() throws Exception {
    // Given Snowflake client is logged in
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG
      // "stmt_tag_e2e"
      try (Statement tagged = conn.createStatement()) {
        tagged.unwrap(SnowflakeStatement.class).setParameter("QUERY_TAG", "stmt_tag_e2e");
        try (ResultSet rs = tagged.executeQuery("SELECT CURRENT_QUERY_TAG()")) {
          assertTrue(rs.next(), "Expected one row");
          assertEquals("stmt_tag_e2e", rs.getString(1));
        }
      }
      // And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
      try (Statement untagged = conn.createStatement();
          ResultSet rs = untagged.executeQuery("SELECT CURRENT_QUERY_TAG()")) {
        // Then the last result should contain empty value
        assertTrue(rs.next(), "Expected one row");
        assertEquals("", rs.getString(1));
      }
    }
  }
}
