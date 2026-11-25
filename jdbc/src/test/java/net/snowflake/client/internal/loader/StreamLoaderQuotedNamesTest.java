package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.Collections;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * snowflake-jdbc 4.x sends these names unescaped and the load fails with a SQL syntax error, so
 * these tests do not run against the old driver.
 */
public class StreamLoaderQuotedNamesTest extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldLoadIntoTableWhoseNameContainsDoubleQuote() throws Exception {
    // Given a table whose name contains a double quote
    String table = "LOADER_Q\"UOTE_" + UUID.randomUUID().toString().replace("-", "");
    String quotedTable = StreamLoader.quoteIdentifier(table);
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("CREATE OR REPLACE TABLE " + quotedTable + " (ID int, C1 varchar(255))");
      Map<LoaderProperty, Object> properties =
          properties(table, Operation.INSERT, Arrays.asList("ID", "C1"), null);

      // When rows are loaded
      CountingLoadResultListener listener = new CountingLoadResultListener(true);
      load(properties, listener, new Object[] {1, "a"}, new Object[] {2, "b"});

      // Then they are in the table
      assertEquals(0, listener.getErrorCount(), "error count");
      assertEquals(2, countRows(statement, quotedTable), "row count");
    } finally {
      execute("DROP TABLE IF EXISTS " + quotedTable);
    }
  }

  @Test
  public void shouldUpsertWithColumnAndKeyNamesThatContainDoubleQuote() throws Exception {
    // Given a table whose key column and a data column contain a double quote
    String table = "LOADER_QUOTED_COLUMNS_" + UUID.randomUUID().toString().replace("-", "");
    String quotedTable = StreamLoader.quoteIdentifier(table);
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute(
          "CREATE OR REPLACE TABLE "
              + quotedTable
              + " (\"I\"\"D\" int, \"C\"\"1\" varchar(255), C2 varchar(255))");
      statement.execute("INSERT INTO " + quotedTable + " VALUES (1, 'old', 'kept')");
      Map<LoaderProperty, Object> properties =
          properties(
              table,
              Operation.UPSERT,
              Arrays.asList("I\"D", "C\"1"),
              Collections.singletonList("I\"D"));

      // When one existing and one new row are upserted
      CountingLoadResultListener listener = new CountingLoadResultListener(true);
      load(properties, listener, new Object[] {1, "new"}, new Object[] {2, "added"});

      // Then the existing row is updated, the new row is inserted and other columns are untouched
      assertEquals(0, listener.getErrorCount(), "error count");
      try (ResultSet rs =
          statement.executeQuery(
              "SELECT \"C\"\"1\", C2 FROM " + quotedTable + " ORDER BY \"I\"\"D\"")) {
        assertTrue(rs.next(), "row 1 should exist");
        assertEquals("new", rs.getString(1), "updated value");
        assertEquals("kept", rs.getString(2), "column that was not loaded");
        assertTrue(rs.next(), "row 2 should exist");
        assertEquals("added", rs.getString(1), "inserted value");
      }
    } finally {
      execute("DROP TABLE IF EXISTS " + quotedTable);
    }
  }

  @Test
  public void shouldDeleteWithKeyNameThatContainsDoubleQuote() throws Exception {
    // Given a table with a key column that contains a double quote and two rows
    String table = "LOADER_QUOTED_KEY_" + UUID.randomUUID().toString().replace("-", "");
    String quotedTable = StreamLoader.quoteIdentifier(table);
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("CREATE OR REPLACE TABLE " + quotedTable + " (\"I\"\"D\" int)");
      statement.execute("INSERT INTO " + quotedTable + " VALUES (1), (2)");
      Map<LoaderProperty, Object> properties =
          properties(
              table,
              Operation.DELETE,
              Collections.singletonList("I\"D"),
              Collections.singletonList("I\"D"));

      // When one key is deleted
      CountingLoadResultListener listener = new CountingLoadResultListener(true);
      load(properties, listener, new Object[] {1});

      // Then only the other row remains
      assertEquals(0, listener.getErrorCount(), "error count");
      assertEquals(1, countRows(statement, quotedTable), "row count");
    } finally {
      execute("DROP TABLE IF EXISTS " + quotedTable);
    }
  }

  private Map<LoaderProperty, Object> properties(
      String table, Operation operation, List<String> columns, List<String> keys)
      throws SQLException {
    Connection connection = getDefaultConnection();
    Map<LoaderProperty, Object> properties = new EnumMap<>(LoaderProperty.class);
    properties.put(LoaderProperty.tableName, table);
    properties.put(LoaderProperty.schemaName, connection.getSchema());
    properties.put(LoaderProperty.databaseName, connection.getCatalog());
    properties.put(LoaderProperty.operation, operation);
    properties.put(LoaderProperty.columns, columns);
    properties.put(LoaderProperty.keys, keys);
    return properties;
  }

  private void load(
      Map<LoaderProperty, Object> properties, CountingLoadResultListener listener, Object[]... rows)
      throws Exception {
    try (StreamLoader loader = new StreamLoader(properties, openConnection(), openConnection())) {
      loader.setListener(listener);
      loader.start();
      for (Object[] row : rows) {
        loader.submitRow(row);
      }
      loader.finish();
    }
  }

  private void execute(String sql) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute(sql);
    }
  }

  private static int countRows(Statement statement, String quotedTable) throws SQLException {
    try (ResultSet rs = statement.executeQuery("SELECT COUNT(*) FROM " + quotedTable)) {
      assertTrue(rs.next(), "COUNT(*) should return a row");
      return rs.getInt(1);
    }
  }
}
