package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * JDBC-specific coverage for {@link ResultSet#isLast()}. This behaviour has no cross-driver Gherkin
 * scenario, so the tests live in a dedicated class not mapped to any shared feature file.
 */
public class ResultSetIsLastTests extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldReportIsLastOnlyOnFinalRow() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When a five-row result set is iterated
    String sql = "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      // Then isLast() is true only on the final row
      assertFalse(resultSet.isLast(), "isLast() should be false before the first row");

      int lastCount = 0;
      while (resultSet.next()) {
        if (resultSet.getInt(1) == 4) {
          assertTrue(resultSet.isLast(), "isLast() should be true on the final row");
          lastCount++;
        } else {
          assertFalse(resultSet.isLast(), "isLast() should be false on non-final rows");
        }
      }
      assertEquals(1, lastCount, "isLast() should be true for exactly one row");

      assertFalse(resultSet.isLast(), "isLast() should be false after the last row");
    }
  }

  @Test
  public void shouldReportIsLastForSingleRowResult() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When a single-row result set is iterated
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
      // Then the only row is also the last row
      assertTrue(resultSet.next(), "Expected one row");
      assertTrue(resultSet.isLast(), "The only row should be the last row");
      assertFalse(resultSet.next(), "Expected exactly one row");
      assertFalse(resultSet.isLast(), "isLast() should be false after the last row");
    }
  }

  @Test
  public void shouldReportIsLastBeforeFirstRowOfEmptyResultSet() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When an empty result set is obtained
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT 1 WHERE 1=0")) {
      // Then isLast() matches snowflake-jdbc, which reports the before-first cursor of an empty
      // result set as the last row
      assertTrue(resultSet.isLast(), "isLast() should match snowflake-jdbc on an empty result set");
      assertFalse(resultSet.next(), "Expected an empty result set");
    }
  }
}
