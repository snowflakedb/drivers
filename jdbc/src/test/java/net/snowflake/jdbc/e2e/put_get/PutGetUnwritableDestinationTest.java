package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.SQLException;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetUnwritableDestinationTest extends SnowflakeIntegrationTestBase
    implements WithPutGet {

  @Test
  public void shouldThrowSQLExceptionWhenGetLocalDestinationCannotBeWritten(@TempDir Path workDir)
      throws Exception {
    // Given A stage with one uploaded file to GET back.
    Path sourceFile = writeTextFile(workDir, "b2_source.csv", "1,2,3\n");
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_GET_UNWRITABLE", sourceFile, false, true);

    // And A local GET destination that cannot be created: a directory nested under an existing
    // regular file. create_dir_all then fails with ENOTDIR regardless of process uid — root-proof
    // (CI containers often run as root, so a read-only dir would be bypassed).
    Path blockingFile = workDir.resolve("blocking_file");
    Files.write(blockingFile, new byte[] {0});
    Path unwritableDest = blockingFile.resolve("subdir");

    // When GET writes the staged file into the unwritable local destination.
    String getSql =
        String.format(
            "GET @%s/%s '%s/'", stageName, sourceFile.getFileName(), fileUri(unwritableDest));

    // Then A SQLException is thrown (fail-fast GET throws; it does not return an empty rowset).
    SQLException exception =
        assertThrows(
            SQLException.class,
            () -> get(getDefaultConnection(), getSql),
            "Expected a SQLException when the GET destination cannot be written");
    String message = exception.getMessage();
    assertNotNull(message, "SQLException message should be present");
    assertFalse(message.isEmpty(), "SQLException message should not be empty");
    // Literals, not ErrorCode.FILE_OPERATION_DOWNLOAD_ERROR: this test also runs on the old
    // driver, whose getMessageCode returns Integer, so a call compiled against this driver's
    // int accessor fails there with NoSuchMethodError.
    assertEquals(200067, exception.getErrorCode(), "errorCode (FILE_OPERATION_DOWNLOAD_ERROR)");
    assertEquals("XX000", exception.getSQLState(), "SQLState (SqlState.INTERNAL_ERROR)");
  }
}
