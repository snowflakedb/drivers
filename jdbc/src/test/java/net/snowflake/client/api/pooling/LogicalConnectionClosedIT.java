package net.snowflake.client.api.pooling;

import static java.sql.Connection.TRANSACTION_READ_COMMITTED;
import static net.snowflake.jdbc.utils.PoolingTestCompat.assertIsValidFalseOnClosedHandle;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.PooledConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.jdbc.utils.PoolingTestCompat;
import org.junit.jupiter.api.Test;

public class LogicalConnectionClosedIT extends PoolingTestBase {

  @Test
  public void shouldLogicalConnectionAlreadyClosed() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = trackPooledConnection(poolDataSource.getPooledConnection());
    try (Connection logicalConnection = pooledConnection.getConnection()) {
      // Close up front so the assertions below exercise the closed-state guards. The
      // try-with-resources close at block exit is an idempotent no-op.
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
          () -> logicalConnection.setTransactionIsolation(TRANSACTION_READ_COMMITTED));
      expectConnectionClosed(() -> logicalConnection.createArrayOf("faketype", null));
      expectConnectionClosed(logicalConnection::createStatement);
      expectConnectionClosed(() -> logicalConnection.prepareStatement("select 1"));
      expectConnectionClosed(() -> logicalConnection.prepareCall("call foo()"));
      expectConnectionClosed(() -> logicalConnection.unwrap(SnowflakeConnectionImpl.class));
      expectConnectionClosed(() -> logicalConnection.isWrapperFor(SnowflakeConnectionImpl.class));
      expectConnectionClosed(logicalConnection::getNetworkTimeout);
      expectConnectionClosed(() -> logicalConnection.setNetworkTimeout(null, 100));
      expectConnectionClosed(logicalConnection::getHoldability);
      expectConnectionClosed(
          () -> logicalConnection.setHoldability(java.sql.ResultSet.CLOSE_CURSORS_AT_COMMIT));
      expectConnectionClosed(logicalConnection::getTypeMap);
      expectConnectionClosed(logicalConnection::createClob);
      expectConnectionClosed(logicalConnection::createBlob);
      expectConnectionClosed(() -> logicalConnection.getClientInfo("k"));
      expectConnectionClosed(logicalConnection::getClientInfo);
      // setClientInfo on a closed handle throws SQLClientInfoException carrying the
      // CONNECTION_CLOSED code (a SQLException subtype), so the shared assertion still applies.
      expectConnectionClosed(() -> logicalConnection.setClientInfo("k", "v"));

      // isValid() on a closed handle is a no-throw false (universal driver); legacy may still
      // probe.
      assertIsValidFalseOnClosedHandle(logicalConnection);
      if (PoolingTestCompat.isUniversalDriverPooling()) {
        PoolingTestCompat.invokeAbort(logicalConnection);
      }
    }

    PoolingTestCompat.closePooledConnection(pooledConnection);
  }
}
