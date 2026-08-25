package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
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

public class PutGetNoSpaceTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  @Test
  public void shouldThrowSQLExceptionWhenGetLocalDestinationCannotBeWritten(@TempDir Path workDir)
      throws Exception {
    // Given A stage with one uploaded file to GET back.
    Path sourceFile = writeTextFile(workDir, "b2_source.csv", "1,2,3\n");
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_STAGE_GET_NOSPACE", sourceFile, false, true);

    // And A local GET destination that cannot be created: a directory nested under an existing
    // regular file. create_dir_all then fails with ENOTDIR regardless of process uid — root-proof
    // (CI containers often run as root, so a read-only dir would be bypassed). This exercises the
    // same FileManagerError::Io channel a full disk (ENOSPC) would.
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
    // The old driver surfaces legacy's diagnostic identity for a failed download. The new driver
    // leaves ErrorKind::Io unmapped (errorCode 0 / null SQLState); assert the legacy contract on
    // the old driver and omit it for the new driver until the vendor code is mapped.
    // TODO: SNOW-4010477 map ErrorKind::Io, then drop this isNewDriver() guard.
    if (!isNewDriver()) {
      assertEquals(
          200067, exception.getErrorCode(), "errorCode (ErrorCode.FILE_OPERATION_DOWNLOAD_ERROR)");
      assertEquals("XX000", exception.getSQLState(), "SQLState (SqlState.INTERNAL_ERROR)");
    }
  }
}
