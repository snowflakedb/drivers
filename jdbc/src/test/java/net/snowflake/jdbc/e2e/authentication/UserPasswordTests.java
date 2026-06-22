package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class UserPasswordTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");
  private static final String PASSWORD = TestParameters.get("SNOWFLAKE_TEST_PASSWORD");

  @Test
  void shouldAuthenticateUsingUsernameAndPassword() throws Exception {
    // Given Authentication is set to default (snowflake) with valid username and password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingExplicitSnowflakeAuthenticator() throws Exception {
    // Given Authentication is explicitly set to snowflake with valid username and password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "snowflake");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() {
    // Given Authentication is set to default with valid username and wrong password
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("user", USER);
    props.setProperty("password", "wrong_password");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    SQLException exception = assertThrows(SQLException.class, connect);
    assertTrue(
        exception
            .getMessage()
            .toLowerCase()
            .contains("incorrect username or password was specified"));
    if (isNewDriver()) {
      assertTrue(exception.getMessage().contains("390100"));
    }
  }
}
