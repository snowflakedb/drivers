package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.zip.GZIPInputStream;

/**
 * Shared PUT/GET helpers for the JDBC e2e mirrors under {@code tests/definitions/shared/put_get/}.
 *
 * <p>PUT/GET rowsets are materialized into small value holders so no {@link ResultSet} escapes its
 * try-with-resources block. Columns are read by name, so the mapping is robust to the {@code
 * encryption} column between {@code status} and {@code message}.
 */
interface WithPutGet {

  /** One row of a PUT command rowset. */
  final class PutRow {
    final String source;
    final String target;
    final long sourceSize;
    final long targetSize;
    final String sourceCompression;
    final String targetCompression;
    final String status;
    final String encryption;
    final String message;

    PutRow(ResultSet rs) throws Exception {
      this.source = rs.getString("source");
      assertFalse(rs.wasNull(), "source should be non-null");
      this.target = rs.getString("target");
      assertFalse(rs.wasNull(), "target should be non-null");
      this.sourceSize = rs.getLong("source_size");
      assertFalse(rs.wasNull(), "source_size should be non-null");
      this.targetSize = rs.getLong("target_size");
      assertFalse(rs.wasNull(), "target_size should be non-null");
      this.sourceCompression = rs.getString("source_compression");
      assertFalse(rs.wasNull(), "source_compression should be non-null");
      this.targetCompression = rs.getString("target_compression");
      assertFalse(rs.wasNull(), "target_compression should be non-null");
      this.status = rs.getString("status");
      assertFalse(rs.wasNull(), "status should be non-null");
      this.encryption = rs.getString("encryption");
      assertFalse(rs.wasNull(), "encryption should be non-null");
      this.message = rs.getString("message");
      assertFalse(rs.wasNull(), "message should be non-null");
    }
  }

  /** One row of a GET command rowset. */
  final class GetRow {
    final String file;
    final long size;
    final String status;
    final String encryption;
    final String message;

    GetRow(ResultSet rs) throws Exception {
      this.file = rs.getString("file");
      assertFalse(rs.wasNull(), "file should be non-null");
      this.size = rs.getLong("size");
      assertFalse(rs.wasNull(), "size should be non-null");
      this.status = rs.getString("status");
      assertFalse(rs.wasNull(), "status should be non-null");
      this.encryption = rs.getString("encryption");
      assertFalse(rs.wasNull(), "encryption should be non-null");
      this.message = rs.getString("message");
      assertFalse(rs.wasNull(), "message should be non-null");
    }
  }

  /**
   * Resolve the shared {@code tests/test_data/generated_test_data} fixture directory, walking up
   * from the working directory so the lookup is independent of where Gradle launches the test.
   */
  default Path sharedTestDataDir() {
    Path dir = Paths.get("").toAbsolutePath();
    while (dir != null) {
      Path candidate = dir.resolve("tests").resolve("test_data").resolve("generated_test_data");
      if (Files.isDirectory(candidate)) {
        return candidate;
      }
      dir = dir.getParent();
    }
    throw new IllegalStateException(
        "Could not locate tests/test_data/generated_test_data above "
            + Paths.get("").toAbsolutePath());
  }

  /** Build the {@code file://} URI a PUT/GET command expects for a local path. */
  default String fileUri(Path path) {
    return "file://" + path.toAbsolutePath().toString().replace('\\', '/');
  }

  /** Create a uniquely named temporary stage and return its name. */
  default String createTemporaryStage(Connection connection, String prefix) throws Exception {
    String stageName = (prefix + "_" + UUID.randomUUID().toString().replace("-", "")).toUpperCase();
    try (Statement statement = connection.createStatement()) {
      statement.execute("CREATE TEMPORARY STAGE IF NOT EXISTS " + stageName);
    }
    return stageName;
  }

  /** Execute a PUT command and return its rowset (one row per uploaded file). */
  default List<PutRow> put(Connection connection, String putSql) throws Exception {
    List<PutRow> rows = new ArrayList<>();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(putSql)) {
      while (resultSet.next()) {
        rows.add(new PutRow(resultSet));
      }
    }
    return rows;
  }

  /** Execute a GET command and return its rowset (one row per downloaded file). */
  default List<GetRow> get(Connection connection, String getSql) throws Exception {
    List<GetRow> rows = new ArrayList<>();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(getSql)) {
      while (resultSet.next()) {
        rows.add(new GetRow(resultSet));
      }
    }
    return rows;
  }

  /** Upload a single file with explicit AUTO_COMPRESS / OVERWRITE flags; return the single row. */
  default PutRow uploadFileToStage(
      Connection connection,
      String stageName,
      Path filePath,
      boolean autoCompress,
      boolean overwrite)
      throws Exception {
    String putSql =
        String.format(
            "PUT '%s' @%s AUTO_COMPRESS=%s OVERWRITE=%s",
            fileUri(filePath),
            stageName,
            String.valueOf(autoCompress).toUpperCase(),
            String.valueOf(overwrite).toUpperCase());
    List<PutRow> rows = put(connection, putSql);
    assertEquals(1, rows.size(), "PUT of a single file should return one row");
    return rows.get(0);
  }

  /** Create a temporary stage and upload a single file to it, asserting UPLOADED. */
  default String createStageAndUploadFile(
      Connection connection,
      String stagePrefix,
      Path filePath,
      boolean autoCompress,
      boolean overwrite)
      throws Exception {
    String stageName = createTemporaryStage(connection, stagePrefix);
    PutRow row = uploadFileToStage(connection, stageName, filePath, autoCompress, overwrite);
    assertEquals("UPLOADED", row.status, "File upload should succeed");
    return stageName;
  }

  /** List a stage with {@code LS}; returns the {@code name} column of every listed object. */
  default List<String> listStageFileNames(Connection connection, String stageName)
      throws Exception {
    List<String> names = new ArrayList<>();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("LS @" + stageName)) {
      while (resultSet.next()) {
        names.add(resultSet.getString(1));
        assertFalse(resultSet.wasNull(), "stage file name should be non-null");
      }
    }
    return names;
  }

  /** Download a single logical file from a stage into {@code downloadDir}. */
  default List<GetRow> getFileFromStage(
      Connection connection, String stageName, String filename, Path downloadDir) throws Exception {
    String getSql = String.format("GET @%s/%s '%s/'", stageName, filename, fileUri(downloadDir));
    return get(connection, getSql);
  }

  /** Execute a wildcard PUT (pattern already POSIX-formatted) and return every uploaded row. */
  default List<PutRow> putWildcard(
      Connection connection,
      String stageName,
      String posixPattern,
      boolean autoCompress,
      boolean overwrite)
      throws Exception {
    String putSql =
        String.format(
            "PUT 'file://%s' @%s AUTO_COMPRESS=%s OVERWRITE=%s",
            posixPattern,
            stageName,
            String.valueOf(autoCompress).toUpperCase(),
            String.valueOf(overwrite).toUpperCase());
    return put(connection, putSql);
  }

  /** Download every stage file whose name matches {@code regexPattern} into {@code downloadDir}. */
  default List<GetRow> getWithPattern(
      Connection connection, String stageName, String regexPattern, Path downloadDir)
      throws Exception {
    String getSql =
        String.format("GET @%s '%s/' PATTERN='%s'", stageName, fileUri(downloadDir), regexPattern);
    return get(connection, getSql);
  }

  /** Write a small UTF-8 text file under {@code directory}, creating parents as needed. */
  default Path writeTextFile(Path directory, String filename, String content) throws IOException {
    Files.createDirectories(directory);
    Path filePath = directory.resolve(filename);
    Files.write(filePath, content.getBytes(java.nio.charset.StandardCharsets.UTF_8));
    return filePath;
  }

  /** Read the plain-text contents of a file, gunzip-ing it first when it ends in {@code .gz}. */
  default String readTextMaybeGzip(Path path) throws IOException {
    if (path.getFileName().toString().endsWith(".gz")) {
      try (InputStream in = new GZIPInputStream(Files.newInputStream(path))) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        int read;
        while ((read = in.read(buffer)) != -1) {
          out.write(buffer, 0, read);
        }
        return new String(out.toByteArray(), java.nio.charset.StandardCharsets.UTF_8);
      }
    }
    return new String(Files.readAllBytes(path), java.nio.charset.StandardCharsets.UTF_8);
  }
}
