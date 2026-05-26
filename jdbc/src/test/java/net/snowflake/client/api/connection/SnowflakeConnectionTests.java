package net.snowflake.client.api.connection;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

class SnowflakeConnectionTests extends SnowflakeIntegrationTestBase {

  @Test
  void isValidReturnsTrueOnOpenConnection() throws Exception {
    try (Connection conn = openConnection()) {
      assertTrue(conn.isValid(0));
    }
  }
}
