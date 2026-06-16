package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import net.snowflake.jdbc.utils.TestParameters;
import org.junit.jupiter.api.Test;

class PatTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldAuthenticateUsingPatAsPassword() throws Exception {
    // Given Authentication is set to password and valid PAT token is provided
    Properties props = loadConnectionProperties();
    props.setProperty("password", TestParameters.get("SNOWFLAKE_TEST_PAT"));

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPatAsToken() throws Exception {
    // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
    props.setProperty("token", TestParameters.get("SNOWFLAKE_TEST_PAT"));

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPatAsTokenWithLowercaseAuthenticator() throws Exception {
    // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is
    // provided
    Properties props = loadConnectionProperties();
    props.remove("password");
    props.setProperty("authenticator", "programmatic_access_token");
    props.setProperty("token", TestParameters.get("SNOWFLAKE_TEST_PAT"));

    // When Trying to Connect
    String url = buildJdbcUrl(props);
    try (Connection conn = DriverManager.getConnection(url, props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailPatAuthenticationWhenInvalidTokenProvided() throws Exception {
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
}
