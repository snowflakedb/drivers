package net.snowflake.client.api.connection;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
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
 * <p>BD#18: the universal driver reports the truth — {@code getTransactionIsolation()} always
 * returns {@code TRANSACTION_READ_COMMITTED}, regardless of what was set. Legacy snowflake-jdbc
 * stores the last set value and returns it, defaulting to {@code TRANSACTION_NONE} before any call.
 */
class TransactionIsolationTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldReportReadCommittedByDefaultOnNewDriverAndNoneOnLegacy() throws Exception {
    // Given a fresh connection with no setTransactionIsolation call
    try (Connection conn = openConnection()) {
      // Then the reported level differs between drivers (BD#18)
      if (isNewDriver()) {
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      } else {
        assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
      }
    }
  }

  @Test
  void shouldStillReportReadCommittedAfterSettingNoneOnNewDriverButRoundTripOnLegacy()
      throws Exception {
    // Given a connection whose isolation level is set to NONE
    try (Connection conn = openConnection()) {
      conn.setTransactionIsolation(Connection.TRANSACTION_NONE);
      // Then the new driver ignores the set value and still reports READ COMMITTED, while
      // legacy round-trips the stored value (BD#18)
      if (isNewDriver()) {
        assertEquals(Connection.TRANSACTION_READ_COMMITTED, conn.getTransactionIsolation());
      } else {
        assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
      }
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
