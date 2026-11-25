package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderTargetTableTest extends LoaderTestBase {

  @Test
  public void shouldLoadIntoTableWithSpacesInTableAndColumnNames() throws Exception {
    // Given a table whose name and one column name contain spaces
    String table = "Load Test Spaces " + uniqueName("T");
    createTable(table, "ID int, \"Column 1\" varchar(255)");
    try {
      // When rows are inserted through the loader
      loadRows(table, Arrays.asList("ID", "Column 1"));

      // Then every row is stored under the quoted names
      assertRowsLoaded(table, "Column 1");
    } finally {
      dropTable(table);
    }
  }

  @Test
  public void shouldLoadSubsetOfColumnsIntoClusteredTable() throws Exception {
    // Given a table clustered by all three of its columns
    String table = uniqueName("CLUSTERED_TABLE");
    createTable(table, "ID int, \"Column1\" varchar(255), \"Column2\" varchar(255)");
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("ALTER TABLE \"" + table + "\" CLUSTER BY (ID, \"Column1\", \"Column2\")");

      // When rows are inserted for two of the three columns
      loadRows(table, Arrays.asList("ID", "Column1"));

      // Then every row is stored and the omitted column is NULL
      assertRowsLoaded(table, "Column1");
      try (ResultSet rs =
          statement.executeQuery(
              "SELECT COUNT(*) FROM \"" + table + "\" WHERE \"Column2\" IS NULL")) {
        assertTrue(rs.next(), "COUNT(*) should return a row");
        assertEquals(5, rs.getInt(1), "rows with NULL Column2");
      }
    } finally {
      dropTable(table);
    }
  }

  private void loadRows(String table, List<String> columns) throws Exception {
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(loaderProperties(table, Operation.INSERT, columns, null), listener)) {
      loader.start();
      for (int i = 0; i < 5; ++i) {
        loader.submitRow(new Object[] {i, "foo_" + i});
      }
      loader.finish();
    }
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(5, listener.operationCount.get(), "inserted rows");
  }

  private void assertRowsLoaded(String table, String stringColumn) throws Exception {
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT ID, \"" + stringColumn + "\" FROM \"" + table + "\" ORDER BY ID")) {
      for (int i = 0; i < 5; ++i) {
        assertTrue(rs.next(), "row " + i + " should exist");
        assertEquals(i, rs.getInt(1), "ID of row " + i);
        assertFalse(rs.wasNull(), "ID should not be NULL");
        assertEquals("foo_" + i, rs.getString(2), "string column of row " + i);
        assertFalse(rs.wasNull(), "string column should not be NULL");
      }
      assertFalse(rs.next(), "no other rows should exist");
    }
  }
}
