package net.snowflake.client.api.pooling;

import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Savepoint;
import java.sql.Statement;
import java.util.HashMap;
import javax.sql.PooledConnection;
import org.junit.jupiter.api.Test;

public class LogicalConnectionFeatureNotSupportedIT extends PoolingTestBase {

  @Test
  public void testLogicalConnectionFeatureNotSupported() throws SQLException {
    SnowflakeConnectionPoolDataSource poolDataSource = createConfiguredPoolDataSource();
    PooledConnection pooledConnection = poolDataSource.getPooledConnection();
    Connection logicalConnection = pooledConnection.getConnection();

    expectFeatureNotSupported(() -> logicalConnection.rollback(new FakeSavepoint()));
    expectFeatureNotSupported(
        () -> logicalConnection.setTransactionIsolation(Connection.TRANSACTION_SERIALIZABLE));
    expectFeatureNotSupported(
        () -> logicalConnection.setTransactionIsolation(Connection.TRANSACTION_REPEATABLE_READ));
    expectFeatureNotSupported(
        () -> logicalConnection.prepareStatement("select 1", new int[] {1, 2}));
    expectFeatureNotSupported(
        () -> logicalConnection.prepareStatement("select 1", new String[] {"c1", "c2"}));
    expectFeatureNotSupported(
        () ->
            logicalConnection.prepareStatement(
                "select 1", ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY));
    expectFeatureNotSupported(
        () ->
            logicalConnection.prepareStatement(
                "select 1",
                ResultSet.TYPE_SCROLL_SENSITIVE,
                ResultSet.CONCUR_READ_ONLY,
                ResultSet.HOLD_CURSORS_OVER_COMMIT));
    expectFeatureNotSupported(
        () ->
            logicalConnection.createStatement(
                ResultSet.TYPE_SCROLL_SENSITIVE, ResultSet.CONCUR_READ_ONLY));
    expectFeatureNotSupported(() -> logicalConnection.setTypeMap(new HashMap<>()));
    expectFeatureNotSupported(logicalConnection::setSavepoint);
    expectFeatureNotSupported(() -> logicalConnection.setSavepoint("fake"));
    expectFeatureNotSupported(() -> logicalConnection.releaseSavepoint(new FakeSavepoint()));
    expectFeatureNotSupported(logicalConnection::createBlob);
    expectFeatureNotSupported(logicalConnection::createNClob);
    expectFeatureNotSupported(logicalConnection::createSQLXML);
    expectFeatureNotSupported(
        () -> logicalConnection.setHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT));
    expectFeatureNotSupported(() -> logicalConnection.createStruct("fakeType", new Object[] {}));
    expectFeatureNotSupported(
        () -> logicalConnection.prepareStatement("select 1", Statement.RETURN_GENERATED_KEYS));

    logicalConnection.close();
    pooledConnection.close();
  }

  private void expectFeatureNotSupported(SQLErrorThrowingRunnable f) {
    SQLException ex = assertThrows(SQLException.class, f::run);
    assertInstanceOf(SQLFeatureNotSupportedException.class, ex,
        "Expected SQLFeatureNotSupportedException but got " + ex.getClass().getName());
  }

  static class FakeSavepoint implements Savepoint {
    @Override
    public int getSavepointId() throws SQLException {
      return 0;
    }

    @Override
    public String getSavepointName() throws SQLException {
      return "";
    }
  }
}
