package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.List;
import net.snowflake.jdbc.utils.EnabledOnGCP;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * JDBC e2e for {@code GCS_USE_DOWNSCOPED_CREDENTIAL} as a GS session parameter, plus SQL PUT/GET on
 * a GCS stage when {@code CLOUD_PROVIDER=gcp}.
 */
public class GcsDownscopedCredentialTest extends SnowflakeIntegrationTestBase
    implements WithPutGet {

  private static final String GCS_USE_DOWNSCOPED_CREDENTIAL = "GCS_USE_DOWNSCOPED_CREDENTIAL";

  @Test
  public void shouldExposeGcsUseDownscopedCredentialAfterConnect() throws Exception {
    // Given connections opened with GCS_USE_DOWNSCOPED_CREDENTIAL true and false
    // When SHOW PARAMETERS LIKE 'GCS_USE_DOWNSCOPED_CREDENTIAL' is executed
    // Then the session value matches the connection property (not the account default)
    assertGcsUseDownscopedCredential("true");
    assertGcsUseDownscopedCredential("false");
  }

  private void assertGcsUseDownscopedCredential(String expectedValue) throws Exception {
    try (Connection connection = openConnection(GCS_USE_DOWNSCOPED_CREDENTIAL, expectedValue);
        Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SHOW PARAMETERS LIKE 'GCS_USE_DOWNSCOPED_CREDENTIAL'")) {
      assertTrue(resultSet.next(), "Expected one SHOW PARAMETERS row");
      String value = resultSet.getString("value");
      assertFalse(resultSet.wasNull(), "value should be non-null");
      assertEquals(expectedValue, value);
      assertFalse(resultSet.next(), "Expected exactly one SHOW PARAMETERS row");
    }
  }

  @EnabledOnGCP
  @Test
  public void shouldPutAndGetOnGcsWithDownscopedCredential(
      @TempDir Path uploadDir, @TempDir Path downloadDir) throws Exception {
    Path source = writeTextFile(uploadDir, "gcs_downscoped.txt", "gcs downscoped payload\n");

    try (Connection connection = openConnection(GCS_USE_DOWNSCOPED_CREDENTIAL, "true")) {
      ensureDatabaseAndSchema(connection);
      // Given a file is uploaded to a GCS stage with AUTO_COMPRESS=FALSE
      String stageName = createTemporaryStage(connection, "TEST_GCS_DOWNSCOPED");
      PutRow put = uploadFileToStage(connection, stageName, source, false, true);
      assertEquals("UPLOADED", put.status, "Expected UPLOADED status");

      // When the file is downloaded with GET
      List<GetRow> rows =
          getFileFromStage(connection, stageName, "gcs_downscoped.txt", downloadDir);

      // Then GET reports DOWNLOADED and the bytes match
      assertEquals(1, rows.size(), "Expected exactly one downloaded file");
      assertEquals("DOWNLOADED", rows.get(0).status, "Expected DOWNLOADED status");
      Path downloaded = downloadDir.resolve("gcs_downscoped.txt");
      assertTrue(Files.exists(downloaded), "Expected the downloaded file on disk");
      assertEquals(
          "gcs downscoped payload",
          readTextMaybeGzip(downloaded).trim(),
          "Unexpected downloaded content");
    }
  }

  @EnabledOnGCP
  @Test
  public void shouldSkipWhenOverwriteIsFalseOnDefaultGcs(@TempDir Path uploadDir) throws Exception {
    try (Connection connection = openConnection()) {
      ensureDatabaseAndSchema(connection);
      // Given a file is already on a GCS stage opened with the default connection
      // When the same file is PUT again with OVERWRITE=FALSE
      // Then STATUS is SKIPPED
      assertOverwriteFalseStatus(connection, uploadDir, "gcs_overwrite_default.txt", "SKIPPED");
    }
  }

  @EnabledOnGCP
  @Test
  public void shouldSkipWhenOverwriteIsFalseOnDownscopedGcs(@TempDir Path uploadDir)
      throws Exception {
    try (Connection connection = openConnection(GCS_USE_DOWNSCOPED_CREDENTIAL, "true")) {
      ensureDatabaseAndSchema(connection);
      // Given a file is already on a GCS stage with GCS_USE_DOWNSCOPED_CREDENTIAL=true
      // When the same file is PUT again with OVERWRITE=FALSE
      // Then STATUS is SKIPPED
      assertOverwriteFalseStatus(connection, uploadDir, "gcs_overwrite_downscoped.txt", "SKIPPED");
    }
  }

  @EnabledOnGCP
  @Test
  public void shouldGetMultipleFilesFromGcsStageInSingleCommand(
      @TempDir Path uploadDir, @TempDir Path downloadDir) throws Exception {
    Path alpha = writeTextFile(uploadDir, "alpha.txt", "contents of alpha\n");
    Path beta = writeTextFile(uploadDir, "beta.txt", "contents of beta\n");

    try (Connection connection = openConnection(GCS_USE_DOWNSCOPED_CREDENTIAL, "true")) {
      ensureDatabaseAndSchema(connection);
      String stageName = createTemporaryStage(connection, "TEST_GCS_MULTI_GET");

      // Given Two files are uploaded to stage
      assertEquals(
          "UPLOADED",
          uploadFileToStage(connection, stageName, alpha, false, true).status,
          "Expected alpha.txt to upload");
      assertEquals(
          "UPLOADED",
          uploadFileToStage(connection, stageName, beta, false, true).status,
          "Expected beta.txt to upload");

      // When All files are downloaded from stage using GET command
      List<GetRow> rows =
          get(connection, String.format("GET @%s '%s/'", stageName, fileUri(downloadDir)));

      // Then All files should be downloaded
      assertEquals(2, rows.size(), "Expected two downloaded files");
      for (GetRow row : rows) {
        assertEquals("DOWNLOADED", row.status, "Expected DOWNLOADED status for " + row.file);
      }

      // And Each file should have correct content
      assertEquals(
          "contents of alpha",
          readTextMaybeGzip(downloadDir.resolve("alpha.txt")).trim(),
          "Unexpected alpha.txt content");
      assertEquals(
          "contents of beta",
          readTextMaybeGzip(downloadDir.resolve("beta.txt")).trim(),
          "Unexpected beta.txt content");
    }
  }

  private void assertOverwriteFalseStatus(
      Connection connection, Path uploadDir, String filename, String expectedSecondStatus)
      throws Exception {
    Path source = writeTextFile(uploadDir, filename, "overwrite-false payload\n");
    String stageName = createTemporaryStage(connection, "TEST_GCS_OVERWRITE_FALSE");
    assertEquals(
        "UPLOADED",
        uploadFileToStage(connection, stageName, source, false, true).status,
        "Expected first PUT to be UPLOADED");
    PutRow second = uploadFileToStage(connection, stageName, source, false, false);
    assertEquals(
        expectedSecondStatus,
        second.status,
        "OVERWRITE=FALSE STATUS for " + filename + " was " + second.status);
  }
}
