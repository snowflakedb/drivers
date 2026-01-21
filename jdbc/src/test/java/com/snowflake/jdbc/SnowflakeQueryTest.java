package com.snowflake.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import org.junit.jupiter.api.Test;

/** Tests for executing queries through the Snowflake JDBC Driver */
public class SnowflakeQueryTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testSimpleSelect() throws Exception {
    try (Connection conn = openConnection()) {
      // Create and execute statement
      Statement stmt = conn.createStatement();
      ResultSet rs = stmt.executeQuery("SELECT 1");

      // Verify result
      assertNotNull(rs, "ResultSet should not be null");
      assertTrue(rs.next(), "ResultSet should have one row");
      assertEquals(1, rs.getInt(1), "Result should be 1");

      // Clean up
      rs.close();
      stmt.close();
    }
  }
}
