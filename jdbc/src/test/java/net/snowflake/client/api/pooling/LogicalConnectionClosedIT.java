package net.snowflake.client.api.pooling;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.PooledConnection;
import net.snowflake.client.api.exception.ErrorCode;
import org.junit.jupiter.api.Test;

public class LogicalConnectionClosedIT extends PoolingTestBase {

  @Test
  public void testLogicalConnectionAlreadyClosed() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    Connection logicalConnection = pooledConnection.getConnection();
    logicalConnection.close();

    expectConnectionClosed(logicalConnection::getMetaData);
    expectConnectionClosed(logicalConnection::getAutoCommit);
    expectConnectionClosed(logicalConnection::commit);
    expectConnectionClosed(logicalConnection::rollback);
    expectConnectionClosed(logicalConnection::isReadOnly);
    expectConnectionClosed(logicalConnection::getCatalog);
    expectConnectionClosed(logicalConnection::getSchema);
    expectConnectionClosed(logicalConnection::getTransactionIsolation);
    expectConnectionClosed(logicalConnection::getWarnings);
    expectConnectionClosed(logicalConnection::clearWarnings);
    expectConnectionClosed(() -> logicalConnection.nativeSQL("select 1"));
    expectConnectionClosed(() -> logicalConnection.setAutoCommit(false));
    expectConnectionClosed(() -> logicalConnection.setReadOnly(false));
    expectConnectionClosed(() -> logicalConnection.setCatalog("fakedb"));
    expectConnectionClosed(() -> logicalConnection.setSchema("fakedb"));
    expectConnectionClosed(
        () -> logicalConnection.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED));
    expectConnectionClosed(() -> logicalConnection.createArrayOf("faketype", null));

    pooledConnection.close();
  }

  private void expectConnectionClosed(Runnable0 f) {
    SQLException ex = assertThrows(SQLException.class, f::run);
    assertEquals(ErrorCode.CONNECTION_CLOSED.getMessageCode(), ex.getErrorCode());
  }

  @FunctionalInterface
  interface Runnable0 {
    void run() throws SQLException;
  }
}