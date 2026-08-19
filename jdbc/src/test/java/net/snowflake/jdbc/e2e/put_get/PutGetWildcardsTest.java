package net.snowflake.jdbc.e2e.put_get;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * JDBC e2e mirror of {@code tests/definitions/shared/put_get/put_get_wildcards.feature}.
 *
 * <p>Covers local-glob expansion on PUT ({@code ?} and {@code *}) and server-side regex selection
 * on GET ({@code PATTERN=}); each asserts that only the intended files cross the wire.
 */
public class PutGetWildcardsTest extends SnowflakeIntegrationTestBase implements WithPutGet {

  @Test
  public void shouldUploadFilesThatMatchWildcardQuestionMarkPattern(@TempDir Path uploadDir)
      throws Exception {
    String baseName = "test_put_wildcard_question_mark";

    // Given Files matching wildcard pattern
    List<String> matchingFiles = createNumberedFiles(uploadDir, baseName);

    // And Files not matching wildcard pattern
    List<String> nonMatchingFiles = Arrays.asList(baseName + "_10.csv", baseName + "_abc.csv");
    for (String name : nonMatchingFiles) {
      writeTextFile(uploadDir, name, "1,2,3\n");
    }

    // When Files are uploaded using command with question mark wildcard
    String stageName =
        createTemporaryStage(getDefaultConnection(), "TEST_PUT_WILDCARD_QUESTION_MARK");
    List<PutRow> uploaded =
        putWildcard(
            getDefaultConnection(),
            stageName,
            posixGlob(uploadDir, baseName + "_?.csv"),
            false,
            true);

    // Then Files matching wildcard pattern are uploaded
    assertEquals(5, uploaded.size(), "Expected the five single-character matches to upload");
    for (PutRow row : uploaded) {
      assertEquals("UPLOADED", row.status, "Every matched file should upload");
    }
    List<String> stagedNames = stagedBaseNames(getDefaultConnection(), stageName);
    for (String name : matchingFiles) {
      assertTrue(stagedNames.contains(name), "Expected " + name + " on the stage");
    }

    // And Files not matching wildcard pattern are not uploaded
    for (String name : nonMatchingFiles) {
      assertFalse(stagedNames.contains(name), "Did not expect " + name + " on the stage");
    }
  }

  @Test
  public void shouldUploadFilesThatMatchWildcardStarPattern(@TempDir Path uploadDir)
      throws Exception {
    String baseName = "test_put_wildcard_star";

    // Given Files matching wildcard pattern
    List<String> matchingFiles = createNumberedFiles(uploadDir, baseName);

    // And Files not matching wildcard pattern
    List<String> nonMatchingFiles = Arrays.asList(baseName + ".csv", baseName + "_test.txt");
    for (String name : nonMatchingFiles) {
      writeTextFile(uploadDir, name, "1,2,3\n");
    }

    // When Files are uploaded using command with star wildcard
    String stageName = createTemporaryStage(getDefaultConnection(), "TEST_PUT_WILDCARD_STAR");
    List<PutRow> uploaded =
        putWildcard(
            getDefaultConnection(),
            stageName,
            posixGlob(uploadDir, baseName + "_*.csv"),
            false,
            true);

    // Then Files matching wildcard pattern are uploaded
    assertEquals(
        5, uploaded.size(), "Expected the five underscore-suffixed .csv matches to upload");
    for (PutRow row : uploaded) {
      assertEquals("UPLOADED", row.status, "Every matched file should upload");
    }
    List<String> stagedNames = stagedBaseNames(getDefaultConnection(), stageName);
    for (String name : matchingFiles) {
      assertTrue(stagedNames.contains(name), "Expected " + name + " on the stage");
    }

    // And Files not matching wildcard pattern are not uploaded
    for (String name : nonMatchingFiles) {
      assertFalse(stagedNames.contains(name), "Did not expect " + name + " on the stage");
    }
  }

  @Test
  public void shouldDownloadFilesThatAreMatchingWildcardPattern(
      @TempDir Path uploadDir, @TempDir Path downloadDir) throws Exception {
    String baseName = "test_get";

    // Given Files matching wildcard pattern are uploaded
    List<String> matchingFiles = createNumberedFiles(uploadDir, baseName);
    String stageName = createTemporaryStage(getDefaultConnection(), "TEST_GET_WILDCARD");
    for (String name : matchingFiles) {
      uploadFileToStage(getDefaultConnection(), stageName, uploadDir.resolve(name), true, true);
    }

    // And Files not matching wildcard pattern are uploaded
    List<String> nonMatchingFiles = Arrays.asList(baseName + "_10.csv", baseName + "_abc.csv");
    for (String name : nonMatchingFiles) {
      writeTextFile(uploadDir, name, "1,2,3\n");
      uploadFileToStage(getDefaultConnection(), stageName, uploadDir.resolve(name), true, true);
    }

    // When Files are downloaded using command with wildcard
    List<GetRow> downloaded =
        getWithPattern(
            getDefaultConnection(), stageName, ".*/" + baseName + "_.\\.csv\\.gz", downloadDir);

    // Then Files matching wildcard pattern are downloaded
    assertEquals(
        5, downloaded.size(), "Expected only the five single-character matches to download");
    List<String> downloadedNames = new ArrayList<>();
    for (GetRow row : downloaded) {
      downloadedNames.add(baseName(row.file));
    }
    for (String name : matchingFiles) {
      assertTrue(downloadedNames.contains(name + ".gz"), "Expected " + name + ".gz downloaded");
    }

    // And Files not matching wildcard pattern are not downloaded
    for (String name : nonMatchingFiles) {
      assertFalse(
          downloadedNames.contains(name + ".gz"), "Did not expect " + name + ".gz downloaded");
    }
  }
}
