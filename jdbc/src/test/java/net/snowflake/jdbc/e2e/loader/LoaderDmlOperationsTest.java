package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Arrays;
import java.util.Calendar;
import java.util.Collections;
import java.util.Date;
import java.util.TimeZone;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderDmlOperationsTest extends LoaderTestBase {

  @Test
  public void shouldDeleteOnlyRowsMatchingAllKeys() throws Exception {
    // Given a populated table and a delete loader keyed on ID and C1
    populateTargetTable(POPULATED_ROWS);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(
                targetTable,
                Operation.DELETE,
                Arrays.asList("ID", "C1"),
                Arrays.asList("ID", "C1")),
            listener)) {
      loader.start();

      // When one matching key pair and one non-matching key pair are submitted
      loader.submitRow(new Object[] {42, "foo_42"});
      loader.submitRow(new Object[] {41, "blah"});
      loader.finish();
    }

    // Then only the matching row is deleted and no error is reported
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(0, listener.getErrorRecordCount(), "error record count");
    assertEquals(2, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(1, listener.processed.get(), "processed rows");
    assertEquals(1, listener.deleted.get(), "deleted rows");
    assertEquals(POPULATED_ROWS - 1, countRows(targetTable), "row count");
  }

  @Test
  public void shouldModifyMatchingRowsWithValuesThatNeedCsvQuoting() throws Exception {
    // Given a populated table and a modify loader keyed on ID
    populateTargetTable(POPULATED_ROWS);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(
                targetTable, Operation.MODIFY, ALL_COLUMNS, Collections.singletonList("ID")),
            listener)) {
      loader.start();

      // When two existing rows are modified with values holding newlines, quotes, backslashes
      // and commas
      loader.submitRow(new Object[] {41, "modified", "some\nthi\"ng\\", 41.6, new Date(), "{}"});
      loader.submitRow(new Object[] {40, "modified", "\"something,", 40.2, new Date(), "{}"});
      loader.finish();
    }

    // Then both rows are updated with the exact values
    assertEquals(2, listener.processed.get(), "processed rows");
    assertEquals(2, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(2, listener.updated.get(), "updated rows");
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(0, listener.getErrorRecordCount(), "error record count");
    assertEquals(POPULATED_ROWS, countRows(targetTable), "row count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT ID, C1, C2, C3 FROM \""
                    + targetTable
                    + "\" WHERE ID IN (40, 41) ORDER BY ID")) {
      assertTrue(rs.next(), "row with ID 40 should exist");
      assertEquals("modified", rs.getString("C1"), "C1 of row 40");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertEquals("\"something,", rs.getString("C2"), "C2 of row 40");
      assertFalse(rs.wasNull(), "C2 should not be NULL");
      assertEquals(40.2, rs.getDouble("C3"), 0.0, "C3 of row 40");
      assertFalse(rs.wasNull(), "C3 should not be NULL");
      assertTrue(rs.next(), "row with ID 41 should exist");
      assertEquals("modified", rs.getString("C1"), "C1 of row 41");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertEquals("some\nthi\"ng\\", rs.getString("C2"), "C2 of row 41");
      assertFalse(rs.wasNull(), "C2 should not be NULL");
      assertEquals(41.6, rs.getDouble("C3"), 0.0, "C3 of row 41");
      assertFalse(rs.wasNull(), "C3 should not be NULL");
    }
  }

  @Test
  public void shouldModifyOnlyRowsWhoseKeysMatch() throws Exception {
    // Given a populated table and a modify loader keyed on ID
    populateTargetTable(POPULATED_ROWS);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(
                targetTable, Operation.MODIFY, ALL_COLUMNS, Collections.singletonList("ID")),
            listener)) {
      loader.start();

      // When one row with an unknown key and one row with an existing key are modified
      loader.submitRow(new Object[] {20000, "modified", "some\nthi\"ng\\", 41.6, new Date(), "{}"});
      loader.submitRow(new Object[] {45, "modified", "\"something2,", 40.2, new Date(), "{}"});
      loader.finish();
    }

    // Then only the existing row is updated and no row is inserted
    assertEquals(1, listener.processed.get(), "processed rows");
    assertEquals(2, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(1, listener.updated.get(), "updated rows");
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(0, listener.getErrorRecordCount(), "error record count");
    assertEquals(POPULATED_ROWS, countRows(targetTable), "row count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery("SELECT C1, C2 FROM \"" + targetTable + "\" WHERE ID = 45")) {
      assertTrue(rs.next(), "row with ID 45 should exist");
      assertEquals("modified", rs.getString("C1"), "C1 of row 45");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertEquals("\"something2,", rs.getString("C2"), "C2 of row 45");
      assertFalse(rs.wasNull(), "C2 should not be NULL");
    }
  }

  @Test
  public void shouldInsertNewAndUpdateExistingRowsOnUpsert() throws Exception {
    // Given a populated table and an upsert loader keyed on ID
    populateTargetTable(POPULATED_ROWS);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    Date timestamp = new Date();
    try (Loader loader =
        createLoader(
            loaderProperties(
                targetTable, Operation.UPSERT, ALL_COLUMNS, Collections.singletonList("ID")),
            listener)) {
      loader.start();

      // When one new row and one existing row are upserted
      loader.submitRow(
          new Object[] {10001, "inserted\\,", "something", 0x4.11_33p2, timestamp, "{}"});
      loader.submitRow(new Object[] {39, "modified", "something", 40.1, timestamp, "{}"});
      loader.finish();
    }

    // Then the new row is inserted with exact values and the existing row is updated
    assertEquals(2, listener.processed.get(), "processed rows");
    assertEquals(2, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(2, listener.updated.get(), "updated or inserted rows");
    assertEquals(0, listener.getErrorCount(), "error count");
    assertEquals(0, listener.getErrorRecordCount(), "error record count");
    Calendar utc = Calendar.getInstance(TimeZone.getTimeZone("UTC"));
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT ID, C1, C3 = 16.26873779296875::DOUBLE AS C3_EXACT, C4 FROM \""
                    + targetTable
                    + "\" WHERE ID IN (39, 10001) ORDER BY ID")) {
      assertTrue(rs.next(), "row with ID 39 should exist");
      assertEquals("modified", rs.getString("C1"), "C1 of the updated row");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertTrue(rs.next(), "row with ID 10001 should exist");
      assertEquals("inserted\\,", rs.getString("C1"), "C1 of the inserted row");
      assertFalse(rs.wasNull(), "C1 should not be NULL");
      assertTrue(rs.getBoolean("C3_EXACT"), "C3 of the inserted row should equal 0x4.1133p2");
      assertFalse(rs.wasNull(), "C3 should not be NULL");
      assertEquals(
          timestamp.getTime(), rs.getTimestamp("C4", utc).getTime(), "C4 of the inserted row");
      assertFalse(rs.wasNull(), "C4 should not be NULL");
    }
  }
}
