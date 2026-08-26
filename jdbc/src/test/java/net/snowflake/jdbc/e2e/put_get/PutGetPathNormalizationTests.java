package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.DriverCompatibility.isOldDriver;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetPathNormalizationTests extends SnowflakeIntegrationTestBase {

  private static final String CONTENT = "a,b,c\n";

  // Convert a filesystem path to the file:// URI string expected by PUT.
  // On Windows, backslashes are replaced with forward slashes.
  private static String toFileUri(Path path) {
    return path.toAbsolutePath().toString().replace('\\', '/');
  }

  private static String uniqueStageName() {
    return ("TEST_PUT_PATH_NORM_" + UUID.randomUUID().toString().replace("-", "")).toUpperCase();
  }

  @Test
  public void shouldUploadFileWhenSourcePathContainsDotdotSegments(@TempDir Path tempDir)
      throws Exception {
    // Given A source file exists in a temporary directory
    Path subDir = Files.createDirectory(tempDir.resolve("sub"));
    Path sourceFile = tempDir.resolve("dotdot_data.csv");
    Files.write(sourceFile, CONTENT.getBytes(StandardCharsets.UTF_8));

    // When PUT command is executed with a source path containing dotdot segments
    Path dotdotPath = subDir.resolve("..").resolve("dotdot_data.csv"); // absolute, un-normalized
    String stageName = uniqueStageName();
    Connection conn = getDefaultConnection();
    execute(conn, "CREATE TEMPORARY STAGE " + stageName);

    // PUT syntax does not support ? binding for file URIs or @stage references;
    // stageName is connector-internally generated.
    String putSql =
        "PUT 'file://"
            + toFileUri(dotdotPath)
            + "' @"
            + stageName
            + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery(putSql)) {
      // Then File is uploaded successfully with correct target name
      assertTrue(rs.next(), "Expected one PUT result row");
      assertEquals("UPLOADED", rs.getString(7), "Expected UPLOADED status");
      assertFalse(rs.wasNull(), "Status must not be NULL");
      assertEquals("dotdot_data.csv", rs.getString(2), "Expected canonical target filename");
      assertFalse(rs.wasNull(), "Target filename must not be NULL");
      assertFalse(rs.next(), "Expected exactly one PUT result row");
    }
  }

  @Test
  public void shouldUploadFileWhenSourcePathIsRelativeToWorkingDirectory() throws Exception {
    // Create under CWD so Path.relativize stays same-root on Windows (system temp can
    // live on another drive, which makes relativize throw IllegalArgumentException).
    Path cwd = Paths.get("").toAbsolutePath();
    Path workDir = Files.createTempDirectory(cwd, "put_relative_jdbc_");
    try {
      // Given A source file exists in a temporary directory
      Path sourceFile = workDir.resolve("relative_data.csv");
      Files.write(sourceFile, CONTENT.getBytes(StandardCharsets.UTF_8));

      // When PUT command is executed with a path relative to the process working directory
      String relativePath = cwd.relativize(sourceFile).toString().replace('\\', '/');
      String stageName = uniqueStageName();
      Connection conn = getDefaultConnection();
      execute(conn, "CREATE TEMPORARY STAGE " + stageName);

      // PUT syntax does not support ? binding for file URIs or @stage references;
      // stageName is connector-internally generated.
      String putSql =
          "PUT 'file://" + relativePath + "' @" + stageName + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
      try (Statement stmt = conn.createStatement();
          ResultSet rs = stmt.executeQuery(putSql)) {
        // Then File is uploaded successfully with correct target name
        assertTrue(rs.next(), "Expected one PUT result row");
        assertEquals("UPLOADED", rs.getString(7), "Expected UPLOADED status");
        assertFalse(rs.wasNull(), "Status must not be NULL");
        assertEquals("relative_data.csv", rs.getString(2), "Expected canonical target filename");
        assertFalse(rs.wasNull(), "Target filename must not be NULL");
        assertFalse(rs.next(), "Expected exactly one PUT result row");
      }
    } finally {
      Files.deleteIfExists(workDir.resolve("relative_data.csv"));
      Files.deleteIfExists(workDir);
    }
  }

  @Test
  public void shouldUploadFileAtSymlinkedSourcePath(@TempDir Path tempDir) throws Exception {
    // Symlinks are a Unix feature; skip on Windows where Files.createSymbolicLink
    // requires elevated privileges and is not needed for this scenario.
    assumeTrue(
        !System.getProperty("os.name", "").toLowerCase().startsWith("windows"),
        "Symlink test requires Unix");

    // Given A source file and a symlink pointing to it exist in a temporary directory
    Path realFile = tempDir.resolve("real.csv");
    Files.write(realFile, CONTENT.getBytes(StandardCharsets.UTF_8));
    Path linkFile = tempDir.resolve("link.csv");
    Files.createSymbolicLink(linkFile, realFile);

    // When PUT command is executed with the symlink as source path
    String stageName = uniqueStageName();
    Connection conn = getDefaultConnection();
    execute(conn, "CREATE TEMPORARY STAGE " + stageName);

    // PUT syntax does not support ? binding for file URIs or @stage references;
    // stageName is connector-internally generated.
    String putSql =
        "PUT 'file://"
            + toFileUri(linkFile)
            + "' @"
            + stageName
            + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery(putSql)) {
      // Then File is uploaded successfully
      assertTrue(rs.next(), "Expected one PUT result row");
      assertEquals("UPLOADED", rs.getString(7), "Expected UPLOADED status");
      assertFalse(rs.wasNull(), "Status must not be NULL");

      // New driver resolves symlinks via dunce::canonicalize (BD#57): stage object is the
      // target's basename. Legacy JDBC only canonicalizes glob-matched paths
      // (SnowflakeFileTransferAgent.expandFileNames); a literal symlinked source like this one
      // is added to the result set verbatim, so legacy JDBC reports the symlink's own basename.
      if (isOldDriver()) {
        assertEquals("link.csv", rs.getString(2), "Old driver preserves the symlink's own name");
      } else {
        assertEquals("real.csv", rs.getString(2), "New driver reports the symlink target's name");
      }
      assertFalse(rs.wasNull(), "Target filename must not be NULL");
      assertFalse(rs.next(), "Expected exactly one PUT result row");
    }
  }

  @Test
  public void shouldUploadFileWhenSourcePathStartsWithTilde() throws Exception {
    Path homeDir = Paths.get(System.getProperty("user.home"));
    Path subDir = Files.createTempDirectory(homeDir, "ud_put_tilde_");
    try {
      // Given A source file exists in a subdirectory under the home directory
      Path sourceFile = subDir.resolve("tilde_data.csv");
      Files.write(sourceFile, CONTENT.getBytes(StandardCharsets.UTF_8));

      // When PUT command is executed with a leading ~ in the source path
      String stageName = uniqueStageName();
      Connection conn = getDefaultConnection();
      execute(conn, "CREATE TEMPORARY STAGE " + stageName);

      // PUT syntax does not support ? binding for file URIs or @stage references;
      // stageName is connector-internally generated.
      String putSql =
          "PUT 'file://~/"
              + subDir.getFileName()
              + "/tilde_data.csv' @"
              + stageName
              + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
      try (Statement stmt = conn.createStatement();
          ResultSet rs = stmt.executeQuery(putSql)) {
        // Then File is uploaded successfully
        assertTrue(rs.next(), "Expected one PUT result row");
        assertEquals("UPLOADED", rs.getString(7), "Expected UPLOADED status");
        assertFalse(rs.wasNull(), "Status must not be NULL");
        assertFalse(rs.next(), "Expected exactly one PUT result row");
      }
    } finally {
      // A single file plus its parent directory under $HOME — no recursive walker needed.
      Files.deleteIfExists(subDir.resolve("tilde_data.csv"));
      Files.deleteIfExists(subDir);
    }
  }
}
