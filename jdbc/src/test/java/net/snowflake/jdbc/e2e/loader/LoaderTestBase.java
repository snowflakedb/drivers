package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderFactory;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;

/**
 * Creates a fresh target table for each test with the column layout the old driver's loader tests
 * use, and builds loaders on two dedicated connections that {@link Loader#close()} closes.
 */
abstract class LoaderTestBase extends SnowflakeIntegrationTestBase {

  static final List<String> DEFAULT_COLUMNS = Arrays.asList("ID", "C1", "C3", "C4", "C5");

  static final List<String> ALL_COLUMNS = Arrays.asList("ID", "C1", "C2", "C3", "C4", "C5");

  static final int POPULATED_ROWS = 1000;

  String targetTable;

  @BeforeEach
  void createTargetTable() throws SQLException {
    targetTable = uniqueName("LOADER_TEST");
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute(
          "CREATE OR REPLACE TABLE \""
              + targetTable
              + "\" (ID int, C1 varchar(255), C2 varchar(255) DEFAULT 'X', C3 double,"
              + " C4 timestamp, C5 variant)");
    }
  }

  @AfterEach
  void dropTargetTable() throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("DROP TABLE IF EXISTS \"" + targetTable + "\"");
    }
  }

  static String uniqueName(String prefix) {
    return prefix + "_" + UUID.randomUUID().toString().replace("-", "").toUpperCase();
  }

  Map<LoaderProperty, Object> loaderProperties(
      String table, Operation operation, List<String> columns, List<String> keys)
      throws SQLException {
    Connection connection = getDefaultConnection();
    Map<LoaderProperty, Object> properties = new HashMap<>();
    properties.put(LoaderProperty.tableName, table);
    properties.put(LoaderProperty.schemaName, connection.getSchema());
    properties.put(LoaderProperty.databaseName, connection.getCatalog());
    properties.put(LoaderProperty.operation, operation);
    properties.put(LoaderProperty.columns, columns);
    properties.put(LoaderProperty.keys, keys);
    return properties;
  }

  Loader createLoader(Map<LoaderProperty, Object> properties, RecordingLoadResultListener listener)
      throws SQLException {
    Loader loader = LoaderFactory.createLoader(properties, openConnection(), openConnection());
    loader.setListener(listener);
    return loader;
  }

  /** Inserts {@code rows} rows of the form {i, "foo_i", i * 1.5, now, {"bar": i}}. */
  void populateTargetTable(int rows) throws Exception {
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader =
        createLoader(
            loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null), listener)) {
      loader.start();
      for (int i = 0; i < rows; i++) {
        loader.submitRow(defaultRow(i));
      }
      loader.finish();
    }
    assertEquals(0, listener.getErrorCount(), "Populating the target table reported errors");
    assertEquals(rows, countRows(targetTable), "Populating the target table lost rows");
  }

  static Object[] defaultRow(int i) {
    return new Object[] {i, "foo_" + i, i * 1.5, new java.util.Date(), "{\"bar\":" + i + "}"};
  }

  int countRows(String table) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs = statement.executeQuery("SELECT COUNT(*) FROM \"" + table + "\"")) {
      assertTrue(rs.next(), "COUNT(*) should return a row");
      return rs.getInt(1);
    }
  }
}
