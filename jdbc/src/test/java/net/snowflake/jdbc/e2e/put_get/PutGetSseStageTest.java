package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.Statement;
import java.util.List;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * JDBC e2e mirror of {@code tests/definitions/shared/put_get/put_get_sse_stage.feature}.
 *
 * <p>Stages created with {@code ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')} return no client-side
 * encryption material; the driver must round-trip PUT/GET despite the missing key. Uploaded
 * uncompressed so the downloaded object is byte-comparable to the source.
 */
public class PutGetSseStageTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private String createSseStage(Connection connection, String prefix, boolean directoryEnabled)
      throws Exception {
    String stageName = (prefix + "_" + UUID.randomUUID().toString().replace("-", "")).toUpperCase();
    StringBuilder ddl =
        new StringBuilder("CREATE TEMPORARY STAGE IF NOT EXISTS ")
            .append(stageName)
            .append(" ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')");
    if (directoryEnabled) {
      ddl.append(" DIRECTORY = (ENABLE = TRUE)");
    }
    try (Statement statement = connection.createStatement()) {
      statement.execute(ddl.toString());
    }
    return stageName;
  }

  @Test
  public void shouldPutAndGetFileOnSseStage(@TempDir Path uploadDir, @TempDir Path downloadDir)
      throws Exception {
    // Given Stage with server-side encryption (SNOWFLAKE_SSE)
    String stageName = createSseStage(getDefaultConnection(), "TEST_SSE_PUT_GET", false);
    Path localFile = writeTextFile(uploadDir, "sse_test.txt", "hello sse\n");

    // When File is uploaded using PUT command
    PutRow putRow = uploadFileToStage(getDefaultConnection(), stageName, localFile, false, true);

    // Then File should be uploaded successfully
    assertEquals("UPLOADED", putRow.status, "Expected the PUT to succeed on the SSE stage");

    // When File is downloaded using GET command
    List<GetRow> getRows =
        getFileFromStage(getDefaultConnection(), stageName, "sse_test.txt", downloadDir);

    // Then File should be downloaded
    assertEquals(1, getRows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", getRows.get(0).status, "Expected DOWNLOADED status");
    Path downloaded = downloadDir.resolve("sse_test.txt");
    assertTrue(Files.exists(downloaded), "Expected the downloaded file on disk");

    // And Have correct content
    assertEquals(
        "hello sse", readTextMaybeGzip(downloaded).trim(), "Unexpected downloaded content");
  }

  @Test
  public void shouldPutAndGetFileOnSseStageWithDirectoryEnabled(
      @TempDir Path uploadDir, @TempDir Path downloadDir) throws Exception {
    // Given Stage with server-side encryption and DIRECTORY enabled
    String stageName = createSseStage(getDefaultConnection(), "TEST_SSE_DIR", true);
    Path localFile = writeTextFile(uploadDir, "test.txt", "hello sse\n");

    // When File is uploaded using PUT command
    PutRow putRow = uploadFileToStage(getDefaultConnection(), stageName, localFile, false, true);

    // Then File should be uploaded successfully
    assertEquals("UPLOADED", putRow.status, "Expected the PUT to succeed on the SSE stage");

    // When File is downloaded using GET command
    List<GetRow> getRows =
        getFileFromStage(getDefaultConnection(), stageName, "test.txt", downloadDir);

    // Then File should be downloaded
    assertEquals(1, getRows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", getRows.get(0).status, "Expected DOWNLOADED status");
    Path downloaded = downloadDir.resolve("test.txt");
    assertTrue(Files.exists(downloaded), "Expected the downloaded file on disk");

    // And Have correct content
    assertEquals(
        "hello sse", readTextMaybeGzip(downloaded).trim(), "Unexpected downloaded content");
  }
}
