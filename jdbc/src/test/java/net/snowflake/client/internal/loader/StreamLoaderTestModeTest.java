package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.EnumMap;
import java.util.Map;
import java.util.UUID;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class StreamLoaderTestModeTest extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldLoadAllRowsAfterSimulatedUploadFailures() throws Exception {
    // Given a loader in test mode, where the first two PUT attempts of every file fail
    String table = "LOADER_TEST_MODE_" + UUID.randomUUID().toString().replace("-", "");
    execute("CREATE OR REPLACE TABLE \"" + table + "\" (ID int, C1 varchar(255))");
    try {
      Connection connection = getDefaultConnection();
      Map<LoaderProperty, Object> properties = new EnumMap<>(LoaderProperty.class);
      properties.put(LoaderProperty.tableName, table);
      properties.put(LoaderProperty.schemaName, connection.getSchema());
      properties.put(LoaderProperty.databaseName, connection.getCatalog());
      properties.put(LoaderProperty.operation, Operation.INSERT);
      properties.put(LoaderProperty.columns, Arrays.asList("ID", "C1"));
      CountingLoadResultListener listener = new CountingLoadResultListener(false);
      try (StreamLoader loader = new StreamLoader(properties, openConnection(), openConnection())) {
        loader.setListener(listener);
        loader.setTestMode(true);
        loader.start();

        // When rows are submitted and the loader finishes
        for (int i = 0; i < 100; i++) {
          loader.submitRow(new Object[] {i, "foo_" + i});
        }
        loader.finish();
      }

      // Then the retries upload every row
      assertEquals(0, listener.getErrorCount(), "error count");
      assertEquals(100, listener.getSubmittedRowCount(), "submitted rows");
      assertEquals(100, countRows(table), "row count");
    } finally {
      execute("DROP TABLE IF EXISTS \"" + table + "\"");
    }
  }

  private void execute(String sql) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute(sql);
    }
  }

  private int countRows(String table) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs = statement.executeQuery("SELECT COUNT(*) FROM \"" + table + "\"")) {
      assertTrue(rs.next(), "COUNT(*) should return a row");
      return rs.getInt(1);
    }
  }
}
