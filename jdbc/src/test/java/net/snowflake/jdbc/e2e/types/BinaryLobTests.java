package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class BinaryLobTests extends SnowflakeIntegrationTestBase {
  private static final int DEFAULT_BINARY_LOB_SIZE = 8_388_608;
  private static final int EXTENDED_BINARY_LOB_SIZE = 67_108_864;

  @Test
  public void shouldHandleMaximumDefaultBinarySize() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with BINARY column exists
    String tableName = createTempTable(connection, "ud_binary_lob_", "val BINARY");

    // When Binary value of 8MB size (8,388,608 bytes) is inserted
    byte[] generated = generateExpectedBinary(DEFAULT_BINARY_LOB_SIZE);
    insertGeneratedBinary(connection, tableName, generated);

    // And Query "SELECT * FROM {table}" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          assertTrue(resultSet.next(), "Expected one row for binary LOB");
          byte[] actualValue = resultSet.getBytes(1);
          assertFalse(resultSet.wasNull(), "LOB value should not be NULL");

          // Then the retrieved value size should be 8MB (8,388,608 bytes)
          assertEquals(DEFAULT_BINARY_LOB_SIZE, actualValue.length, "Unexpected binary LOB length");

          // And data integrity should be maintained
          assertArrayEquals(generated, actualValue, "Fetched binary LOB value mismatch");
          assertFalse(resultSet.next(), "Expected exactly one row for binary LOB");
        });
  }

  @Test
  public void shouldHandleExtendedMaximumBinarySize() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with BINARY(67108864) column exists
    String tableName =
        createTempTable(
            connection, "ud_binary_lob_", "val BINARY(" + EXTENDED_BINARY_LOB_SIZE + ")");

    // When Binary value of 64MB size (67,108,864 bytes) is inserted
    byte[] generated = generateExpectedBinary(EXTENDED_BINARY_LOB_SIZE);
    insertGeneratedBinary(connection, tableName, generated);

    // And Query "SELECT * FROM {table}" is executed
    String sql = "SELECT * FROM " + tableName;
    withQueryResult(
        connection,
        sql,
        resultSet -> {
          assertTrue(resultSet.next(), "Expected one row for binary LOB");
          byte[] actualValue = resultSet.getBytes(1);
          assertFalse(resultSet.wasNull(), "LOB value should not be NULL");

          // Then the retrieved value size should be 64MB (67,108,864 bytes)
          assertEquals(
              EXTENDED_BINARY_LOB_SIZE, actualValue.length, "Unexpected binary LOB length");

          // And data integrity should be maintained
          assertArrayEquals(generated, actualValue, "Fetched binary LOB value mismatch");
          assertFalse(resultSet.next(), "Expected exactly one row for binary LOB");
        });
  }

  private static void insertGeneratedBinary(
      Connection connection, String tableName, byte[] generated) throws Exception {
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      preparedStatement.setBytes(1, generated);
      preparedStatement.execute();
    }
  }

  private static byte[] generateExpectedBinary(int targetLength) {
    byte[] generated = new byte[targetLength];
    for (int i = 0; i < targetLength; i++) {
      generated[i] = (byte) (i % 256);
    }
    return generated;
  }
}
