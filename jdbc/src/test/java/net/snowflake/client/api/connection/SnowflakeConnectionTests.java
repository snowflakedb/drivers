package net.snowflake.client.api.connection;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

class SnowflakeConnectionTests extends SnowflakeIntegrationTestBase {

  @Test
  void isValidReturnsTrueOnOpenConnection() throws SQLException {
    try (Connection conn = openConnection()) {
      assertTrue(conn.isValid(0));
    }
  }

  @Test
  void shouldReturnSessionIdOnOpenConnection() throws SQLException {
    try (Connection conn = openConnection()) {
      String sessionID = conn.unwrap(SnowflakeConnection.class).getSessionID();
      assertNotNull(sessionID);
      assertTrue(Long.parseLong(sessionID) > 0);
    }
  }
}
