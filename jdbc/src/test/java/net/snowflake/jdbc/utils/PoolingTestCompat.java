package net.snowflake.jdbc.utils;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.ConnectionEvent;
import javax.sql.PooledConnection;
import net.snowflake.client.api.exception.ErrorCode;

/**
 * Helpers for pooling integration/e2e tests that also run against the legacy {@code snowflake-jdbc}
 * artifact via the {@code referenceTest} task.
 */
public final class PoolingTestCompat {

  /** Vendor code for {@link ErrorCode#CONNECTION_CLOSED} on both drivers. */
  public static final int CONNECTION_CLOSED_VENDOR_CODE = 200052;

  /** SQLState for {@link ErrorCode#CONNECTION_CLOSED} on both drivers. */
  public static final String CONNECTION_CLOSED_SQL_STATE = "08003";

  private PoolingTestCompat() {}

  /**
   * {@code true} when the universal-driver pooling implementation is on the classpath ({@code test}
   * task); {@code false} for {@code referenceTest}, which swaps in the legacy {@code
   * snowflake-jdbc} JAR whose {@code LogicalConnection} semantics differ.
   */
  public static boolean isUniversalDriverPooling() {
    return DriverCompatibility.isNewDriver();
  }

  public static void assertConnectionClosed(SQLException ex) {
    assertEquals(CONNECTION_CLOSED_VENDOR_CODE, ex.getErrorCode());
    assertEquals(CONNECTION_CLOSED_SQL_STATE, ex.getSQLState());
  }

  /**
   * Asserts the network timeout observed after {@code setNetworkTimeout(setValue)}. The universal
   * driver returns 0. The legacy driver returns the value stored on the connection (BD#41). HTTP
   * does not read that field on either driver.
   */
  public static void assertNetworkTimeoutAfterSet(Connection connection, int setValue)
      throws SQLException {
    if (isUniversalDriverPooling()) {
      assertEquals(0, connection.getNetworkTimeout());
    } else {
      assertEquals(setValue, connection.getNetworkTimeout());
    }
  }

  /** Universal driver closes both handles after abort; legacy abort semantics differ (BD#40). */
  public static void assertPhysicalConnectionClosedAfterAbort(
      Connection physicalConnection, Connection logicalConnection) throws SQLException {
    if (!isUniversalDriverPooling()) {
      return;
    }
    assertTrue(physicalConnection.isClosed(), "physical connection should be closed after abort");
    assertTrue(logicalConnection.isClosed(), "logical handle should be closed after abort");
  }

  /**
   * Closes a pooled connection. Legacy {@code SnowflakePooledConnection.close()} may throw when the
   * physical connection was already aborted because it probes {@code getSessionID()} before closing
   * (the legacy abort-then-close quirk related to BD#40); that legacy-only {@code SQLException} is
   * swallowed while the universal driver rethrows.
   */
  public static void closePooledConnection(PooledConnection pooledConnection) throws SQLException {
    try {
      pooledConnection.close();
    } catch (SQLException e) {
      if (isUniversalDriverPooling()) {
        throw e;
      }
    }
  }

  /**
   * Invokes {@link Connection#abort(Executor)} for tests. Legacy abort semantics differ (BD#40) and
   * may throw when aborting a closed logical handle or when the physical {@code abort()} fails.
   */
  public static void invokeAbort(Connection logicalConnection) throws SQLException {
    try {
      logicalConnection.abort(null);
    } catch (SQLException e) {
      if (isUniversalDriverPooling()) {
        throw e;
      }
    }
  }

  /**
   * Asserts that an operation on a closed logical handle is rejected with {@code
   * CONNECTION_CLOSED}. Methods that differ on the legacy driver are client info (BD#29), unwrap
   * and isWrapperFor (BD#33), and abort (BD#40). This helper runs on the universal driver only.
   */
  public static void assertThrowsConnectionClosed(SqlRunnable action) {
    if (isUniversalDriverPooling()) {
      assertConnectionClosed(assertThrows(SQLException.class, action::run));
    }
  }

  /**
   * Both drivers return false from {@code isValid} after logical {@code close()} returns.
   * snowflake-jdbc short-circuits on the closed flag and does not probe the physical connection
   * (BD#34).
   */
  public static void assertIsValidFalseOnClosedHandle(Connection logicalConnection)
      throws SQLException {
    assertFalse(logicalConnection.isValid(10));
  }

  /**
   * Universal driver throws {@code CONNECTION_CLOSED}; legacy driver throws {@code
   * NullPointerException} when borrowing from a closed pooled connection (BD#27).
   */
  public static void assertGetConnectionAfterPooledCloseFails(PooledConnection pooledConnection)
      throws SQLException {
    if (isUniversalDriverPooling()) {
      assertConnectionClosed(assertThrows(SQLException.class, pooledConnection::getConnection));
    } else {
      assertThrows(NullPointerException.class, pooledConnection::getConnection);
    }
  }

  /**
   * Universal driver suppresses {@code connectionErrorOccurred} for unsupported JDBC features; the
   * legacy driver may fire an error event per unsupported call (BD#26).
   */
  public static void assertNoConnectionErrorEventsForUnsupportedFeatures(int errorEventCount) {
    if (isUniversalDriverPooling()) {
      assertEquals(
          0,
          errorEventCount,
          "SQLFeatureNotSupportedException must not fire connectionErrorOccurred");
    }
  }

  /**
   * Universal driver closes both handles and signals pool eviction via {@code
   * connectionErrorOccurred}; legacy abort semantics are covered by BD#40 and are not asserted
   * here.
   */
  public static void assertUniversalAbortSemantics(
      Connection logicalConnection,
      boolean physicalClosed,
      int closedEventCount,
      int errorEventCount,
      ConnectionEvent errorEvent) {
    if (!isUniversalDriverPooling()) {
      return;
    }
    assertTrue(physicalClosed, "physical connection should be closed after abort");
    try {
      assertTrue(logicalConnection.isClosed(), "logical handle should be closed after abort");
    } catch (SQLException e) {
      throw new AssertionError("logicalConnection.isClosed() failed", e);
    }
    assertEquals(0, closedEventCount, "abort must not fire connectionClosed");
    assertEquals(1, errorEventCount, "abort must fire connectionErrorOccurred");
    assertNotNull(errorEvent.getSQLException());
  }

  @FunctionalInterface
  public interface SqlRunnable {
    void run() throws SQLException;
  }
}
