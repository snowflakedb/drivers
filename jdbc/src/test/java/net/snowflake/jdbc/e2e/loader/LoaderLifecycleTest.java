package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.util.Collections;
import java.util.Map;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderFactory;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;

public class LoaderLifecycleTest extends LoaderTestBase {

  private static final String JOIN_FAILURE_PREFIX = "Failed to join StreamLoader queue: ";

  @Test
  public void shouldThrowDataErrorWhenFinishIsCalledWithoutStart() throws Exception {
    // Given a loader that was never started
    try (Loader loader =
        createLoader(
            loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null),
            new RecordingLoadResultListener())) {

      // When the loader finishes
      Loader.DataError error = assertThrows(Loader.DataError.class, loader::finish);

      // Then the error reports that the queues could not be joined
      assertTrue(
          error.getMessage().startsWith(JOIN_FAILURE_PREFIX),
          () -> "unexpected DataError message: " + error.getMessage());
    }
  }

  @Test
  public void shouldIgnoreRowsAndThrowDataErrorOnFinishWhenStartedWithoutOperation()
      throws Exception {
    // Given a loader with keys but no operation
    Map<LoaderProperty, Object> properties =
        loaderProperties(targetTable, null, DEFAULT_COLUMNS, Collections.singletonList("ID"));
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader = createLoader(properties, listener)) {

      // When the loader starts, receives a row and finishes
      loader.start();
      loader.submitRow(defaultRow(1));
      Loader.DataError error = assertThrows(Loader.DataError.class, loader::finish);

      // Then the row is dropped and finish reports that the queues could not be joined
      assertTrue(
          error.getMessage().startsWith(JOIN_FAILURE_PREFIX),
          () -> "unexpected DataError message: " + error.getMessage());
      assertEquals(0, listener.getSubmittedRowCount(), "submitted rows");
    }
    assertEquals(0, countRows(targetTable), "row count");
  }

  @Test
  public void shouldCloseBothConnectionsOnClose() throws Exception {
    // Given a loader built on two open connections
    try (Connection uploadConnection = openConnection();
        Connection processingConnection = openConnection()) {
      Loader loader =
          LoaderFactory.createLoader(
              loaderProperties(targetTable, Operation.INSERT, DEFAULT_COLUMNS, null),
              uploadConnection,
              processingConnection);

      // When the loader is closed
      loader.close();

      // Then both connections are closed
      assertTrue(uploadConnection.isClosed(), "upload connection should be closed");
      assertTrue(processingConnection.isClosed(), "processing connection should be closed");
    }
  }
}
