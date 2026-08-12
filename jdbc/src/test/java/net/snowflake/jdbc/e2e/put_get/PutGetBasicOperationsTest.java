package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.sql.Types;
import java.util.List;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetBasicOperationsTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private Path testDataCsv() {
    return sharedTestDataDir().resolve("compression").resolve("test_data.csv");
  }

  @Test
  public void shouldSelectDataFromFileUploadedToStage() throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_SELECT", testDataCsv(), true, true);

    // When File data is queried using Select command
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT $1, $2, $3 FROM @" + stageName)) {
      // Then File data should be correctly returned
      assertTrue(resultSet.next(), "Expected one row from the uploaded file");
      assertEquals("1", resultSet.getString(1), "Unexpected first column");
      assertEquals("2", resultSet.getString(2), "Unexpected second column");
      assertEquals("3", resultSet.getString(3), "Unexpected third column");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldListFileUploadedToStage() throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_LS", testDataCsv(), true, true);

    // When Stage content is listed using LS command
    List<String> files = listStageFileNames(getDefaultConnection(), stageName);

    // Then File should be listed with correct filename
    assertEquals(1, files.size(), "Expected exactly one file on the stage");
    assertTrue(
        files.get(0).contains("test_data.csv.gz"),
        "Expected the compressed filename, got: " + files.get(0));
  }

  @Test
  public void shouldGetFileUploadedToStage(@TempDir Path downloadDir) throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_GET", testDataCsv(), true, true);

    // When File is downloaded using GET command
    List<GetRow> rows =
        getFileFromStage(getDefaultConnection(), stageName, "test_data.csv", downloadDir);

    // Then File should be downloaded
    assertEquals(1, rows.size(), "Expected exactly one downloaded file");
    assertEquals("DOWNLOADED", rows.get(0).status, "Expected DOWNLOADED status");
    Path downloaded = downloadDir.resolve("test_data.csv.gz");
    assertTrue(Files.exists(downloaded), "Expected the compressed file on disk");

    // And Have correct content
    assertEquals("1,2,3", readTextMaybeGzip(downloaded).trim(), "Unexpected downloaded content");
  }

  @Test
  public void shouldReturnCorrectRowsetForPut() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When File is uploaded to stage
    String stageName = createTemporaryStage(connection, "TEST_STAGE_PUT_ROWSET");
    PutRow row = uploadFileToStage(connection, stageName, testDataCsv(), true, true);

    // Then Rowset for PUT command should be correct
    assertEquals("test_data.csv", row.source, "Unexpected source filename");
    assertEquals("test_data.csv.gz", row.target, "Unexpected target filename");
    assertEquals(6, row.sourceSize, "Unexpected source size");
    assertEquals(26, row.targetSize, "Unexpected target size");
    assertEquals("NONE", row.sourceCompression, "Unexpected source compression");
    assertEquals("GZIP", row.targetCompression, "Unexpected target compression");
    assertEquals("UPLOADED", row.status, "Unexpected status");
    assertEquals("ENCRYPTED", row.encryption, "Unexpected encryption");
    assertEquals("", row.message, "Expected empty message");
  }

  @Test
  public void shouldReturnCorrectRowsetForGet(@TempDir Path downloadDir) throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_GET_ROWSET", testDataCsv(), true, true);

    // When File is downloaded using GET command
    List<GetRow> rows =
        getFileFromStage(getDefaultConnection(), stageName, "test_data.csv", downloadDir);

    // Then Rowset for GET command should be correct
    assertEquals(1, rows.size(), "Expected exactly one downloaded file");
    GetRow row = rows.get(0);
    assertEquals("test_data.csv.gz", row.file, "Unexpected downloaded filename");
    assertEquals(26, row.size, "Unexpected downloaded size");
    assertEquals("DOWNLOADED", row.status, "Unexpected status");
    assertEquals("DECRYPTED", row.encryption, "Unexpected encryption");
    assertEquals("", row.message, "Expected empty message");
  }

  @Test
  public void shouldReturnCorrectColumnMetadataForPut() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When File is uploaded to stage
    String stageName = createTemporaryStage(connection, "TEST_STAGE_PUT_COLUMN_METADATA");
    String putSql =
        String.format(
            "PUT '%s' @%s AUTO_COMPRESS=TRUE OVERWRITE=TRUE", fileUri(testDataCsv()), stageName);
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(putSql)) {
      // Then Column metadata for PUT command should be correct
      ResultSetMetaData meta = resultSet.getMetaData();
      assertEquals(9, meta.getColumnCount(), "PUT should return 9 columns");
      assertTextColumn(meta, 1, "source");
      assertTextColumn(meta, 2, "target");
      assertFixedColumn(meta, 3, "source_size");
      assertFixedColumn(meta, 4, "target_size");
      assertTextColumn(meta, 5, "source_compression");
      assertTextColumn(meta, 6, "target_compression");
      assertTextColumn(meta, 7, "status");
      assertTextColumn(meta, 8, "encryption");
      assertTextColumn(meta, 9, "message");
      assertTrue(resultSet.next(), "Expected one PUT result row");
      assertEquals("UPLOADED", resultSet.getString(7), "Expected UPLOADED status");
      assertFalse(resultSet.wasNull(), "status should be non-null");
    }
  }

  @Test
  public void shouldReturnCorrectColumnMetadataForGet(@TempDir Path downloadDir) throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_GET_COLUMN_METADATA", testDataCsv(), true, true);

    // When File is downloaded using GET command
    String getSql = String.format("GET @%s/test_data.csv '%s/'", stageName, fileUri(downloadDir));
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet resultSet = statement.executeQuery(getSql)) {
      // Then Column metadata for GET command should be correct
      ResultSetMetaData meta = resultSet.getMetaData();
      assertEquals(5, meta.getColumnCount(), "GET should return 5 columns");
      assertTextColumn(meta, 1, "file");
      assertFixedColumn(meta, 2, "size");
      assertTextColumn(meta, 3, "status");
      assertTextColumn(meta, 4, "encryption");
      assertTextColumn(meta, 5, "message");
      assertTrue(resultSet.next(), "Expected one GET result row");
      assertEquals("DOWNLOADED", resultSet.getString(3), "Expected DOWNLOADED status");
      assertFalse(resultSet.wasNull(), "status should be non-null");
    }
  }

  private static void assertTextColumn(ResultSetMetaData meta, int column, String expectedName)
      throws Exception {
    assertEquals(
        expectedName,
        meta.getColumnName(column).toLowerCase(),
        "Unexpected name for column " + column);
    assertEquals(
        Types.VARCHAR, meta.getColumnType(column), "Expected TEXT column at position " + column);
  }

  private static void assertFixedColumn(ResultSetMetaData meta, int column, String expectedName)
      throws Exception {
    assertEquals(
        expectedName,
        meta.getColumnName(column).toLowerCase(),
        "Unexpected name for column " + column);
    // BD#54: the new driver reports PUT/GET size columns as BIGINT (standard scale-0 rule);
    // legacy hard-codes them to DECIMAL.
    int expectedType = isNewDriver() ? Types.BIGINT : Types.DECIMAL;
    assertEquals(
        expectedType, meta.getColumnType(column), "Expected FIXED column at position " + column);
  }
}
