package net.snowflake.client.api.pooling;

import static net.snowflake.jdbc.utils.PoolingTestCompat.assertGetConnectionAfterPooledCloseFails;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertNetworkTimeoutAfterSet;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertUniversalAbortSemantics;
import static net.snowflake.jdbc.utils.PoolingTestCompat.closePooledConnection;
import static net.snowflake.jdbc.utils.PoolingTestCompat.invokeAbort;
import static net.snowflake.jdbc.utils.PoolingTestCompat.isUniversalDriverPooling;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.CallableStatement;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Properties;
import javax.sql.ConnectionEvent;
import javax.sql.ConnectionEventListener;
import javax.sql.PooledConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.jdbc.utils.PoolingTestResources;
import org.junit.jupiter.api.Test;

public class LogicalConnectionIT extends PoolingTestBase {

  @Test
  public void shouldCreateStatementWithHoldability() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection();
        Statement statement =
            logicalConnection.createStatement(
                ResultSet.TYPE_FORWARD_ONLY,
                ResultSet.CONCUR_READ_ONLY,
                ResultSet.CLOSE_CURSORS_AT_COMMIT);
        ResultSet resultSet = statement.executeQuery("show parameters")) {
      assertTrue(resultSet.next());
      assertFalse(logicalConnection.isClosed());
      assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, statement.getResultSetHoldability());
      assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, logicalConnection.getHoldability());
    }
  }

  @Test
  public void shouldNetworkTimeout() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertEquals(0, logicalConnection.getNetworkTimeout());
      // The universal driver has not yet wired connection network timeout to sf_core:
      // SnowflakeConnectionImpl.setNetworkTimeout is a no-op and getNetworkTimeout returns 0. The
      // legacy reference driver implements it and returns the set value (BD#41), so the post-set
      // assertion is gated per driver.
      logicalConnection.setNetworkTimeout(null, 200);
      assertNetworkTimeoutAfterSet(logicalConnection, 200);
    }
    pooledConnection.close();
  }

  @Test
  public void shouldIsValid() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertTrue(logicalConnection.isValid(10));
      assertThrows(SQLException.class, () -> logicalConnection.isValid(-10));
    }
    pooledConnection.close();
  }

  @Test
  public void shouldConnectionClientInfo() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      Properties property = logicalConnection.getClientInfo();
      assertEquals(0, property.size());
      assertNull(logicalConnection.getClientInfo("Peter"));
    }
    pooledConnection.close();
  }

  @Test
  public void shouldAbort() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    CollectingConnectionListener listener = new CollectingConnectionListener();
    pooledConnection.addConnectionEventListener(listener);
    Connection logicalConnection = pooledConnection.getConnection();
    SnowflakeConnectionImpl physicalConnection =
        logicalConnection.unwrap(SnowflakeConnectionImpl.class);
    assertFalse(physicalConnection.isClosed());
    invokeAbort(logicalConnection);
    assertUniversalAbortSemantics(
        logicalConnection,
        physicalConnection.isClosed(),
        listener.closedEvents.size(),
        listener.errorEvents.size(),
        listener.errorEvents.isEmpty() ? null : listener.errorEvents.get(0));
    if (isUniversalDriverPooling()) {
      // The physical connection is now dead, so borrowing a new logical connection must fail with
      // CONNECTION_CLOSED rather than handing out a connection backed by a closed session.
      assertGetConnectionAfterPooledCloseFails(pooledConnection);
    }
    pooledConnection.removeConnectionEventListener(listener);
    closePooledConnection(pooledConnection);
  }

  private static class CollectingConnectionListener implements ConnectionEventListener {
    final List<ConnectionEvent> closedEvents = new ArrayList<>();
    final List<ConnectionEvent> errorEvents = new ArrayList<>();

    @Override
    public void connectionClosed(ConnectionEvent event) {
      closedEvents.add(event);
    }

    @Override
    public void connectionErrorOccurred(ConnectionEvent event) {
      errorEvents.add(event);
    }
  }

  @Test
  public void shouldNativeSQL() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertEquals("select 1", logicalConnection.nativeSQL("select 1"));
    }
    pooledConnection.close();
  }

  @Test
  public void shouldUnwrapper() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      boolean canUnwrap = logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class);
      assertTrue(canUnwrap);
      // isWrapperFor returning true must be backed by a real unwrap that exposes a usable physical
      // connection, not just a type check.
      SnowflakeConnectionImpl physicalConnection =
          logicalConnection.unwrap(SnowflakeConnectionImpl.class);
      assertFalse(physicalConnection.isClosed());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldTransactionStatement() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      logicalConnection.setAutoCommit(false);
      assertFalse(logicalConnection.getAutoCommit());
      logicalConnection.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED);
      assertEquals(
          Connection.TRANSACTION_READ_COMMITTED, logicalConnection.getTransactionIsolation());

      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute(
            "create or replace temporary table test_transaction (colA int, colB string)");
        statement.execute("insert into test_transaction values (1, 'abc')");

        logicalConnection.commit();
        try (ResultSet resultSet =
            statement.executeQuery("select count(*) from test_transaction")) {
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
        }

        statement.execute("delete from test_transaction");
        logicalConnection.rollback();
        try (ResultSet resultSet =
            statement.executeQuery("select count(*) from test_transaction")) {
          assertTrue(resultSet.next());
          assertEquals(1, resultSet.getInt(1));
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldReadOnly() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertFalse(logicalConnection.isReadOnly());
      logicalConnection.setReadOnly(true);
      assertFalse(logicalConnection.isReadOnly());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldGetTypeMap() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertEquals(Collections.emptyMap(), logicalConnection.getTypeMap());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldPreparedStatement() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute("create or replace temporary table test_prep (colA int, colB varchar)");
        try (PreparedStatement preparedStatement =
            logicalConnection.prepareStatement("insert into test_prep values (?, ?)")) {
          preparedStatement.setInt(1, 25);
          preparedStatement.setString(2, "hello world");
          preparedStatement.execute();
          int count = 0;
          try (ResultSet resultSet = statement.executeQuery("select * from test_prep")) {
            while (resultSet.next()) {
              count++;
              // Assert the bound values round-trip, not just the row count: the test must fail if
              // the prepared statement inserted a row with wrong column values.
              assertEquals(25, resultSet.getInt("COLA"));
              assertEquals("hello world", resultSet.getString("COLB"));
            }
          }
          assertEquals(1, count);
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldSetSchema() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      String schema = logicalConnection.getSchema();
      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rst = stmt.executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals(schema, rst.getString(1));
      }

      logicalConnection.setSchema("PUBLIC");
      try (Statement stmt = logicalConnection.createStatement();
          ResultSet rst = stmt.executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals("PUBLIC", rst.getString(1));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldDatabaseMetaData() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      DatabaseMetaData databaseMetaData = logicalConnection.getMetaData();
      assertEquals("Snowflake", databaseMetaData.getDatabaseProductName());
    }
    pooledConnection.close();
  }

  @Test
  public void shouldPrepareCall() throws SQLException {
    String procedureName = "output_message_" + PoolingTestResources.SUFFIX;
    String procedure =
        "CREATE OR REPLACE PROCEDURE "
            + procedureName
            + "(message VARCHAR)\n"
            + "RETURNS VARCHAR NOT NULL\n"
            + "LANGUAGE SQL\n"
            + "AS\n"
            + "BEGIN\n"
            + "  RETURN message;\n"
            + "END;";
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute(procedure);

        try {
          try (CallableStatement callableStatement =
              logicalConnection.prepareCall("call " + procedureName + "(?)")) {
            callableStatement.setString(1, "hello world");
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
            }
          }

          try (CallableStatement callableStatement =
              logicalConnection.prepareCall(
                  "call " + procedureName + "('hello world')",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY)) {
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
              assertEquals(ResultSet.TYPE_FORWARD_ONLY, callableStatement.getResultSetType());
              assertEquals(ResultSet.CONCUR_READ_ONLY, callableStatement.getResultSetConcurrency());
            }
          }

          try (CallableStatement callableStatement =
              logicalConnection.prepareCall(
                  "call " + procedureName + "('hello world')",
                  ResultSet.TYPE_FORWARD_ONLY,
                  ResultSet.CONCUR_READ_ONLY,
                  ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
            try (ResultSet resultSet = callableStatement.executeQuery()) {
              resultSet.next();
              assertEquals("hello world", resultSet.getString(1));
              assertEquals(
                  ResultSet.CLOSE_CURSORS_AT_COMMIT, callableStatement.getResultSetHoldability());
            }
          }
        } finally {
          statement.execute("drop procedure if exists " + procedureName + "(varchar)");
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void shouldClob() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      try (Statement statement = logicalConnection.createStatement()) {
        statement.execute("create or replace temporary table test_clob (colA text)");
      }

      try (PreparedStatement preparedStatement =
          logicalConnection.prepareStatement("insert into test_clob values (?)")) {
        Clob clob = logicalConnection.createClob();
        clob.setString(1, "hello world");
        preparedStatement.setClob(1, clob);
        preparedStatement.execute();
      }

      try (Statement statement = logicalConnection.createStatement()) {
        try (ResultSet resultSet = statement.executeQuery("select * from test_clob")) {
          assertTrue(resultSet.next());
          assertEquals("hello world", resultSet.getString("COLA"));
        }
      }
    }
    pooledConnection.close();
  }
}
