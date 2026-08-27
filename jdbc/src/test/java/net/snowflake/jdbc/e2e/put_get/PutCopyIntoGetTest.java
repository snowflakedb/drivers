package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutCopyIntoGetTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  @Test
  public void shouldReportUpdateCountsAndFileCountForPutCopyIntoGet(
      @TempDir Path workDir, @TempDir Path downloadDir) throws Exception {
    Connection connection = getDefaultConnection();
    String table = "TEST_COPY_" + UUID.randomUUID().toString().replace("-", "").toUpperCase();
    String fileName = "test_copy.csv";
    String content = "1,2,str1\n3,4,str2\n";

    try (Statement statement = connection.createStatement()) {
      // Given a fresh table and a local CSV file
      statement.execute("create or replace table " + table + " (c1 number, c2 number, c3 string)");
      assertEquals(0, statement.getUpdateCount(), "CREATE TABLE should report update count 0");
      Path csv = writeTextFile(workDir, fileName, content);

      // When the file is PUT onto the table stage
      List<PutRow> putRows = new ArrayList<>();
      try (ResultSet resultSet = statement.executeQuery("PUT '" + fileUri(csv) + "' @%" + table)) {
        while (resultSet.next()) {
          putRows.add(new PutRow(resultSet));
        }
      }

      // Then PUT reports exactly one uploaded file
      assertEquals(1, putRows.size(), "PUT should upload exactly one file");
      assertEquals(fileName, putRows.get(0).source, "Unexpected uploaded file name");
      assertEquals("UPLOADED", putRows.get(0).status, "Unexpected PUT status");
      // PUT produces a result set: new driver reports the JDBC-spec -1, legacy the stale prior
      // count (CREATE TABLE's 0 here). See BD#58.
      assertEquals(
          isNewDriver() ? -1 : 0, statement.getUpdateCount(), "Unexpected PUT update count");

      // When the staged file is COPied into the table
      int copiedRows = statement.executeUpdate("copy into " + table);

      // Then COPY INTO reports the summed rows_loaded (2), matching legacy snowflake-jdbc. Read
      // before any further query resets the update count.
      assertEquals(2, copiedRows, "Unexpected COPY INTO update count");
      assertEquals(2, statement.getUpdateCount(), "Unexpected COPY INTO update count");
      try (ResultSet count = statement.executeQuery("select count(*) from " + table)) {
        assertTrue(count.next(), "COUNT(*) should return a row");
        assertEquals(2, count.getInt(1), "COPY INTO should load two rows into the table");
      }

      // When the file is GET back from the table stage
      List<GetRow> getRows =
          get(connection, "GET @%" + table + " '" + fileUri(downloadDir) + "/' parallel=8");

      // Then GET downloads one file whose round-trip content is preserved
      assertEquals(1, getRows.size(), "GET should download exactly one file");
      assertEquals("DOWNLOADED", getRows.get(0).status, "Unexpected GET status");
      Path downloaded = downloadDir.resolve(fileName + ".gz");
      assertTrue(Files.exists(downloaded), "Expected the compressed file on disk");
      assertEquals(
          content, readTextMaybeGzip(downloaded), "Round-trip content should be preserved");
    } finally {
      try (Statement cleanup = connection.createStatement()) {
        cleanup.execute("drop table if exists " + table);
      }
    }
  }
}
