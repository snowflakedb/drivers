package net.snowflake.jdbc.wiremock;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import org.junit.jupiter.api.Test;

/**
 * Verifies that JDBC sees transparent session renewal: a query that hits a 401 "session expired"
 * response triggers sf_core's {@code execute_with_refresh} loop, which calls {@code
 * /session/token-request} and retries the query exactly once. From the JDBC caller's perspective
 * the {@code Statement.execute} call simply succeeds.
 */
public class SessionRenewalWiremockTest extends BaseWiremockTest {

  @Test
  public void queryRefreshesSessionOn401AndRetriesTransparently() throws Exception {
    // The four mappings together form the renewal scenario state machine
    // (Started -> Token Expired -> Token Refreshed):
    //   * login returns tokens with substrings expected by the renewal matchers
    //     (session "expired-session-token" + master "valid-master-token")
    //   * first query is 401, transitions state to "Token Expired"
    //   * token-request returns new tokens, transitions state to "Token Refreshed"
    //   * retry query returns success with statementTypeId=SCL (no result set)
    wiremock.addMapping("auth/login_for_session_renewal.json");
    wiremock.addMapping("session/query_401_then_refresh_then_success.json");
    wiremock.addMapping("session/token_refresh_success.json");
    wiremock.addMapping("session/query_success_after_refresh.json");

    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");

    Class.forName(SnowflakeDriver.class.getName());
    try (Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), props);
        Statement stmt = conn.createStatement()) {
      // Statement.execute() returning without exception is the success criterion: sf_core
      // saw 401, refreshed the session, retried the query, and got back a valid response.
      // The mock response carries statementTypeId=SCL (non-result-set) so the JDBC wrapper
      // skips the result-stream materialisation path.
      stmt.execute("USE DATABASE TEST_DB");
    }

    wiremock.verifyRequestCount(2, "/queries/v1/query-request");
    wiremock.verifyRequestCount(1, "/session/token-request");
  }
}
