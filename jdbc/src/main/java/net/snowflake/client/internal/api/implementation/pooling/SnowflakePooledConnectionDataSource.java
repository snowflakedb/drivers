package net.snowflake.client.internal.api.implementation.pooling;

import java.sql.Connection;
import java.sql.SQLException;
import javax.sql.PooledConnection;
import net.snowflake.client.api.pooling.SnowflakeConnectionPoolDataSource;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.datasource.SnowflakeBasicDataSource;
import net.snowflake.client.internal.codegen.JdbcBoundary;

@JdbcBoundary
public class SnowflakePooledConnectionDataSource extends SnowflakeBasicDataSource
    implements SnowflakeConnectionPoolDataSource {

  // Explicit UID: this ConnectionPoolDataSource is the type typically bound in JNDI, so pin the
  // serialized form (the Serializable base class is SnowflakeBasicDataSource).
  private static final long serialVersionUID = 1L;

  @Override
  public PooledConnection getPooledConnection() {
    return wrap(super.getConnection());
  }

  @Override
  public PooledConnection getPooledConnection(String user, String password) {
    return wrap(super.getConnection(user, password));
  }

  /**
   * Wraps an already-open physical connection in a {@link SnowflakePooledConnection}, closing the
   * physical connection if the wrapper cannot be created so the just-opened session is not leaked.
   */
  private static PooledConnection wrap(Connection connection) {
    try {
      SnowflakePooledConnection pooled = new SnowflakePooledConnection(connection);
      DecoratedSnowflakePooledConnection decorated =
          new DecoratedSnowflakePooledConnection(pooled, Decorators.telemetryOf(connection));
      // Listeners register on the decorator (the object the application holds); make the raw impl
      // report it as ConnectionEvent.getSource() so events match the registered source.
      pooled.setEventSource(decorated);
      return decorated;
    } catch (RuntimeException e) {
      try {
        connection.close();
      } catch (SQLException closeError) {
        e.addSuppressed(closeError);
      }
      throw e;
    }
  }
}
