package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * Runs under {@code test} (native driver) and {@code referenceTest} (snowflake-jdbc 4.4.0). Both
 * drivers return an empty generated-keys result whose getters return type defaults; they differ
 * only in {@code unwrap}/{@code isWrapperFor} for {@code ResultSet}.
 */
public class GeneratedKeysTests extends SnowflakeIntegrationTestBase {

  @Test
  public void shouldReportGeneratedKeysUnsupportedInMetadata() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When the database metadata is read
    // Then generated keys are reported as unsupported
    assertFalse(connection.getMetaData().supportsGetGeneratedKeys());
  }

  @Test
  public void shouldReturnEmptyGeneratedKeysBeforeAnyExecute() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When getGeneratedKeys is called on a statement that has not executed
    try (Statement statement = connection.createStatement();
        ResultSet keys = statement.getGeneratedKeys()) {
      // Then the result matches the empty generated-keys stub
      assertEmptyGeneratedKeys(keys);
    }
  }

  @Test
  public void shouldReturnEmptyGeneratedKeysAfterInsert() throws Exception {
    // Given Snowflake client is logged in and a temporary table exists
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_genkeys_", "id INT");

    // When a row is inserted with NO_GENERATED_KEYS
    try (PreparedStatement statement =
        connection.prepareStatement(
            "INSERT INTO " + tableName + " VALUES (?)", Statement.NO_GENERATED_KEYS)) {
      statement.setInt(1, 7);
      assertEquals(1, statement.executeUpdate());

      // Then getGeneratedKeys is the empty generated-keys result
      try (ResultSet keys = statement.getGeneratedKeys()) {
        assertEmptyGeneratedKeys(keys);
      }
    }
  }

  @Test
  public void shouldTreatNavigationAndUpdatesAsStubCalls() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When non-getter calls are made on the generated-keys result
    try (Statement statement = connection.createStatement();
        ResultSet keys = statement.getGeneratedKeys()) {
      // Then they behave as the 4.x stub: no-ops, zeros, and nulls
      assertFalse(keys.first());
      assertFalse(keys.absolute(1));
      assertEquals(0, keys.getRow());
      keys.setFetchDirection(ResultSet.FETCH_REVERSE);
      keys.updateInt(1, 7);
      assertEquals(0, keys.getType());
      assertEquals(0, keys.getConcurrency());
      assertNull(keys.getCursorName());
      assertNull(keys.unwrap(Statement.class));
      assertFalse(keys.isWrapperFor(Statement.class));
    }
  }

  @Test
  public void shouldRejectPrepareStatementWithReturnGeneratedKeys() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When prepareStatement is called with RETURN_GENERATED_KEYS
    // Then the driver rejects the flag
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () ->
            connection.prepareStatement(
                "INSERT INTO t VALUES (?)", Statement.RETURN_GENERATED_KEYS));
  }

  @Test
  public void shouldRejectExecuteWithReturnGeneratedKeys() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When execute is called with RETURN_GENERATED_KEYS
    try (Statement statement = connection.createStatement()) {
      // Then the driver rejects the flag
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> statement.execute("INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () ->
              statement.executeUpdate("INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () ->
              statement.executeLargeUpdate(
                  "INSERT INTO t VALUES (1)", Statement.RETURN_GENERATED_KEYS));
    }
  }

  @Test
  public void shouldExecuteLargeUpdateWhenGeneratedKeysFlagIsNone() throws Exception {
    // Given Snowflake client is logged in and a temporary table exists
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_genkeys_", "id INT");

    // When executeLargeUpdate is called with NO_GENERATED_KEYS
    try (Statement statement = connection.createStatement()) {
      assertEquals(
          1L,
          statement.executeLargeUpdate(
              "INSERT INTO " + tableName + " VALUES (1)", Statement.NO_GENERATED_KEYS));
      // Then the statement reports the long update count and getGeneratedKeys is empty
      assertEquals(1L, statement.getLargeUpdateCount());
      try (ResultSet keys = statement.getGeneratedKeys()) {
        assertEmptyGeneratedKeys(keys);
      }
    }
  }

  private static void assertEmptyGeneratedKeys(ResultSet keys) throws SQLException {
    assertFalse(keys.isBeforeFirst());
    assertFalse(keys.isAfterLast());
    assertFalse(keys.isLast());
    assertFalse(keys.next(), "Expected no generated-key rows");
    assertFalse(keys.isClosed(), "Generated-keys result set stays open until closed");
    assertEquals(0, keys.getInt(1));
    assertFalse(keys.wasNull());
    assertEquals(0L, keys.getLong(1));
    assertEquals("", keys.getString("col1"));
    assertFalse(keys.wasNull());
    assertNull(keys.getMetaData());
    assertNull(keys.getStatement());
  }
}
