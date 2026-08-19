package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class PutGetSourceCompressionTest extends SnowflakeIntegrationTestBase
    implements WithPutGet {

  @Test
  public void shouldAutoDetectStandardCompressionTypesWhenSourceCompressionSetToAutoDetect()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
    String[][] cases = {
      {"GZIP", "test_data.csv.gz"},
      {"BZIP2", "test_data.csv.bz2"},
      {"BROTLI", "test_data.csv.br"},
      {"ZSTD", "test_data.csv.zst"},
      {"DEFLATE", "test_data.csv.deflate"},
    };

    for (String[] testCase : cases) {
      String compression = testCase[0];
      String filename = testCase[1];
      String stageName = createTemporaryStage(connection, "TEST_STAGE_" + compression);

      // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
      String putSql =
          String.format(
              "PUT '%s' @%s SOURCE_COMPRESSION=AUTO_DETECT",
              fileUri(compressionFile(filename)), stageName);
      PutRow row = putSingle(connection, putSql);

      // Then Target compression has correct type and all PUT results are correct
      assertPutCompression(row, filename, compression, filename, compression);
    }
  }

  @Test
  public void shouldUploadCompressedFilesWithSourceCompressionSetToExplicitTypes()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
    String[][] cases = {
      {"GZIP", "test_data.csv.gz"},
      {"BZIP2", "test_data.csv.bz2"},
      {"BROTLI", "test_data.csv.br"},
      {"ZSTD", "test_data.csv.zst"},
      {"DEFLATE", "test_data.csv.deflate"},
      {"RAW_DEFLATE", "test_data.csv.raw_deflate"},
    };

    for (String[] testCase : cases) {
      String compression = testCase[0];
      String filename = testCase[1];
      String stageName = createTemporaryStage(connection, "TEST_STAGE_EXPLICIT_" + compression);

      // When File is uploaded with SOURCE_COMPRESSION set to explicit type
      String putSql =
          String.format(
              "PUT '%s' @%s SOURCE_COMPRESSION=%s",
              fileUri(compressionFile(filename)), stageName, compression);
      PutRow row = putSingle(connection, putSql);

      // Then Target compression has correct type and all PUT results are correct
      assertPutCompression(row, filename, compression, filename, compression);
    }
  }

  @Test
  public void shouldNotCompressFileWhenSourceCompressionSetToAutoDetectAndAutoCompressSetToFalse()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Uncompressed file
    String stageName = createTemporaryStage(connection, "TEST_STAGE_AUTO_DETECT_NO_COMPRESS");

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to
    // FALSE
    String putSql =
        String.format(
            "PUT '%s' @%s SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE",
            fileUri(compressionFile("test_data.csv")), stageName);
    PutRow row = putSingle(connection, putSql);

    // Then File is not compressed
    assertPutCompression(row, "test_data.csv", "NONE", "test_data.csv", "NONE");
  }

  @Test
  public void shouldNotCompressFileWhenSourceCompressionSetToNoneAndAutoCompressSetToFalse()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Uncompressed file
    String stageName = createTemporaryStage(connection, "TEST_STAGE_NONE_NO_COMPRESS");

    // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
    String putSql =
        String.format(
            "PUT '%s' @%s SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE",
            fileUri(compressionFile("test_data.csv")), stageName);
    PutRow row = putSingle(connection, putSql);

    // Then File is not compressed
    assertPutCompression(row, "test_data.csv", "NONE", "test_data.csv", "NONE");
  }

  @Test
  // spotless:off
  public void shouldCompressUncompressedFileWhenSourceCompressionSetToAutoDetectAndAutoCompressSetToTrue()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Uncompressed file
    String stageName = createTemporaryStage(connection, "TEST_STAGE_AUTO_DETECT_COMPRESS");

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
    String putSql =
        String.format(
            "PUT '%s' @%s SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE",
            fileUri(compressionFile("test_data.csv")), stageName);
    PutRow row = putSingle(connection, putSql);

    // Then Target compression has GZIP type and all PUT results are correct
    assertPutCompression(row, "test_data.csv", "NONE", "test_data.csv.gz", "GZIP");
    // spotless:on
  }

  @Test
  public void shouldCompressUncompressedFileWhenSourceCompressionSetToNoneAndAutoCompressSetToTrue()
      throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Uncompressed file
    String stageName = createTemporaryStage(connection, "TEST_STAGE_NONE_COMPRESS");

    // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
    String putSql =
        String.format(
            "PUT '%s' @%s SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE",
            fileUri(compressionFile("test_data.csv")), stageName);
    PutRow row = putSingle(connection, putSql);

    // Then Target compression has GZIP type and all PUT results are correct
    assertPutCompression(row, "test_data.csv", "NONE", "test_data.csv.gz", "GZIP");
  }

  @Test
  public void shouldRecordAnErrorRowForUnsupportedCompressionType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And File compressed with unsupported format
    String stageName = createTemporaryStage(connection, "TEST_STAGE_UNSUPPORTED");

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    String putSql =
        String.format(
            "PUT '%s' @%s SOURCE_COMPRESSION=AUTO_DETECT",
            fileUri(compressionFile("test_data.csv.xz")), stageName);

    // Then PUT result status is ERROR
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(putSql)) {
      assertTrue(resultSet.next(), "PUT of a single file should return one row");
      String source = resultSet.getString("source");
      assertFalse(resultSet.wasNull(), "source should be non-null");
      assertEquals("test_data.csv.xz", source, "Unexpected source filename");
      String status = resultSet.getString("status");
      assertFalse(resultSet.wasNull(), "status should be non-null");
      assertEquals("ERROR", status, "Unexpected status");
      String sourceCompression = resultSet.getString("source_compression");
      assertFalse(resultSet.wasNull(), "source_compression should be non-null");
      assertEquals("XZ", sourceCompression, "Unexpected source compression");
      String message = resultSet.getString("message");
      assertFalse(resultSet.wasNull(), "message should be non-null");
      // And The message is "Copy command does not support compression type XZ."
      assertEquals(
          "Copy command does not support compression type XZ.",
          message,
          "Unexpected ERROR-row message");
      assertFalse(resultSet.next(), "PUT of a single file should return exactly one row");
    }
  }
}
