package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * JDBC-specific coverage for {@link ResultSet#getString(String)} over a BINARY column, which
 * renders the value using the session's BINARY_OUTPUT_FORMAT. This class validates the JDBC API
 * behavior directly and is not mapped to a shared feature file.
 */
public class ResultSetBinaryOutputFormatTests extends SnowflakeIntegrationTestBase {

  // Columns are read by index: snowflake-jdbc matches labels case-sensitively against the
  // server-uppercased name, so a getString("bin") lookup would fail the reference run.
  private static final String BINARY_LITERAL_SQL = "SELECT X'0123456789ABCDEF' AS bin";

  @Test
  public void shouldRenderBinaryAsHexWhenBinaryOutputFormatIsHex() throws Exception {
    // Given Snowflake client is logged in with BINARY_OUTPUT_FORMAT = HEX
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET BINARY_OUTPUT_FORMAT = 'HEX'");

      // When a BINARY column is read as a String
      withQueryResult(
          connection,
          BINARY_LITERAL_SQL,
          resultSet -> {
            assertTrue(resultSet.next(), "Expected one row");

            // Then each byte is rendered as two uppercase hex characters
            assertEquals("0123456789ABCDEF", resultSet.getString(1));
            assertFalse(resultSet.wasNull(), "A non-null BINARY value should not report wasNull()");
          });
    }
  }

  @Test
  public void shouldRenderBinaryAsBase64WhenBinaryOutputFormatIsBase64() throws Exception {
    // Given Snowflake client is logged in with BINARY_OUTPUT_FORMAT = BASE64
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET BINARY_OUTPUT_FORMAT = 'BASE64'");

      // When a BINARY column is read as a String
      withQueryResult(
          connection,
          BINARY_LITERAL_SQL,
          resultSet -> {
            assertTrue(resultSet.next(), "Expected one row");

            // Then the value is rendered as padded standard Base64
            assertEquals("ASNFZ4mrze8=", resultSet.getString(1));
            assertFalse(resultSet.wasNull(), "A non-null BINARY value should not report wasNull()");
          });
    }
  }
}
