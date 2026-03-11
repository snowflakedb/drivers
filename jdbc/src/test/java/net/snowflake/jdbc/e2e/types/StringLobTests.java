package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class StringLobTests extends SnowflakeIntegrationTestBase {
  private static final int LOB_16MB_SIZE = 16_777_216;
  private static final int LOB_128MB_SIZE = 134_217_728;
  private static final String CHARS64 =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789()";

  @Test
  public void shouldHandleLobStringAtHistorical16MbLimit() throws Exception {
    // Given Snowflake client is logged in
    // And A temporary table with VARCHAR column is created
    // When A string of 16777216 ASCII characters is generated and inserted
    // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
    // Then the result should show length 16777216
    // And the returned string should exactly match the generated string
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_string_lob_", "val VARCHAR");

    String generated = generateExpectedString(LOB_16MB_SIZE);
    insertGeneratedLiteral(connection, tableName, generated);

    assertLargeLobUsingChunkedReads(connection, tableName, LOB_16MB_SIZE, generated);
  }

  @Test
  public void shouldHandleLobStringAtMaximum128MbLimitWithIncreasedLobSize() throws Exception {
    // Given Snowflake client is logged in
    // And A temporary table with VARCHAR column is created
    // When A string of 134217728 ASCII characters is generated and inserted
    // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
    // Then the result should show length 134217728
    // And the returned string should exactly match the generated string
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_string_lob_", "val VARCHAR(134217728)");

    String generated = generateExpectedString(LOB_128MB_SIZE);
    insertGeneratedLiteral(connection, tableName, generated);

    assertLargeLobUsingChunkedReads(connection, tableName, LOB_128MB_SIZE, generated);
  }

  private void insertGeneratedLiteral(Connection connection, String tableName, String generated)
      throws Exception {
    int repeatCount = generated.length() / CHARS64.length();
    execute(
        connection,
        "INSERT INTO " + tableName + " SELECT REPEAT('" + CHARS64 + "', " + repeatCount + ")");
  }

  private static void assertLargeLobUsingChunkedReads(
      Connection connection, String tableName, int expectedLength, String expectedValue)
      throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT LENGTH(val) as len FROM " + tableName)) {
      assertTrue(resultSet.next(), "Expected one row for string LOB");
      assertEquals(expectedLength, resultSet.getLong(1), "Unexpected LOB length");
      assertFalse(resultSet.wasNull(), "LOB length should not be NULL");
      assertFalse(resultSet.next(), "Expected exactly one row for string LOB");
    }

    int chunkSize = 1_000_000;
    for (int start = 0; start < expectedLength; start += chunkSize) {
      int length = Math.min(chunkSize, expectedLength - start);
      String sql = "SELECT SUBSTR(val, " + (start + 1) + ", " + length + ") FROM " + tableName;
      String expectedChunk = expectedValue.substring(start, start + length);
      assertSingleChunk(connection, sql, expectedChunk, start);
    }
  }

  private static void assertSingleChunk(
      Connection connection, String sql, String expectedChunk, int chunkStart) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for string LOB");
      String actualChunk = resultSet.getString(1);
      assertFalse(resultSet.wasNull(), "LOB chunk should not be NULL");
      assertEquals(expectedChunk, actualChunk, "Chunk mismatch for LOB at index " + chunkStart);
      assertFalse(resultSet.next(), "Expected exactly one row for string LOB");
    }
  }

  private static String generateExpectedString(int targetLength) {
    int unitLength = CHARS64.length();
    int repeats = targetLength / unitLength;
    StringBuilder builder = new StringBuilder(targetLength);
    for (int i = 0; i < repeats; i++) {
      builder.append(CHARS64);
    }
    return builder.toString();
  }
}
