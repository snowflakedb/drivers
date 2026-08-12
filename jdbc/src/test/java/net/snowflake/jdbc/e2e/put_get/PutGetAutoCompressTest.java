package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.util.List;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetAutoCompressTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private Path uncompressedFile() {
    return sharedTestDataDir().resolve("compression").resolve("test_data.csv");
  }

  @Test
  public void shouldCompressTheFileBeforeUploadingToStageWhenAutoCompressSetToTrue(
      @TempDir Path downloadDir) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When File is uploaded to stage with AUTO_COMPRESS set to true
    String stageName =
        createStageAndUploadFile(
            connection, "TEST_PUT_GET_AUTO_COMPRESS_TRUE", uncompressedFile(), true, true);

    // Then Only compressed file should be downloaded
    List<GetRow> rows =
        getFileFromStage(getDefaultConnection(), stageName, "test_data.csv", downloadDir);
    assertEquals(1, rows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", rows.get(0).status, "Expected DOWNLOADED status");
    Path compressed = downloadDir.resolve("test_data.csv.gz");
    assertTrue(Files.exists(compressed), "Expected the compressed file on disk");
    assertFalse(
        Files.exists(downloadDir.resolve("test_data.csv")),
        "Did not expect an uncompressed file on disk");

    // And Have correct content
    assertEquals("1,2,3", readTextMaybeGzip(compressed).trim(), "Unexpected downloaded content");
  }

  @Test
  public void shouldNotCompressTheFileBeforeUploadingToStageWhenAutoCompressSetToFalse(
      @TempDir Path downloadDir) throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When File is uploaded to stage with AUTO_COMPRESS set to false
    String stageName =
        createStageAndUploadFile(
            connection, "TEST_PUT_GET_AUTO_COMPRESS_FALSE", uncompressedFile(), false, true);

    // Then Only uncompressed file should be downloaded
    List<GetRow> rows =
        getFileFromStage(getDefaultConnection(), stageName, "test_data.csv", downloadDir);
    assertEquals(1, rows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", rows.get(0).status, "Expected DOWNLOADED status");
    Path uncompressed = downloadDir.resolve("test_data.csv");
    assertTrue(Files.exists(uncompressed), "Expected the uncompressed file on disk");
    assertFalse(
        Files.exists(downloadDir.resolve("test_data.csv.gz")),
        "Did not expect a compressed file on disk");

    // And Have correct content
    assertEquals("1,2,3", readTextMaybeGzip(uncompressed).trim(), "Unexpected downloaded content");
  }
}
