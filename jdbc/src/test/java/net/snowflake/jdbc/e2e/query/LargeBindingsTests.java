package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import net.snowflake.jdbc.utils.DriverCompatibility;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Stage (SYSTEM$BIND) array-binding coverage for the shared scenarios in {@code
 * large_bindings.feature}. Stage detection ({@code LIST @SYSTEM$BIND} count deltas) is asserted
 * only on the universal driver via {@link DriverCompatibility#isNewDriver()}; the round-trip
 * SELECTs run on both drivers.
 */
public class LargeBindingsTests extends SnowflakeIntegrationTestBase {

  private static final int DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD = 65280;

  // Threshold is session state on the shared connection; restore the default after every scenario.
  @AfterEach
  public void restoreThreshold() throws Exception {
    execute(
        getDefaultConnection(),
        "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = "
            + DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD);
  }

  @Test
  public void shouldStageBindAtTheDefaultThresholdAndReuseSystemBindAcrossConsecutiveBulkInserts()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName = createTempTable(connection, "ud_large_bindings_", "id NUMBER, name VARCHAR");

    // When 33000 rows generated as [[i, "first-" + i] for i in 0..33000] are inserted using
    // multirow
    // binding
    long beforeFirst = countSystemBindFiles(connection);
    insertNamedRows(connection, tableName, 0, 33000, "first-");

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeFirst,
          "First bulk insert should upload a bind file to SYSTEM$BIND at the default threshold");
    }

    // When 33000 rows generated as [[33000 + i, "second-" + i] for i in 0..33000] are inserted
    // using
    // multirow binding
    long beforeSecond = countSystemBindFiles(connection);
    insertNamedRows(connection, tableName, 33000, 33000, "second-");

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeSecond,
          "Second bulk insert should upload another bind file, reusing the SYSTEM$BIND stage");
    }

    // And Query "SELECT id, name FROM {table} ORDER BY id" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT id, name FROM " + tableName + " ORDER BY id")) {
      // Then Result should contain the same values as the bound parameters from both bulk inserts
      for (int id = 0; id < 66000; id++) {
        assertTrue(resultSet.next(), "Expected row for id " + id);
        assertEquals(id, resultSet.getInt(1), "Unexpected id");
        String expectedName = id < 33000 ? "first-" + id : "second-" + (id - 33000);
        assertEquals(expectedName, resultSet.getString(2), "Unexpected name for id " + id);
      }
      assertFalse(resultSet.next(), "Expected exactly 66000 rows across both bulk inserts");
    }
  }

  @Test
  public void shouldRoundTripAllBindableTypesViaStageBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR)
    // exists
    String tableName =
        createTempTable(
            connection,
            "ud_large_bindings_",
            "id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR");

    // When 13200 rows are inserted using multirow binding
    int rowCount = 13200; // 13200 x 5 columns = 66000 cells, above the default 65280 threshold
    long beforeInsert = countSystemBindFiles(connection);
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?, ?, ?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      for (int id = 0; id < rowCount; id++) {
        preparedStatement.setInt(1, id);
        preparedStatement.setInt(2, id * 7);
        preparedStatement.setDouble(3, id + 0.5d);
        preparedStatement.setBoolean(4, id % 2 == 0);
        preparedStatement.setString(5, "txt-" + id);
        preparedStatement.addBatch();
      }
      int[] counts = preparedStatement.executeBatch();
      assertEquals(rowCount, counts.length, "Expected one update count per batched row");
    }

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeInsert,
          "All-types bulk insert should upload a bind file to SYSTEM$BIND");
    }

    // And Query "SELECT id, n, f, flag, txt FROM {table} ORDER BY id" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery(
                "SELECT id, n, f, flag, txt FROM " + tableName + " ORDER BY id")) {
      // Then Result should contain the same values as the bound parameters
      for (int id = 0; id < rowCount; id++) {
        assertTrue(resultSet.next(), "Expected row for id " + id);
        assertEquals(id, resultSet.getInt(1), "Unexpected id");
        assertEquals(id * 7, resultSet.getInt(2), "Unexpected NUMBER value for id " + id);
        assertEquals(id + 0.5d, resultSet.getDouble(3), 0.0001d, "Unexpected FLOAT value");
        assertEquals(id % 2 == 0, resultSet.getBoolean(4), "Unexpected BOOLEAN value");
        assertEquals("txt-" + id, resultSet.getString(5), "Unexpected VARCHAR value");
      }
      assertFalse(resultSet.next(), "Expected exactly " + rowCount + " rows");
    }
  }

  @Test
  public void shouldPreserveCsvEscapingHazardsViaStageBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, txt VARCHAR) exists
    String tableName = createTempTable(connection, "ud_large_bindings_", "id NUMBER, txt VARCHAR");

    // comma, embedded quote, newline, backslashes, empty string, SQL NULL, multibyte UTF-8
    String[] hazards = {"val,0", "say\"1\"", "a\nb", "C:\\dir\\3", "", null, "日本語"};

    // When 33000 rows are inserted using multirow binding with values cycling every 7 rows through
    // [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6,
    // "日本語"]]
    int rowCount = 33000; // 33000 x 2 columns = 66000 cells, above the default 65280 threshold
    long beforeInsert = countSystemBindFiles(connection);
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      for (int row = 0; row < rowCount; row++) {
        int slot = row % 7;
        preparedStatement.setInt(1, slot);
        if (hazards[slot] == null) {
          preparedStatement.setNull(2, Types.VARCHAR);
        } else {
          preparedStatement.setString(2, hazards[slot]);
        }
        preparedStatement.addBatch();
      }
      int[] counts = preparedStatement.executeBatch();
      assertEquals(rowCount, counts.length, "Expected one update count per batched row");
    }

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeInsert,
          "CSV-hazard bulk insert should upload a bind file to SYSTEM$BIND");
    }

    // And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery(
                "SELECT id, txt FROM " + tableName + " WHERE id BETWEEN 0 AND 6 ORDER BY id")) {
      // Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3,
      // "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]
      boolean[] seen = new boolean[7];
      while (resultSet.next()) {
        int id = resultSet.getInt(1);
        assertTrue(id >= 0 && id <= 6, "Unexpected id outside the cycled range: " + id);
        seen[id] = true;
        String txt = resultSet.getString(2);
        if (hazards[id] == null) {
          assertNull(txt, "Expected SQL NULL for id " + id);
          assertTrue(resultSet.wasNull(), "Expected wasNull() for id " + id);
        } else {
          assertFalse(resultSet.wasNull(), "Expected non-null txt for id " + id);
          assertEquals(hazards[id], txt, "Unexpected round-tripped hazard value for id " + id);
        }
      }
      for (int id = 0; id <= 6; id++) {
        assertTrue(seen[id], "Expected at least one row for cycled id " + id);
      }
    }
  }

  @Test
  public void shouldNotStageBindScalarOrNonInsertQueriesEvenWhenThresholdIsCrossed()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 1");

    // When "SELECT ? AS val" is executed with bound integer value 42
    long beforeExecute = countSystemBindFiles(connection);
    try (PreparedStatement preparedStatement = connection.prepareStatement("SELECT ? AS val")) {
      preparedStatement.setInt(1, 42);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then the bind file on SYSTEM$BIND from the last execute should not contain the bound
        // parameter values
        if (DriverCompatibility.isNewDriver()) {
          assertEquals(
              beforeExecute,
              countSystemBindFiles(connection),
              "A scalar execute must not upload a bind file even below the threshold");
        }

        // And the result should equal 42
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(42, resultSet.getInt(1), "Unexpected scalar bound value");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldUseInlineJsonWhenRowCountIsBelowClientStageArrayBindingThreshold()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName = createTempTable(connection, "ud_large_bindings_", "id NUMBER, name VARCHAR");

    // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 100
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 100");

    // When 10 rows generated as [[i, "json-" + i] for i in 0..10] are inserted using multirow
    // binding
    long beforeInsert = countSystemBindFiles(connection);
    insertNamedRows(connection, tableName, 0, 10, "json-");

    // Then no new bind file should have been uploaded to SYSTEM$BIND
    if (DriverCompatibility.isNewDriver()) {
      assertEquals(
          beforeInsert,
          countSystemBindFiles(connection),
          "20 cells below the threshold of 100 must stay on the inline JSON path");
    }

    // And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery(
                "SELECT id, name FROM " + tableName + " WHERE id IN (0, 9) ORDER BY id")) {
      // Then Result should contain rows [[0, "json-0"], [9, "json-9"]]
      assertTrue(resultSet.next(), "Expected first matching row");
      assertEquals(0, resultSet.getInt(1), "Unexpected first id");
      assertEquals("json-0", resultSet.getString(2), "Unexpected first name");
      assertTrue(resultSet.next(), "Expected second matching row");
      assertEquals(9, resultSet.getInt(1), "Unexpected second id");
      assertEquals("json-9", resultSet.getString(2), "Unexpected second name");
      assertFalse(resultSet.next(), "Expected exactly two matching rows");
    }
  }

  @Test
  public void shouldUseStageBindingAtExactThresholdBoundary() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName = createTempTable(connection, "ud_large_bindings_", "id NUMBER, name VARCHAR");

    // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 20
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 20");

    // When 10 rows generated as [[i, "stage-" + i] for i in 0..10] are inserted using multirow
    // binding
    long beforeInsert = countSystemBindFiles(connection);
    insertNamedRows(connection, tableName, 0, 10, "stage-");

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeInsert,
          "10 rows x 2 columns == the threshold of 20 must stage-bind (cells >= threshold)");
    }

    // And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery(
                "SELECT id, name FROM " + tableName + " WHERE id IN (0, 9) ORDER BY id")) {
      // Then Result should contain rows [[0, "stage-0"], [9, "stage-9"]]
      assertTrue(resultSet.next(), "Expected first matching row");
      assertEquals(0, resultSet.getInt(1), "Unexpected first id");
      assertEquals("stage-0", resultSet.getString(2), "Unexpected first name");
      assertTrue(resultSet.next(), "Expected second matching row");
      assertEquals(9, resultSet.getInt(1), "Unexpected second id");
      assertEquals("stage-9", resultSet.getString(2), "Unexpected second name");
      assertFalse(resultSet.next(), "Expected exactly two matching rows");
    }
  }

  @Test
  public void shouldKeepAnAllNullRowOnTheInlineJsonPathWhenStageBindingIsDisabled()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD
    // NUMBER, colE INTEGER) exists
    String tableName = createAllNullableColumnsTable(connection);

    // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 0
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 0");

    // When a batch of one row with every column set to SQL NULL is inserted using multirow binding
    long beforeInsert = countSystemBindFiles(connection);
    insertAllNullRow(connection, tableName);

    // Then no new bind file should have been uploaded to SYSTEM$BIND
    if (DriverCompatibility.isNewDriver()) {
      assertEquals(
          beforeInsert,
          countSystemBindFiles(connection),
          "A disabled threshold (0) must keep the all-NULL batch on the inline JSON path");
    }

    // And every column of the round-tripped row reads back as SQL NULL
    assertRowIsAllNull(connection, tableName);
  }

  @Test
  public void shouldStageBindAnAllNullRowWhenTheBoundCellCountMeetsTheThreshold() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD
    // NUMBER, colE INTEGER) exists
    String tableName = createAllNullableColumnsTable(connection);

    // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 6
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 6");

    // When a batch of one row with every column set to SQL NULL is inserted using multirow binding
    long beforeInsert = countSystemBindFiles(connection);
    insertAllNullRow(connection, tableName);

    // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as
    // the bound parameters
    if (DriverCompatibility.isNewDriver()) {
      assertTrue(
          countSystemBindFiles(connection) > beforeInsert,
          "6 bound cells == the threshold of 6 must stage-bind the all-NULL row to SYSTEM$BIND");
    }

    // And every column of the round-tripped row reads back as SQL NULL
    assertRowIsAllNull(connection, tableName);
  }

  private String createAllNullableColumnsTable(Connection connection) throws Exception {
    return createTempTable(
        connection,
        "ud_large_bindings_null_",
        "id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER");
  }

  private void insertAllNullRow(Connection connection, String tableName) throws Exception {
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?, ?, ?, ?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      preparedStatement.setNull(1, Types.INTEGER);
      preparedStatement.setNull(2, Types.DOUBLE);
      preparedStatement.setNull(3, Types.FLOAT);
      preparedStatement.setNull(4, Types.VARCHAR);
      preparedStatement.setNull(5, Types.NUMERIC);
      preparedStatement.setNull(6, Types.INTEGER);
      preparedStatement.addBatch();
      int[] counts = preparedStatement.executeBatch();
      assertEquals(1, counts.length, "Expected exactly one batched row");
      assertEquals(1, counts[0], "Expected one row inserted");
    }
  }

  private void assertRowIsAllNull(Connection connection, String tableName) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT id, colA, colB, colC, colD, colE FROM " + tableName)) {
      assertTrue(resultSet.next(), "Expected the inserted all-NULL row");
      resultSet.getInt(1);
      assertTrue(resultSet.wasNull(), "id should read back as SQL NULL");
      resultSet.getDouble(2);
      assertTrue(resultSet.wasNull(), "colA should read back as SQL NULL");
      resultSet.getFloat(3);
      assertTrue(resultSet.wasNull(), "colB should read back as SQL NULL");
      resultSet.getString(4);
      assertTrue(resultSet.wasNull(), "colC should read back as SQL NULL");
      resultSet.getLong(5);
      assertTrue(resultSet.wasNull(), "colD should read back as SQL NULL");
      resultSet.getInt(6);
      assertTrue(resultSet.wasNull(), "colE should read back as SQL NULL");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  private void insertNamedRows(
      Connection connection, String tableName, int idStart, int count, String namePrefix)
      throws Exception {
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      for (int offset = 0; offset < count; offset++) {
        preparedStatement.setInt(1, idStart + offset);
        preparedStatement.setString(2, namePrefix + offset);
        preparedStatement.addBatch();
      }
      int[] counts = preparedStatement.executeBatch();
      assertEquals(count, counts.length, "Expected one update count per batched row");
    }
  }

  // Snowflake vendor code for "Object ... does not exist or not authorized" — what LIST returns
  // before the lazily-created @SYSTEM$BIND stage exists.
  private static final int OBJECT_DOES_NOT_EXIST_VENDOR_CODE = 2003;

  // Files staged under @SYSTEM$BIND. The stage is created lazily on the first stage bind, so LIST
  // fails with "does not exist" before that — treat only that case as zero so callers can compare
  // deltas. Any other failure (expired session, missing warehouse, permission, network) must
  // propagate: swallowing it would let a broken LIST read as an empty stage and silently pass the
  // negative staging assertions.
  private long countSystemBindFiles(Connection connection) throws SQLException {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("LIST @SYSTEM$BIND")) {
      long count = 0;
      while (resultSet.next()) {
        count++;
      }
      return count;
    } catch (SQLException e) {
      if (e.getErrorCode() == OBJECT_DOES_NOT_EXIST_VENDOR_CODE) {
        return 0;
      }
      throw e;
    }
  }
}
