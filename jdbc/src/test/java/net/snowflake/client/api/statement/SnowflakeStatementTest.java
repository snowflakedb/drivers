package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/** Tests for executing queries through the Snowflake JDBC Driver */
public class SnowflakeStatementTest extends SnowflakeIntegrationTestBase {

  @Test
  public void testSimpleSelect() throws Exception {
    Connection conn = getDefaultConnection();
    // Create and execute statement
    try (Statement stmt = conn.createStatement()) {
      try (ResultSet rs = stmt.executeQuery("SELECT 1")) {

        // Verify result
        assertNotNull(rs, "ResultSet should not be null");
        assertTrue(rs.next(), "ResultSet should have one row");
        assertEquals(1, rs.getInt(1), "Result should be 1");
      }
    }
  }

  @Test
  public void testStatementExecuteWithDecfloatResultSet() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 123.456::DECFLOAT")) {
      assertNotNull(rs, "ResultSet should be available after executeQuery");
      assertTrue(rs.next(), "Expected one row");
      assertEquals(new BigDecimal("123.456"), rs.getBigDecimal(1));
      assertFalse(rs.next(), "Expected exactly one row");
    }
  }
}
