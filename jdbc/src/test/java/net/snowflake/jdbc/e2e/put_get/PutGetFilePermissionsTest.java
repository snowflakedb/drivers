package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.PosixFilePermission;
import java.nio.file.attribute.PosixFilePermissions;
import java.sql.Connection;
import java.util.List;
import java.util.Properties;
import java.util.Set;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetFilePermissionsTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private static final Set<PosixFilePermission> OWNER_ONLY =
      PosixFilePermissions.fromString("rw-------");

  private Path testDataCsv() {
    return sharedTestDataDir().resolve("compression").resolve("test_data.csv");
  }

  private static void assumePosix() {
    assumeTrue(
        FileSystems.getDefault().supportedFileAttributeViews().contains("posix"),
        "POSIX file permissions are not supported on this filesystem");
  }

  private static Set<PosixFilePermission> umaskBaseline(Path dir) throws Exception {
    Path baseline = dir.resolve("umask_baseline");
    Files.write(baseline, new byte[0]);
    return Files.getPosixFilePermissions(baseline);
  }

  private Set<PosixFilePermission> uploadThenDownloadPermissions(
      Connection connection, String stagePrefix, Path downloadDir) throws Exception {
    String stageName =
        createStageAndUploadFile(connection, stagePrefix, testDataCsv(), false, true);
    List<GetRow> rows = getFileFromStage(connection, stageName, "test_data.csv", downloadDir);
    assertEquals(1, rows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", rows.get(0).status, "Expected DOWNLOADED status");
    Path downloaded = downloadDir.resolve("test_data.csv");
    assertTrue(Files.exists(downloaded), "Expected the downloaded file on disk");
    return Files.getPosixFilePermissions(downloaded);
  }

  @Test
  public void shouldDownloadFileWithDriverDefaultPermissions(@TempDir Path downloadDir)
      throws Exception {
    assumePosix();

    // When a staged file is downloaded over the default connection
    Set<PosixFilePermission> actual =
        uploadThenDownloadPermissions(
            getDefaultConnection(), "TEST_FILE_PERMS_DEFAULT", downloadDir);

    // Then the mode bits match the driver's default
    if (isNewDriver()) {
      assertEquals(
          OWNER_ONLY, actual, "Universal-driver GET download must be owner-only by default");
    } else {
      assertEquals(
          umaskBaseline(downloadDir),
          actual,
          "Legacy GET download uses the process umask by default");
    }
  }

  @Test
  public void shouldDownloadFileWithNonDefaultPermissionsWhenConfigured(@TempDir Path downloadDir)
      throws Exception {
    assumePosix();

    Properties overrides = new Properties();
    if (isNewDriver()) {
      overrides.setProperty("unsafe_file_write", "true");
    } else {
      overrides.setProperty("ownerOnlyStageFilePermissionsEnabled", "true");
    }

    try (Connection connection = openConnection(overrides)) {
      // When a staged file is downloaded over a connection that inverts the default
      Set<PosixFilePermission> actual =
          uploadThenDownloadPermissions(connection, "TEST_FILE_PERMS_CONFIGURED", downloadDir);

      // Then the mode bits match the configured, non-default mode
      if (isNewDriver()) {
        assertEquals(
            umaskBaseline(downloadDir),
            actual,
            "Universal-driver unsafe_file_write=true must fall back to the process umask");
      } else {
        assertEquals(
            OWNER_ONLY,
            actual,
            "Legacy ownerOnlyStageFilePermissionsEnabled=true must force owner-only");
      }
    }
  }
}
