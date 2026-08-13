package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class PutGetOverwriteTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  private Path originalFile() {
    return sharedTestDataDir().resolve("overwrite").resolve("original").resolve("test_data.csv");
  }

  private Path updatedFile() {
    return sharedTestDataDir().resolve("overwrite").resolve("updated").resolve("test_data.csv");
  }

  @Test
  public void shouldOverwriteFileWhenOverwriteIsSetToTrue() throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_PUT_GET_OVERWRITE_TRUE", originalFile(), false, true);

    // When Updated file is uploaded with OVERWRITE set to true
    PutRow updated =
        uploadFileToStage(getDefaultConnection(), stageName, updatedFile(), false, true);

    // Then UPLOADED status is returned
    assertEquals("UPLOADED", updated.status, "Expected the overwriting PUT to be UPLOADED");

    // And File was overwritten
    assertStagedRow(stageName, "updated");
  }

  @Test
  public void shouldNotOverwriteFileWhenOverwriteIsSetToFalse() throws Exception {
    // Given File is uploaded to stage
    String stageName =
        createStageAndUploadFile(
            getDefaultConnection(), "TEST_PUT_GET_OVERWRITE_FALSE", originalFile(), false, true);

    // When Updated file is uploaded with OVERWRITE set to false
    PutRow updated =
        uploadFileToStage(getDefaultConnection(), stageName, updatedFile(), false, false);

    // Then SKIPPED status is returned
    assertEquals("SKIPPED", updated.status, "Expected the second PUT to be SKIPPED");

    // And File was not overwritten
    assertStagedRow(stageName, "original");
  }

  private void assertStagedRow(String stageName, String expectedFirstColumn) throws Exception {
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT $1, $2, $3 FROM @" + stageName)) {
      assertTrue(resultSet.next(), "Expected one row from the staged file");
      assertEquals(expectedFirstColumn, resultSet.getString(1), "Unexpected first column");
      assertEquals("test", resultSet.getString(2), "Unexpected second column");
      assertEquals("data", resultSet.getString(3), "Unexpected third column");
    }
  }
}
