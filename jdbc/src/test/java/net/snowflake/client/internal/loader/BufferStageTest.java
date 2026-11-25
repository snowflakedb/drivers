package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.io.File;
import java.io.IOException;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.attribute.PosixFilePermissions;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Collections;
import java.util.Comparator;
import java.util.UUID;
import java.util.stream.Stream;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class BufferStageTest {

  private String table;
  private StreamLoader loader;

  @BeforeEach
  void setUp() throws SQLException {
    table = "BUFFER_STAGE_TEST_" + UUID.randomUUID().toString().replace("-", "");
    Connection connection = mock(Connection.class);
    when(connection.createStatement()).thenReturn(mock(Statement.class));
    loader =
        new StreamLoader(
            Collections.singletonMap(LoaderProperty.tableName, table), connection, connection);
  }

  @AfterEach
  void deleteTableDirectory() throws IOException {
    Path tableDir = Paths.get(loader.getBase(), table);
    if (!Files.exists(tableDir)) {
      return;
    }
    try (Stream<Path> paths = Files.walk(tableDir)) {
      for (Path path : (Iterable<Path>) paths.sorted(Comparator.reverseOrder())::iterator) {
        path.toFile().setWritable(true);
        Files.delete(path);
      }
    }
  }

  @Test
  void shouldCreateOwnerOnlyStageDirectoryOnPosix() throws IOException {
    assumeTrue(FileSystems.getDefault().supportedFileAttributeViews().contains("posix"));

    BufferStage stage = new BufferStage(loader, Operation.INSERT, 64, 1024);

    Path stageDir = Paths.get(loader.getBase(), stage.getRemoteLocation());
    assertEquals(
        PosixFilePermissions.fromString("rwx------"),
        Files.getPosixFilePermissions(stageDir),
        "stage directory permissions");
    assertEquals(
        Paths.get(loader.getBase(), table, "INSERT"), stageDir.getParent(), "stage directory");
    stage.completeUploading();
  }

  @Test
  void shouldAbortAndThrowWhenStageDirectoryCannotBeCreated() throws IOException {
    Path tableDir = Paths.get(loader.getBase(), table);
    Files.createDirectories(tableDir);
    Files.createFile(tableDir.resolve("INSERT"));

    RuntimeException error =
        assertThrows(
            RuntimeException.class, () -> new BufferStage(loader, Operation.INSERT, 64, 1024));

    assertTrue(
        error
            .getMessage()
            .startsWith(
                "Could not initialize the local staging area. "
                    + "Make sure the directory is writable and readable: "
                    + tableDir.resolve("INSERT")),
        () -> "unexpected message: " + error.getMessage());
    assertTrue(loader.isAborted(), "loader should be aborted");
  }

  @ParameterizedTest
  @CsvSource({".,_", "..,__"})
  void shouldKeepLocalStageDirectoryUnderBaseForDotTableNames(
      String dotTable, String localDirectory) throws Exception {
    StreamLoader dotLoader =
        new StreamLoader(
            Collections.singletonMap(LoaderProperty.tableName, dotTable),
            mock(Connection.class),
            mock(Connection.class));

    BufferStage stage = new BufferStage(dotLoader, Operation.INSERT, 64, 1024);

    String batch = stage.getRemoteLocation().substring(stage.getRemoteLocation().lastIndexOf('/'));
    assertEquals(dotTable + "/INSERT" + batch, stage.getRemoteLocation(), "remote location");
    Path batchDir = Paths.get(dotLoader.getBase(), localDirectory, "INSERT", batch.substring(1));
    try {
      assertTrue(Files.isDirectory(batchDir), () -> "missing local stage directory " + batchDir);
    } finally {
      stage.completeUploading();
      deleteRecursively(batchDir);
    }
  }

  @Test
  void shouldMapOnlySeparatorsAndDotSegmentsInLocalTableDirectory() {
    assertEquals("a_b", BufferStage.localTableDirectory("a" + File.separator + "b"));
    assertEquals("...", BufferStage.localTableDirectory("..."));
    assertEquals(".hidden", BufferStage.localTableDirectory(".hidden"));
    assertEquals("Table 1", BufferStage.localTableDirectory("Table 1"));
  }

  private static void deleteRecursively(Path root) throws IOException {
    if (!Files.exists(root)) {
      return;
    }
    try (Stream<Path> paths = Files.walk(root)) {
      for (Path path : (Iterable<Path>) paths.sorted(Comparator.reverseOrder())::iterator) {
        Files.delete(path);
      }
    }
  }
}
