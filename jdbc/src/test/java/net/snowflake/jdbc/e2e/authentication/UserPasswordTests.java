package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withSnowflakeAuth;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class UserPasswordTests implements WithQueryUtils, WithConnect {

  @Test
  void shouldAuthenticateUsingUsernameAndPassword() throws Exception {
    // Given Authentication is set to default (snowflake) with valid username and password
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldAuthenticateUsingExplicitSnowflakeAuthenticator() throws Exception {
    // Given Authentication is explicitly set to snowflake with valid username and password
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());
    props.setProperty("authenticator", "snowflake");

    // When Trying to Connect
    try (Connection conn = connect(props)) {
      // Then Login is successful and simple query can be executed
      assertSimpleQuerySucceeds(conn);
    }
  }

  @Test
  void shouldFailAuthenticationWhenWrongPasswordIsProvided() {
    // Given Authentication is set to default with valid username and wrong password
    Properties props = withSnowflakeAuth(loadDefaultConnectionProperties());
    props.setProperty("password", "wrong_password");

    // When Trying to Connect
    Executable connect = () -> connect(props);

    // Then There is error returned
    assertThrows(SQLException.class, connect);
  }
}
