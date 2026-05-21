package net.snowflake.jdbc.e2e.authentication;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import java.util.Random;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import net.snowflake.client.api.datasource.SnowflakeDataSource;
import net.snowflake.client.api.datasource.SnowflakeDataSourceFactory;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

public class PatTests extends SnowflakeIntegrationTestBase {

  private String patToken;
  private String patTokenName;

  @BeforeAll
  void createPATToken() throws Exception {
    Properties props = loadConnectionProperties();
    String user = props.getProperty("user");
    String role = props.getProperty("role");
    patTokenName = "UD_JDBC_E2E_" + String.format("%08x", new Random().nextInt());

    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement();
        ResultSet rs =
            stmt.executeQuery(
                "ALTER USER IF EXISTS "
                    + user
                    + " ADD PROGRAMMATIC ACCESS TOKEN "
                    + patTokenName
                    + " ROLE_RESTRICTION = "
                    + role)) {
      assertTrue(rs.next(), "ALTER USER should return a result");
      patToken = rs.getString(2);
      assertNotNull(patToken, "PAT token secret should not be null");
      assertFalse(patToken.isEmpty(), "PAT token secret should not be empty");
    }
  }

  @AfterAll
  void cleanupPATToken() throws Exception {
    if (patTokenName != null) {
      try (Connection conn = openConnection();
          Statement stmt = conn.createStatement()) {
        Properties props = loadConnectionProperties();
        String user = props.getProperty("user");
        stmt.execute(
            "ALTER USER IF EXISTS " + user + " REMOVE PROGRAMMATIC ACCESS TOKEN " + patTokenName);
      } catch (Exception e) {
        System.err.println("Failed to cleanup PAT token " + patTokenName + ": " + e.getMessage());
      }
    }
  }

  @Test
  void shouldAuthenticateUsingPATAsPassword() throws Exception {
    // Given Authentication is set to password and valid PAT token is provided
    Properties props = loadConnectionProperties();
    props.setProperty("password", patToken);

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPATAsToken() throws Exception {
    // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
    props.setProperty("token", patToken);

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPATAsTokenWithLowercaseAuthenticator() throws Exception {
    // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is
    // provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "programmatic_access_token");
    props.setProperty("token", patToken);

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailPATAuthenticationWhenInvalidTokenProvided() throws Exception {
    // Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
    props.setProperty("token", "invalid_token_12345");

    // When Trying to Connect
    String url = buildJdbcUrl(props);

    // Then There is error returned
    assertThrows(SQLException.class, () -> DriverManager.getConnection(url, props));
  }

  @Test
  void shouldAuthenticateUsingPATViaDataSource() throws Exception {
    // Given DataSource is configured with PAT authentication
    Properties props = loadConnectionProperties();
    SnowflakeDataSource ds = SnowflakeDataSourceFactory.createDataSource();
    ds.setUrl(buildJdbcUrl(props));
    ds.setUser(props.getProperty("user"));
    ds.setAccount(props.getProperty("account"));
    ds.setAuthenticator("PROGRAMMATIC_ACCESS_TOKEN");
    ds.setToken(patToken);

    // When Trying to Connect via DataSource
    try (Connection conn = ds.getConnection()) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  private void assertSimpleQuerySucceeds(Connection conn) throws SQLException {
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1")) {
      assertTrue(rs.next(), "Result set should have at least one row");
      assertTrue(rs.getInt(1) == 1, "SELECT 1 should return 1");
    }
  }
}
