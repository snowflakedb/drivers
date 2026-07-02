package net.snowflake.jdbc.wiremock.connection;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

public class LogoutTest extends BaseWiremockTest {

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  private Properties connectionProps() {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");
    return props;
  }

  @Test
  void closeSendsLogoutRequest() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");
    wiremock.addMapping("session/logout_success.json");

    Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps());
    conn.close();

    wiremock.verifyRequestCount(1, "/session$");
    assertTrue(conn.isClosed());
  }

  @Test
  void closeDoesNotThrowWhenServerReturns500() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");
    wiremock.addMapping("session/logout_500_always.json");

    Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps());
    assertDoesNotThrow(conn::close);
    assertTrue(conn.isClosed());
  }

  @Test
  void closeThrowsWhenStrictModeAndServerReturns500() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");
    wiremock.addMapping("session/logout_500_always.json");

    Properties props = connectionProps();
    props.setProperty("logout_error_strategy", "strict");

    Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), props);
    assertThrows(SQLException.class, conn::close);
    assertTrue(conn.isClosed());
  }

  @Test
  void closeRetriesOn503() throws Exception {
    wiremock.addMapping("auth/login_success_any.json");
    wiremock.addMapping("session/logout_503_then_success.json");

    Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), connectionProps());
    conn.close();

    int logoutRequests = wiremock.getRequests("/session$").size();
    assertTrue(
        logoutRequests >= 2, "Expected at least 2 logout attempts (retry), got " + logoutRequests);
    assertTrue(conn.isClosed());
  }
}
