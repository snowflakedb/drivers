package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderBatchingTest extends LoaderTestBase {

  private static final int ROWS = 10000;

  @Test
  public void shouldLoadSameDataWhenSplitIntoManySmallFilesAndStages() throws Exception {
    // Given a data set loaded into the target table with the default file and bucket sizes
    List<Object[]> dataSet = new ArrayList<>();
    for (int i = 0; i < ROWS; i++) {
      dataSet.add(defaultRow(i));
    }
    loadDataSet(loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null), dataSet);
    String referenceTable = uniqueName("LOADER_TEST_TABLE_REF");
    createTable(
        referenceTable,
        "ID int, C1 varchar(255), C2 varchar(255) DEFAULT 'X', C3 double, C4 timestamp,"
            + " C5 variant");
    try {
      // When the same data set is loaded with 30000-byte files and two files per stage
      Map<LoaderProperty, Object> properties =
          loaderProperties(referenceTable, Operation.INSERT, DEFAULT_COLUMNS, null);
      properties.put(LoaderProperty.csvFileBucketSize, 2L);
      properties.put(LoaderProperty.csvFileSize, 30000L);
      loadDataSet(properties, dataSet);

      // Then both tables hold the same rows
      assertEquals(ROWS, countRows(referenceTable), "reference row count");
      assertEquals(hashAgg(targetTable), hashAgg(referenceTable), "hash_agg of both tables");
    } finally {
      dropTable(referenceTable);
    }
  }

  @Test
  public void shouldLoadEveryRowWhenBatchRowSizeFlushesMidStream() throws Exception {
    // Given a loader that flushes after every 300 submitted rows
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null);
    properties.put(LoaderProperty.batchRowSize, 300);
    List<Object[]> dataSet = new ArrayList<>();
    for (int i = 0; i < 1000; i++) {
      dataSet.add(defaultRow(i));
    }

    // When 1000 rows are submitted
    RecordingLoadResultListener listener = loadDataSet(properties, dataSet);

    // Then every row is loaded once
    assertEquals(1000, listener.getSubmittedRowCount(), "submitted rows");
    assertEquals(1000, listener.processed.get(), "processed rows");
    assertEquals(1000, countRows(targetTable), "row count");
  }

  private RecordingLoadResultListener loadDataSet(
      Map<LoaderProperty, Object> properties, List<Object[]> dataSet) throws Exception {
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader = createLoader(properties, listener)) {
      loader.start();
      for (Object[] row : dataSet) {
        loader.submitRow(row);
      }
      loader.finish();
    }
    assertEquals(0, listener.getErrorCount(), "error count");
    return listener;
  }

  private long hashAgg(String table) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs = statement.executeQuery("SELECT hash_agg(*) FROM \"" + table + "\"")) {
      assertTrue(rs.next(), "hash_agg should return a row");
      return rs.getLong(1);
    }
  }
}
