package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.sql.SQLException;
import java.sql.Statement;
import java.util.Arrays;
import java.util.Date;
import java.util.Map;
import java.util.Random;
import java.util.concurrent.Callable;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderConcurrencyTest extends LoaderTestBase {

  private static final int ROWS_PER_LOADER = 50000;

  @Test
  public void shouldLoadAllRowsWhenTwoLoadersRunConcurrentlyThroughNamedStage() throws Exception {
    // Given a named stage and a timestamp table shared by two loaders
    String stage = uniqueName("STAGE_MULTITHREAD_LOADER");
    String table = uniqueName("TABLE_ISSUE_SIMPLEDATEFORMAT");
    createTable(table, "ID int, C1 timestamp");
    ExecutorService executor = Executors.newFixedThreadPool(2);
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("CREATE OR REPLACE STAGE " + stage);

      // When both loaders insert rows with random dates at the same time
      Future<RecordingLoadResultListener> first = executor.submit(loadRandomDates(table, stage));
      Future<RecordingLoadResultListener> second = executor.submit(loadRandomDates(table, stage));
      RecordingLoadResultListener firstListener = first.get(10, TimeUnit.MINUTES);
      RecordingLoadResultListener secondListener = second.get(10, TimeUnit.MINUTES);

      // Then each loader reports every row without errors and the table holds both sets
      for (RecordingLoadResultListener listener : Arrays.asList(firstListener, secondListener)) {
        assertEquals(0, listener.getErrorCount(), "error count");
        assertEquals(ROWS_PER_LOADER, listener.getSubmittedRowCount(), "submitted rows");
      }
      assertEquals(ROWS_PER_LOADER * 2, countRows(table), "row count");
    } finally {
      executor.shutdownNow();
      dropStage(stage);
      dropTable(table);
    }
  }

  private Callable<RecordingLoadResultListener> loadRandomDates(String table, String stage)
      throws SQLException {
    Map<LoaderProperty, Object> properties =
        loaderProperties(table, Operation.INSERT, Arrays.asList("ID", "C1"), null);
    properties.put(LoaderProperty.remoteStage, stage);
    properties.put(LoaderProperty.startTransaction, true);
    properties.put(LoaderProperty.truncateTable, false);
    return () -> {
      RecordingLoadResultListener listener = new RecordingLoadResultListener();
      try (Loader loader = createLoader(properties, listener)) {
        loader.start();
        Random random = new Random();
        for (int i = 0; i < ROWS_PER_LOADER; ++i) {
          long ms =
              -946771200000L + (Math.abs(random.nextLong()) % (70L * 365 * 24 * 60 * 60 * 1000));
          loader.submitRow(new Object[] {i, new Date(ms)});
        }
        loader.finish();
      }
      return listener;
    };
  }

  private void dropStage(String stage) throws SQLException {
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute("DROP STAGE IF EXISTS " + stage);
    }
  }
}
