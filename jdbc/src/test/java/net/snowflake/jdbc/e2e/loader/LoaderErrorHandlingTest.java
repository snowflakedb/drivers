package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Collections;
import java.util.Date;
import java.util.Map;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderErrorHandlingTest extends LoaderTestBase {

  @Test
  public void shouldRollBackAndThrowDataErrorWhenUpsertRowIsRejected() throws Exception {
    // Given a populated table with an extra row and a transactional upsert loader that throws
    // on error
    populateTargetTable(POPULATED_ROWS);
    try (PreparedStatement insert =
        getDefaultConnection()
            .prepareStatement(
                "INSERT INTO \""
                    + targetTable
                    + "\"(ID,C1,C2,C3,C4,C5) SELECT column1, column2, column3, column4,"
                    + " column5, parse_json(column6) FROM VALUES(?,?,?,?,?,?)")) {
      insert.setInt(1, 10001);
      insert.setString(2, "inserted\\,");
      insert.setString(3, "something");
      insert.setDouble(4, 0x4.11_33p2);
      insert.setDate(5, new java.sql.Date(new Date().getTime()));
      insert.setObject(6, "{}");
      insert.execute();
    }
    RecordingLoadResultListener listener = new RecordingLoadResultListener(true);
    Map<LoaderProperty, Object> properties =
        loaderProperties(
            targetTable, Operation.UPSERT, ALL_COLUMNS, Collections.singletonList("ID"));
    properties.put(LoaderProperty.startTransaction, true);
    properties.put(LoaderProperty.preserveStageFile, true);
    try (Loader loader = createLoader(properties, listener)) {
      loader.start();
      loader.submitRow(new Object[] {"10001", "inserted", "something", "42", new Date(), "{}"});
      loader.submitRow(new Object[] {"10002-", "inserted", "something", "42-", new Date(), "{}"});

      // When the loader finishes
      Loader.DataError error = assertThrows(Loader.DataError.class, loader::finish);

      // Then the data error names the rejected value
      assertTrue(
          error.getMessage().contains("10002-"),
          () -> "DataError should name the rejected value: " + error.getMessage());
      assertTrue(
          error.getMessage().contains("not recognized"),
          () -> "DataError should carry the conversion error: " + error.getMessage());
    } finally {
      removeUserStageFiles(targetTable);
    }

    // And nothing was merged and the valid row's change was rolled back
    assertEquals(0, listener.processed.get(), "processed rows");
    assertEquals(2, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(0, listener.updated.get(), "updated or inserted rows");
    assertEquals(2, listener.getErrorCount(), "error count");
    assertEquals(1, listener.getErrorRecordCount(), "error record count");
    assertEquals(POPULATED_ROWS + 1, countRows(targetTable), "row count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT C3 = 16.26873779296875::DOUBLE FROM \""
                    + targetTable
                    + "\" WHERE ID = 10001")) {
      assertTrue(rs.next(), "row with ID 10001 should exist");
      assertTrue(rs.getBoolean(1), "C3 after rollback should equal 0x4.1133p2");
      assertFalse(rs.wasNull(), "C3 should not be NULL");
    }
  }

  @Test
  public void shouldThrowConnectionErrorWhenStagedFileHasCorruptRows() throws Exception {
    // Given a transactional insert loader with ON_ERROR=ABORT_STATEMENT that injects a corrupt
    // line after every row
    RecordingLoadResultListener listener = new RecordingLoadResultListener(true);
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null);
    properties.put(LoaderProperty.onError, "ABORT_STATEMENT");
    properties.put(LoaderProperty.testRemoteBadCSV, true);
    properties.put(LoaderProperty.startTransaction, true);
    try (Loader loader = createLoader(properties, listener)) {
      loader.start();
      for (int i = 0; i < 100; i++) {
        loader.submitRow(defaultRow(i));
      }

      // When the loader finishes
      Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::finish);

      // Then the COPY failure surfaces with the server's SQLException and no row is loaded
      assertTrue(
          error.getMessage().startsWith("State: COPY_INTO_TABLE, "),
          () -> "ConnectionError should name the COPY state: " + error.getMessage());
      assertInstanceOf(SQLException.class, error.getCause());
    }
    assertEquals(0, countRows(targetTable), "row count");
  }

  private void removeUserStageFiles(String table) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("RM @~/" + table);
    }
  }
}
