package net.snowflake.client.internal.api.implementation.pooling;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.PooledConnection;
import net.snowflake.client.api.pooling.SnowflakeConnectionPoolDataSource;
import net.snowflake.client.internal.api.implementation.datasource.SnowflakeBasicDataSource;

public class SnowflakePooledConnectionDataSource extends SnowflakeBasicDataSource
    implements SnowflakeConnectionPoolDataSource {
  @Override
  public PooledConnection getPooledConnection() throws SQLException {
    return wrap(super.getConnection());
  }

  @Override
  public PooledConnection getPooledConnection(String user, String password) throws SQLException {
    return wrap(super.getConnection(user, password));
  }

  /**
   * Wraps an already-open physical connection in a {@link SnowflakePooledConnection}, closing the
   * physical connection if the wrapper cannot be created so the just-opened session is not leaked.
   */
  private static PooledConnection wrap(Connection connection) throws SQLException {
    try {
      return new SnowflakePooledConnection(connection);
    } catch (SQLException | RuntimeException e) {
      try {
        connection.close();
      } catch (SQLException closeError) {
        e.addSuppressed(closeError);
      }
      throw e;
    }
  }
}
