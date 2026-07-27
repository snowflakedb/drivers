package net.snowflake.client.api.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * Transaction isolation behavior across drivers. Snowflake supports exactly one isolation level
 * (READ COMMITTED) and never acts on the value passed to {@code setTransactionIsolation}.
 *
 * <p>Both drivers store the last set value and return it, defaulting to {@code TRANSACTION_NONE}
 * before any call (BD#18 fixed).
 */
class TransactionIsolationTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldReportTransactionNoneByDefault() throws Exception {
    try (Connection conn = openConnection()) {
      assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
    }
  }

  @Test
  void shouldRoundTripTransactionNone() throws Exception {
    try (Connection conn = openConnection()) {
      conn.setTransactionIsolation(Connection.TRANSACTION_NONE);
      assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
    }
  }

  @Test
  void shouldReportReadCommittedAfterSettingReadCommitted() throws Exception {
    // Given a connection explicitly set to READ COMMITTED (the one level Snowflake runs at)
    try (Connection conn = openConnection()) {
      conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED);
      // Then both drivers report READ COMMITTED
      assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
    }
  }

  @Test
  void shouldRejectUnsupportedLevel() throws Exception {
    // Given a connection
    try (Connection conn = openConnection()) {
      // Then both drivers reject a level Snowflake does not support with the same exception —
      // this is not a BD#18 divergence: legacy also validates and throws here, it only differs
      // in what it stores/reports for the accepted NONE/READ_COMMITTED levels.
      assertThrows(
          SQLFeatureNotSupportedException.class,
          () -> conn.setTransactionIsolation(Connection.TRANSACTION_SERIALIZABLE));
    }
  }
}
