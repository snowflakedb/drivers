package net.snowflake.jdbc.e2e.query;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class MaxRowsTests {

  private static final String HUNDRED_ROWS_QUERY =
      "SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 100))";
  private static final String MULTI_CHUNK_QUERY =
      "SELECT seq4(), randstr(100, random()) FROM TABLE(GENERATOR(ROWCOUNT => 500000))";

  @Test
  void shouldReturnAtMostMaxRowsRowsFromStatementQuery() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      // When max rows is set to 30 and a query producing 100 rows is executed
      stmt.setMaxRows(30);
      try (ResultSet rs = stmt.executeQuery(HUNDRED_ROWS_QUERY)) {
        // Then the result set should contain 30 rows
        assertEquals(30, countRows(rs));
      }
      // And max rows should be reported as 30
      assertEquals(30, stmt.getMaxRows());
    }
  }

  @Test
  void shouldCapMultiChunkResultToMaxRows() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      // When max rows is set to 10 and a query producing a multi-chunk result is executed
      stmt.setMaxRows(10);
      try (ResultSet rs = stmt.executeQuery(MULTI_CHUNK_QUERY)) {
        // Then the result set should contain 10 rows
        assertEquals(10, countRows(rs));
      }
    }
  }

  @Test
  void shouldReturnAtMostMaxRowsRowsFromPreparedStatementQuery() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection();
        PreparedStatement stmt =
            conn.prepareStatement("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => ?))")) {
      // When max rows is set to 30 and a prepared query producing 100 rows is executed
      stmt.setMaxRows(30);
      stmt.setInt(1, 100);
      try (ResultSet rs = stmt.executeQuery()) {
        // Then the result set should contain 30 rows
        assertEquals(30, countRows(rs));
      }
    }
  }

  @Test
  void shouldApplyMaxRowsToEveryLaterExecutionOfTheStatement() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      // When max rows is set to 30 and a query producing 100 rows is executed twice
      stmt.setMaxRows(30);
      try (ResultSet first = stmt.executeQuery(HUNDRED_ROWS_QUERY)) {
        // Then the first result set should contain 30 rows
        assertEquals(30, countRows(first));
      }
      // And the second result set should contain 30 rows
      assertTrue(stmt.execute(HUNDRED_ROWS_QUERY));
      try (ResultSet second = stmt.getResultSet()) {
        assertEquals(30, countRows(second));
      }
    }
  }

  @Test
  void shouldReturnAllRowsAfterMaxRowsIsResetToZero() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection();
        Statement stmt = conn.createStatement()) {
      // When max rows is set to 30 and then reset to 0
      stmt.setMaxRows(30);
      stmt.setMaxRows(0);
      try (ResultSet rs = stmt.executeQuery(HUNDRED_ROWS_QUERY)) {
        // Then the result set should contain all 100 rows
        assertEquals(100, countRows(rs));
      }
      // And max rows should be reported as 0
      assertEquals(0, stmt.getMaxRows());
    }
  }

  private static Connection openConnection() throws SQLException {
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    return DriverManager.getConnection(buildJdbcUrl(props), props);
  }

  private static int countRows(ResultSet rs) throws SQLException {
    int rows = 0;
    while (rs.next()) {
      rows++;
    }
    return rows;
  }
}
