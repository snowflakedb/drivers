package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.SQLException;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class PutGetDisabledTests extends SnowflakeIntegrationTestBase implements WithPutGet {

  private static final String DISABLED_MESSAGE = "file transfers have been disabled.";

  @Test
  public void shouldRejectPutWhenFileTransfersDisabled(@TempDir Path sourceDir) throws Exception {
    // Given a connection opened with enablePutGet=false and a real local file plus a real stage
    try (Connection connection = openConnection("enablePutGet", "false")) {
      ensureDatabaseAndSchema(connection);
      Path localFile = writeTextFile(sourceDir, "disabled_put.csv", "1,2,3\n");
      String stageName = createTemporaryStage(connection, "TEST_STAGE_PUT_DISABLED");

      // When PUT is executed for that file
      String putSql = String.format("PUT '%s' @%s", fileUri(localFile), stageName);

      // Then the transfer is rejected with "File transfers have been disabled."
      SQLException exception =
          assertThrows(
              SQLException.class,
              () -> put(connection, putSql),
              "PUT should be rejected when file transfers are disabled");
      assertEquals(
          DISABLED_MESSAGE,
          exception.getMessage().toLowerCase(),
          "Unexpected message for a disabled PUT");
    }
  }

  @Test
  public void shouldRejectGetWhenFileTransfersDisabled(@TempDir Path downloadDir) throws Exception {
    // Given a connection opened with enablePutGet=false and a real stage
    try (Connection connection = openConnection("enablePutGet", "false")) {
      ensureDatabaseAndSchema(connection);
      String stageName = createTemporaryStage(connection, "TEST_STAGE_GET_DISABLED");

      // When GET is executed against that stage
      String getSql = String.format("GET @%s '%s/'", stageName, fileUri(downloadDir));

      // Then the transfer is rejected with "File transfers have been disabled."
      SQLException exception =
          assertThrows(
              SQLException.class,
              () -> get(connection, getSql),
              "GET should be rejected when file transfers are disabled");
      assertEquals(
          DISABLED_MESSAGE,
          exception.getMessage().toLowerCase(),
          "Unexpected message for a disabled GET");
    }
  }
}
