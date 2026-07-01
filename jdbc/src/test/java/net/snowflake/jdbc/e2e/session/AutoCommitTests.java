package net.snowflake.jdbc.e2e.session;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

class AutoCommitTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldReportAutocommitAsDisabledAfterItWasDisabledOnTheConnection() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection()) {
      // When autocommit is disabled on the connection
      conn.setAutoCommit(false);
      // Then the autocommit setting reports as disabled
      assertFalse(conn.getAutoCommit());
    }
  }

  @Test
  void shouldReportAutocommitAsEnabledAfterItWasReEnabledOnTheConnection() throws Exception {
    // Given Snowflake client is logged in
    try (Connection conn = openConnection()) {
      // And autocommit was disabled on the connection
      conn.setAutoCommit(false);
      // When autocommit is enabled on the connection
      conn.setAutoCommit(true);
      // Then the autocommit setting reports as enabled
      assertTrue(conn.getAutoCommit());
    }
  }

  @Test
  void shouldDiscardUncommittedInsertsOnRollback() throws Exception {
    // Given Snowflake client is logged in
    Connection reader = getDefaultConnection();
    try (Connection writer = openConnection()) {
      ensureDatabaseAndSchema(writer);
      // And a transient table exists in the test schema
      String table = uniqueTableName();
      execute(writer, "CREATE TRANSIENT TABLE " + table + " (id NUMBER)");
      try {
        // When the writer disables autocommit, inserts a row, and rolls back
        writer.setAutoCommit(false);
        execute(writer, "INSERT INTO " + table + " VALUES (1)");
        writer.rollback();
        // Then a reader session sees zero rows
        assertRowCount(reader, table, 0);
      } finally {
        // DDL auto-commits in Snowflake regardless of session autocommit, so DROP
        // closes any open transaction and removes the table without an explicit reset.
        execute(writer, "DROP TABLE IF EXISTS " + table);
      }
    }
  }

  @Test
  void shouldPublishCommittedInsertsToOtherSessions() throws Exception {
    // Given Snowflake client is logged in
    Connection reader = getDefaultConnection();
    try (Connection writer = openConnection()) {
      ensureDatabaseAndSchema(writer);
      // And a transient table exists in the test schema
      String table = uniqueTableName();
      execute(writer, "CREATE TRANSIENT TABLE " + table + " (id NUMBER)");
      try {
        // When the writer disables autocommit, inserts a row, and commits
        writer.setAutoCommit(false);
        execute(writer, "INSERT INTO " + table + " VALUES (1)");
        writer.commit();
        // Then a reader session sees one row
        assertRowCount(reader, table, 1);
      } finally {
        execute(writer, "DROP TABLE IF EXISTS " + table);
      }
    }
  }

  private static String uniqueTableName() {
    return "AUTOCOMMIT_E2E_" + UUID.randomUUID().toString().replace("-", "");
  }

  private void assertRowCount(Connection conn, String table, int expected) throws Exception {
    withQueryResult(
        conn,
        "SELECT COUNT(*) FROM " + table,
        rs -> {
          rs.next();
          assertEquals(expected, rs.getInt(1));
        });
  }
}
