package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class LargeResultSetTests extends SnowflakeIntegrationTestBase {
  @Test
  public void shouldProcessOneMillionRowResultSet() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id"
    // is executed
    String sql = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then there are 1000000 numbered sequentially rows returned
      long expectedValue = 0;
      while (resultSet.next()) {
        final long row = expectedValue;
        assertEquals(expectedValue, resultSet.getLong(1), () -> "Unexpected value at row " + row);
        assertFalse(resultSet.wasNull(), () -> "Sequential value should not be NULL at row " + row);
        expectedValue++;
      }
      assertEquals(1_000_000L, expectedValue, "Unexpected number of sequential rows");
    }
  }
}
