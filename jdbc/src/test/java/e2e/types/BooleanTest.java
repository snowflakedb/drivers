package e2e.types;

import static org.junit.jupiter.api.Assertions.*;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.HashSet;
import java.util.Set;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/** BOOLEAN type tests for JDBC wrapper. */
public class BooleanTest extends SnowflakeIntegrationTestBase {

  private static final int LARGE_RESULT_SET_SIZE = 1_000_000;
  private Connection connection;
  private String testSchema;

  @BeforeEach
  public void setUp() throws Exception {
    connection = openConnection();
    ensureDatabaseAndSchema(connection);
    testSchema = loadConnectionProperties().getProperty("schema");
  }

  @AfterEach
  public void tearDown() throws Exception {
    if (connection != null && !connection.isClosed()) {
      connection.close();
    }
  }

  // ===========================================================================
  // Type casting
  // ===========================================================================

  @Test
  public void shouldCastBooleanValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in

    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
    try (Statement stmt = connection.createStatement()) {
      ResultSet rs = stmt.executeQuery("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN");

      // Then All values should be returned as appropriate type
      assertTrue(rs.next());
      boolean val1 = rs.getBoolean(1);
      boolean val2 = rs.getBoolean(2);
      boolean val3 = rs.getBoolean(3);

      // And Values should match [TRUE, FALSE, TRUE]
      assertTrue(val1);
      assertFalse(val2);
      assertTrue(val3);

      assertFalse(rs.next());
    }
  }

  // ===========================================================================
  // SELECT with literals (no tables)
  // ===========================================================================

  @Test
  public void shouldSelectBooleanLiterals() throws Exception {
    // Given Snowflake client is logged in

    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    try (Statement stmt = connection.createStatement()) {
      ResultSet rs = stmt.executeQuery("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

      // Then Result should contain [TRUE, FALSE]
      assertTrue(rs.next());
      assertTrue(rs.getBoolean(1));
      assertFalse(rs.getBoolean(2));

      assertFalse(rs.next());
    }
  }

  @Test
  public void shouldHandleNullValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in

    // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
    try (Statement stmt = connection.createStatement()) {
      ResultSet rs =
          stmt.executeQuery("SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN");

      // Then Result should contain [FALSE, NULL, TRUE, NULL]
      assertTrue(rs.next());

      assertFalse(rs.getBoolean(1));
      assertFalse(rs.wasNull());

      rs.getBoolean(2);
      assertTrue(rs.wasNull());

      assertTrue(rs.getBoolean(3));
      assertFalse(rs.wasNull());

      rs.getBoolean(4);
      assertTrue(rs.wasNull());

      assertFalse(rs.next());
    }
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGenerator() throws Exception {
    // Given Snowflake client is logged in

    // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
    String sql =
        String.format(
            "SELECT (seq8() %% 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => %d))",
            LARGE_RESULT_SET_SIZE);

    try (Statement stmt = connection.createStatement()) {
      ResultSet rs = stmt.executeQuery(sql);

      // Then Result should contain 500000 TRUE and 500000 FALSE values
      int total = 0;
      int numTrue = 0;

      while (rs.next()) {
        total++;
        if (rs.getBoolean(1)) {
          numTrue++;
        }
      }

      assertEquals(LARGE_RESULT_SET_SIZE, total);
      assertEquals(LARGE_RESULT_SET_SIZE / 2, numTrue);
    }
  }

  // ===========================================================================
  // Table operations
  // ===========================================================================

  @Test
  public void shouldSelectBooleanValuesFromTable() throws Exception {
    // Given Snowflake client is logged in

    // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
    String tableName = testSchema + ".boolean_table_" + System.currentTimeMillis();
    try (Statement stmt = connection.createStatement()) {
      stmt.execute(
          String.format(
              "CREATE TEMPORARY TABLE %s (col1 BOOLEAN, col2 BOOLEAN, col3 BOOLEAN)", tableName));

      // And Row (TRUE, FALSE, TRUE) is inserted
      stmt.execute(String.format("INSERT INTO %s VALUES (TRUE, FALSE, TRUE)", tableName));

      // When Query "SELECT * FROM <table>" is executed
      ResultSet rs = stmt.executeQuery(String.format("SELECT * FROM %s", tableName));

      // Then Result should contain [TRUE, FALSE, TRUE]
      assertTrue(rs.next());
      assertTrue(rs.getBoolean(1));
      assertFalse(rs.getBoolean(2));
      assertTrue(rs.getBoolean(3));

      assertFalse(rs.next());
    }
  }

  @Test
  public void shouldHandleNullValuesFromTable() throws Exception {
    // Given Snowflake client is logged in

    // And Table with BOOLEAN column exists
    String tableName = testSchema + ".null_table_" + System.currentTimeMillis();
    try (Statement stmt = connection.createStatement()) {
      stmt.execute(String.format("CREATE TEMPORARY TABLE %s (col BOOLEAN)", tableName));

      // And Rows [NULL, TRUE, FALSE] are inserted
      stmt.execute(String.format("INSERT INTO %s VALUES (NULL), (TRUE), (FALSE)", tableName));

      // When Query "SELECT * FROM <table>" is executed
      ResultSet rs = stmt.executeQuery(String.format("SELECT * FROM %s", tableName));

      // Then Result should contain [NULL, TRUE, FALSE] in any order
      Set<Boolean> result = new HashSet<>();
      boolean hasNull = false;

      while (rs.next()) {
        boolean val = rs.getBoolean(1);
        if (rs.wasNull()) {
          hasNull = true;
        } else {
          result.add(val);
        }
      }

      assertTrue(hasNull);
      assertTrue(result.contains(true));
      assertTrue(result.contains(false));
      assertEquals(2, result.size());
    }
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in

    // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
    String tableName = testSchema + ".large_boolean_table_" + System.currentTimeMillis();
    try (Statement stmt = connection.createStatement()) {
      stmt.execute(String.format("CREATE TEMPORARY TABLE %s (col BOOLEAN)", tableName));

      String insertSql =
          String.format(
              "INSERT INTO %s SELECT (seq8() %% 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => %d))",
              tableName, LARGE_RESULT_SET_SIZE);
      stmt.execute(insertSql);

      // When Query "SELECT col FROM <table>" is executed
      ResultSet rs = stmt.executeQuery(String.format("SELECT col FROM %s", tableName));

      // Then Result should contain 500000 TRUE and 500000 FALSE values
      int total = 0;
      int numTrue = 0;

      while (rs.next()) {
        total++;
        if (rs.getBoolean(1)) {
          numTrue++;
        }
      }

      assertEquals(LARGE_RESULT_SET_SIZE, total);
      assertEquals(LARGE_RESULT_SET_SIZE / 2, numTrue);
    }
  }

  // ===========================================================================
  // Parameter binding
  // ===========================================================================

  @Test
  public void shouldSelectBooleanUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in

    // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values
    // [TRUE, FALSE, TRUE]
    try (java.sql.PreparedStatement pstmt =
        connection.prepareStatement("SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN")) {
      pstmt.setBoolean(1, true);
      pstmt.setBoolean(2, false);
      pstmt.setBoolean(3, true);

      ResultSet rs = pstmt.executeQuery();

      // Then Result should contain [TRUE, FALSE, TRUE]
      assertTrue(rs.next());
      assertTrue(rs.getBoolean(1));
      assertFalse(rs.getBoolean(2));
      assertTrue(rs.getBoolean(3));

      assertFalse(rs.next());
    }

    // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
    try (java.sql.PreparedStatement pstmt = connection.prepareStatement("SELECT ?::BOOLEAN")) {
      pstmt.setNull(1, java.sql.Types.BOOLEAN);

      ResultSet rs = pstmt.executeQuery();

      // Then Result should contain [NULL]
      assertTrue(rs.next());
      rs.getBoolean(1);
      assertTrue(rs.wasNull());

      assertFalse(rs.next());
    }
  }

  @Test
  public void shouldInsertBooleanUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in

    // And Table with BOOLEAN column exists
    String tableName = testSchema + ".boolean_bind_table_" + System.currentTimeMillis();
    try (Statement stmt = connection.createStatement()) {
      stmt.execute(String.format("CREATE TEMPORARY TABLE %s (col BOOLEAN)", tableName));

      // When Boolean values [TRUE, FALSE, NULL] are inserted using binding
      try (java.sql.PreparedStatement pstmt =
          connection.prepareStatement(String.format("INSERT INTO %s VALUES (?)", tableName))) {
        pstmt.setBoolean(1, true);
        pstmt.executeUpdate();

        pstmt.setBoolean(1, false);
        pstmt.executeUpdate();

        pstmt.setNull(1, java.sql.Types.BOOLEAN);
        pstmt.executeUpdate();
      }

      // Then SELECT should return the same values in any order
      ResultSet rs = stmt.executeQuery(String.format("SELECT * FROM %s", tableName));

      Set<Boolean> result = new HashSet<>();
      boolean hasNull = false;

      while (rs.next()) {
        boolean val = rs.getBoolean(1);
        if (rs.wasNull()) {
          hasNull = true;
        } else {
          result.add(val);
        }
      }

      assertTrue(hasNull);
      assertTrue(result.contains(true));
      assertTrue(result.contains(false));
      assertEquals(2, result.size());
    }
  }
}
