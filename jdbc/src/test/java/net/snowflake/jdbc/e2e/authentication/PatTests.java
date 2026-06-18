package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class PatTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");
  private static final String PAT = TestParameters.get("SNOWFLAKE_TEST_PAT");

  @Test
  void shouldAuthenticateUsingPatAsPassword() throws Exception {
    // Given Authentication is set to password and valid PAT token is provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("password", PAT);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPatAsToken() throws Exception {
    // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
    props.setProperty("user", USER);
    props.setProperty("token", PAT);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingPatAsTokenWithLowercaseAuthenticator() throws Exception {
    // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is
    // provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "programmatic_access_token");
    props.setProperty("user", USER);
    props.setProperty("token", PAT);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailPatAuthenticationWhenInvalidTokenProvided() {
    // Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "PROGRAMMATIC_ACCESS_TOKEN");
    props.setProperty("user", USER);
    props.setProperty("token", "invalid_token_12345");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    assertThrows(SQLException.class, connect);
  }
}
