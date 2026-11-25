package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Collections;
import java.util.Date;
import java.util.Map;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.LoadingError;
import net.snowflake.client.api.loader.LoadingError.ErrorProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderBasicOperationsTest extends LoaderTestBase {

  @Test
  public void shouldInsertSubmittedRowsAndReportCounts() throws Exception {
    // Given a loader that inserts into an empty table
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null), listener)) {

      // When rows are submitted and the loader finishes
      loader.start();
      for (int i = 0; i < POPULATED_ROWS; i++) {
        loader.submitRow(defaultRow(i));
      }
      loader.finish();
    }

    // Then the listener counts every row as submitted, processed and inserted without errors
    assertEquals(POPULATED_ROWS, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(POPULATED_ROWS, listener.processed.get(), "processed rows");
    assertEquals(POPULATED_ROWS, listener.operationCount.get(), "inserted rows");
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(0, listener.getErrorRecordCount(), "error record count");
    assertEquals(POPULATED_ROWS - 1, listener.getLastRecord()[0], "last provided record");

    // And the table holds every row with its values and the column default for C2
    assertEquals(POPULATED_ROWS, countRows(targetTable), "row count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT C1, C2, C3, C5:bar::int FROM \"" + targetTable + "\" WHERE ID = 7")) {
      assertTrue(rs.next(), "row with ID 7 should exist");
      assertEquals("foo_7", rs.getString(1), "C1");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertEquals("X", rs.getString(2), "C2 should take the column default");
      assertFalse(rs.wasNull(), "C2 should not be NULL");
      assertEquals(10.5, rs.getDouble(3), 0.0, "C3");
      assertFalse(rs.wasNull(), "C3 should not be NULL");
      assertEquals(7, rs.getInt(4), "C5:bar");
      assertFalse(rs.wasNull(), "C5:bar should not be NULL");
    }
  }

  @Test
  public void shouldThrowConnectionErrorWhenCopyAbortsOnInvalidValue() throws Exception {
    // Given a loader with ON_ERROR=ABORT_STATEMENT and a listener that throws on error
    RecordingLoadResultListener listener = new RecordingLoadResultListener(true);
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null);
    properties.put(LoaderProperty.onError, "ABORT_STATEMENT");
    try (Loader loader = createLoader(properties, listener)) {
      loader.start();
      for (int i = 0; i < 10; i++) {
        Object[] row = defaultRow(i);
        if (i == 7) {
          row[2] = "INVALID_INTEGER";
        }
        loader.submitRow(row);
      }

      // When the loader finishes
      Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::finish);

      // Then the COPY failure surfaces with the server's error for the rejected value
      assertTrue(
          error.getMessage().startsWith("State: COPY_INTO_TABLE, "),
          () -> "ConnectionError should name the COPY state: " + error.getMessage());
      assertTrue(
          error.getMessage().contains("INVALID_INTEGER"),
          () -> "ConnectionError should name the rejected value: " + error.getMessage());
      SQLException cause = assertInstanceOf(SQLException.class, error.getCause());
      assertEquals("22018", cause.getSQLState(), "SQLState of the conversion error");
      assertEquals(100038, cause.getErrorCode(), "vendor code of the conversion error");
    }
  }

  @Test
  public void shouldReportLoadingErrorsAndApplyValidRowsOnUpsert() throws Exception {
    // Given a populated table and an upsert loader keyed on ID
    populateTargetTable(POPULATED_ROWS);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(
                targetTable, Operation.UPSERT, ALL_COLUMNS, Collections.singletonList("ID")),
            listener)) {
      loader.start();

      // When one invalid row, one new row and one existing row are upserted
      loader.submitRow(new Object[] {"10001-", "inserted", "something", "42-", new Date(), "{}"});
      loader.submitRow(new Object[] {10002, "inserted", "something", 43, new Date(), "{}"});
      loader.submitRow(new Object[] {45, "modified", "something", 46.1, new Date(), "{}"});
      loader.finish();
    }

    // Then the listener reports one rejected record with an error for each of its bad values
    assertEquals(3, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(3, listener.processed.get(), "processed rows");
    assertEquals(2, listener.operationCount.get(), "merged rows");
    assertEquals(2, listener.updated.get(), "updated or inserted rows");
    assertEquals(2, listener.getErrorCount(), "error count");
    assertEquals(1, listener.getErrorRecordCount(), "error record count");
    assertEquals(2, listener.getErrors().size(), "errors passed to addError");
    for (LoadingError error : listener.getErrors()) {
      assertEquals(targetTable, error.getTarget(), "LoadingError target");
      assertEquals("1", error.getProperty(ErrorProperty.LINE), "LoadingError line");
      assertTrue(
          error.getStage().startsWith(targetTable + "/UPSERT/"),
          () -> "LoadingError stage: " + error.getStage());
      assertTrue(
          error.getFile().startsWith("/stream_"), () -> "LoadingError file: " + error.getFile());
      assertTrue(
          error.getException().getMessage().startsWith("{\"ERROR\": \""),
          () -> "LoadingError exception message: " + error.getException().getMessage());
    }

    // And the valid rows were applied
    assertEquals(POPULATED_ROWS + 1, countRows(targetTable), "row count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT ID, C1 FROM \"" + targetTable + "\" WHERE ID IN (45, 10002) ORDER BY ID")) {
      assertTrue(rs.next(), "row with ID 45 should exist");
      assertEquals("modified", rs.getString(2), "C1 of the updated row");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertTrue(rs.next(), "row with ID 10002 should exist");
      assertEquals("inserted", rs.getString(2), "C1 of the inserted row");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertFalse(rs.next(), "no other rows should match");
    }
  }

  @Test
  public void shouldThrowConnectionErrorWithSqlCauseWhenExecuteBeforeFails() throws Exception {
    // Given a loader whose executeBefore SQL does not compile
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null);
    properties.put(LoaderProperty.onError, "ABORT_STATEMENT");
    properties.put(LoaderProperty.executeBefore, "SELECT * FROOOOOM TBL");
    try (Loader loader = createLoader(properties, new RecordingLoadResultListener())) {
      loader.start();

      // When the loader finishes
      Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::finish);

      // Then the error names the failed SQL and carries the server's SQLException
      assertEquals("Execute Before SQL failed to run: SELECT * FROOOOOM TBL", error.getMessage());
      SQLException cause = assertInstanceOf(SQLException.class, error.getCause());
      assertEquals("42000", cause.getSQLState(), "SQLState of the syntax error");
      assertEquals(1003, cause.getErrorCode(), "vendor code of the syntax error");
    }
  }

  @Test
  public void shouldThrowConnectionErrorWithSqlCauseWhenExecuteAfterFails() throws Exception {
    // Given a loader whose executeAfter SQL references a missing table
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null);
    properties.put(LoaderProperty.onError, "ABORT_STATEMENT");
    properties.put(LoaderProperty.executeBefore, "select current_version()");
    properties.put(LoaderProperty.executeAfter, "SELECT * FROM " + uniqueName("MISSING_TABLE"));
    try (Loader loader = createLoader(properties, new RecordingLoadResultListener())) {
      loader.start();

      // When the loader finishes
      Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::finish);

      // Then the error carries the server's SQLException for the missing object
      SQLException cause = assertInstanceOf(SQLException.class, error.getCause());
      assertEquals("42S02", cause.getSQLState(), "SQLState of the missing object error");
      assertEquals(2003, cause.getErrorCode(), "vendor code of the missing object error");
    }
  }
}
