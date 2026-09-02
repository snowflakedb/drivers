package net.snowflake.perf;

import java.sql.Connection;
import java.sql.Driver;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.List;
import java.util.Properties;

final class ConnectionFactory {

  private static final String UNIVERSAL_DRIVER = "net.snowflake.client.api.driver.SnowflakeDriver";
  private static final String OLD_DRIVER = "net.snowflake.client.jdbc.SnowflakeDriver";

  private ConnectionFactory() {}

  // Both drivers register jdbc:snowflake://, so instantiate the intended one by name and call
  // Driver.connect directly instead of relying on DriverManager.
  static Connection connect(String driverType, String url, Properties props) throws Exception {
    String driverClass = "old".equals(driverType) ? OLD_DRIVER : UNIVERSAL_DRIVER;
    Driver driver = (Driver) Class.forName(driverClass).getDeclaredConstructor().newInstance();
    Connection conn = driver.connect(url, props);
    if (conn == null) {
      throw new SQLException("Driver " + driverClass + " did not accept URL: " + url);
    }
    return conn;
  }

  static String driverVersion(Connection conn) {
    try {
      return conn.getMetaData().getDriverVersion();
    } catch (SQLException e) {
      System.out.println("Warning: could not determine driver version: " + e.getMessage());
      return "UNKNOWN";
    }
  }

  static String serverVersion(Connection conn) {
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_VERSION()")) {
      return rs.next() ? rs.getString(1) : "UNKNOWN";
    } catch (SQLException e) {
      System.out.println("Warning: could not retrieve server version: " + e.getMessage());
      return "UNKNOWN";
    }
  }

  static void executeSetupQueries(Connection conn, List<String> setupQueries) throws SQLException {
    executeSetupQueries(conn, setupQueries, true);
  }

  static void executeSetupQueries(Connection conn, List<String> setupQueries, boolean logEach)
      throws SQLException {
    for (String query : setupQueries) {
      if (logEach) {
        System.out.println("Setup query: " + query);
      }
      try (Statement stmt = conn.createStatement()) {
        stmt.execute(query);
      }
    }
  }
}
