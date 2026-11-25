package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import java.util.Arrays;
import java.util.stream.Stream;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.Operation;
import net.snowflake.client.internal.loader.StreamLoader;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * The staging table is created {@code LIKE} the target, so it inherits the VECTOR column and the
 * server rejects the CSV {@code COPY} into it before the {@code setVectorColumnType} cast in the
 * final {@code INSERT} is reached. Both drivers fail the same way.
 */
public class LoaderVectorTest extends LoaderTestBase {

  static Stream<Arguments> vectorColumns() {
    return Stream.of(
        Arguments.of("VECTOR(FLOAT, 3)", "FLOAT", "[12, 14.0, 100]"),
        Arguments.of("VECTOR(INT, 3)", "INT", "[12, 14, 100]"),
        Arguments.of("VECTOR(FLOAT, 3)", null, "[12, 14.0, 100]"));
  }

  @ParameterizedTest
  @MethodSource("vectorColumns")
  public void shouldThrowConnectionErrorAtCopyWhenTargetHasVectorColumn(
      String vectorDdl, String vectorType, String vectorLiteral) throws Exception {
    // Given a table with a vector column and a loader with the given vector column type
    String table = uniqueName("VECTOR_TABLE");
    createTable(table, "ID int, VEC " + vectorDdl);
    try {
      try (Loader loader =
          createLoader(
              loaderProperties(table, Operation.INSERT, Arrays.asList("ID", "VEC"), null),
              new RecordingLoadResultListener())) {
        if (vectorType != null) {
          ((StreamLoader) loader).setVectorColumnType(vectorType);
        }
        loader.start();
        loader.submitRow(new Object[] {1, vectorLiteral});

        // When the loader finishes
        Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::finish);

        // Then the COPY into the staging table fails with a SQL compilation error
        assertTrue(
            error.getMessage().startsWith("State: COPY_INTO_TABLE, "),
            () -> "unexpected ConnectionError message: " + error.getMessage());
        SQLException cause = assertInstanceOf(SQLException.class, error.getCause());
        assertEquals(1010, cause.getErrorCode(), "vendor code");
      }
      assertEquals(0, countRows(table), "row count");
    } finally {
      dropTable(table);
    }
  }
}
