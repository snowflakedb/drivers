package net.snowflake.client.api.pooling;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Clob;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Collections;
import java.util.Properties;
import javax.sql.PooledConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import org.junit.jupiter.api.Test;

public class LogicalConnectionIT extends PoolingTestBase {

  @Test
  public void testCreateStatementWithHoldability() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    Connection logicalConnection = pooledConnection.getConnection();
    try (Statement statement =
        logicalConnection.createStatement(
            ResultSet.TYPE_FORWARD_ONLY,
            ResultSet.CONCUR_READ_ONLY,
            ResultSet.CLOSE_CURSORS_AT_COMMIT)) {
      try (ResultSet resultSet = statement.executeQuery("show parameters")) {
        assertTrue(resultSet.next());
        assertFalse(logicalConnection.isClosed());
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, logicalConnection.getHoldability());
      }
    }
    logicalConnection.close();
    assertTrue(logicalConnection.isClosed());
    pooledConnection.close();
  }

  @Test
  public void testNetworkTimeout() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      int millis = logicalConnection.getNetworkTimeout();
      assertEquals(0, millis);
      logicalConnection.setNetworkTimeout(null, 200);
      assertEquals(200, logicalConnection.getNetworkTimeout());
    }
    pooledConnection.close();
  }

  @Test
  public void testIsValid() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertTrue(logicalConnection.isValid(10));
      assertThrows(SQLException.class, () -> logicalConnection.isValid(-10));
    }
    pooledConnection.close();
  }

  @Test
  public void testConnectionClientInfo() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      Properties property = logicalConnection.getClientInfo();
      assertEquals(0, property.size());
      assertNull(logicalConnection.getClientInfo("Peter"));
    }
    pooledConnection.close();
  }

  @Test
  public void testAbort() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    Connection logicalConnection = pooledConnection.getConnection();
    assertFalse(logicalConnection.isClosed());
    logicalConnection.abort(null);
    // After abort, the logical connection should report closed
    // and getting a new logical connection from the pooled connection should fail
    // because the physical connection is closed
    try {
      Connection logicalConnection2 = pooledConnection.getConnection();
      // Physical connection was closed by abort, so new logical connections may see it closed
      assertTrue(logicalConnection2.isClosed());
    } catch (SQLException e) {
      // Expected - physical connection may be closed
    }
  }

  @Test
  public void testNativeSQL() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertEquals("select 1", logicalConnection.nativeSQL("select 1"));
    }
    pooledConnection.close();
  }

  @Test
  public void testUnwrapper() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      boolean canUnwrap = logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class);
      assertTrue(canUnwrap);
    }
    pooledConnection.close();
  }

  @Test
  public void testTransactionStatement() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      logicalConnection.setAutoCommit(false);
      assertFalse(logicalConnection.getAutoCommit());
      logicalConnection.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED);
      assertEquals(2, logicalConnection.getTransactionIsolation());

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
  public void testReadOnly() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertFalse(logicalConnection.isReadOnly());
      logicalConnection.setReadOnly(true);
      assertFalse(logicalConnection.isReadOnly());
    }
    pooledConnection.close();
  }

  @Test
  public void testGetTypeMap() throws Throwable {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      assertEquals(Collections.emptyMap(), logicalConnection.getTypeMap());
    }
    pooledConnection.close();
  }

  @Test
  public void testPreparedStatement() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
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
            }
          }
          assertEquals(1, count);
        }
      }
    }
    pooledConnection.close();
  }

  @Test
  public void testSetSchema() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      String schema = logicalConnection.getSchema();
      try (ResultSet rst =
          logicalConnection.createStatement().executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals(schema, rst.getString(1));
      }

      logicalConnection.setSchema("PUBLIC");
      try (ResultSet rst =
          logicalConnection.createStatement().executeQuery("select current_schema()")) {
        assertTrue(rst.next());
        assertEquals("PUBLIC", rst.getString(1));
      }
    }
    pooledConnection.close();
  }

  @Test
  public void testDatabaseMetaData() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      DatabaseMetaData databaseMetaData = logicalConnection.getMetaData();
      assertEquals("Snowflake", databaseMetaData.getDatabaseProductName());
    }
    pooledConnection.close();
  }

  @Test
  public void testClob() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
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

      try (Statement statement = logicalConnection.createStatement();
          ResultSet resultSet = statement.executeQuery("select * from test_clob")) {
        assertTrue(resultSet.next());
        assertEquals("hello world", resultSet.getString("COLA"));
      }
    }
    pooledConnection.close();
  }
}
