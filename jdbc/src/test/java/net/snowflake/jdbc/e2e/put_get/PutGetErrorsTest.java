package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Path;
import java.sql.SQLException;
import java.util.List;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * JDBC e2e mirror of {@code tests/definitions/shared/put_get/put_get_errors.feature}.
 *
 * <p>A missing local source (PUT) surfaces vendor code {@code 200008}. A missing staged object
 * (GET) returns an empty result set — the JDBC-only scenario; Python and core raise instead.
 */
public class PutGetErrorsTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private static final int FILE_NOT_FOUND = 200008;

  @Test
  public void shouldReturnErrorWhenPuttingNonexistentLocalFile() throws Exception {
    // Given A stage is created
    String stageName = createTemporaryStage(getDefaultConnection(), "TEST_STAGE_PUT_ERR");

    // When PUT is executed with a path to a nonexistent local file
    String nonexistentPath =
        "/tmp/nonexistent_file_" + UUID.randomUUID().toString().replace("-", "") + ".csv";
    String putSql = String.format("PUT 'file://%s' @%s", nonexistentPath, stageName);

    // Then An error is raised indicating the local file does not exist
    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> put(getDefaultConnection(), putSql),
            "Expected a missing-local-file error");
    assertEquals(FILE_NOT_FOUND, exception.getErrorCode(), "Unexpected Snowflake vendor code");
    assertEquals("22000", exception.getSQLState(), "Unexpected SQLSTATE");
  }

  @Test
  public void shouldReturnEmptyResultSetWhenGettingNonexistentFileFromStage(
      @TempDir Path downloadDir) throws Exception {
    // Given An empty stage is created
    String stageName = createTemporaryStage(getDefaultConnection(), "TEST_STAGE_GET_ERR");

    // When GET is executed for a file that does not exist in stage
    String nonexistentFile =
        "nonexistent_file_" + UUID.randomUUID().toString().replace("-", "") + ".csv";
    String getSql =
        String.format("GET @%s/%s '%s/'", stageName, nonexistentFile, fileUri(downloadDir));

    // Then An empty result set is returned
    List<GetRow> rows = get(getDefaultConnection(), getSql);
    assertEquals(
        0, rows.size(), "Expected an empty GET result set when the staged file is missing");
  }
}
