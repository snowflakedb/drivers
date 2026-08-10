package net.snowflake.client.internal.api.implementation;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.api.datasource.SnowflakeDataSource;
import net.snowflake.client.internal.api.decorator.AbstractDecorator;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.datasource.DecoratedSnowflakeBasicDataSource;
import net.snowflake.client.internal.api.implementation.datasource.SnowflakeBasicDataSource;
import net.snowflake.client.internal.api.implementation.resultset.DecoratedSnowflakeAsyncResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.DecoratedSnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeAsyncResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.DecoratedSnowflakeStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;

/**
 * Wraps a raw impl in its generated {@code Decorated*} boundary before it leaves an impl method. A
 * boundary method that hands back another JDBC object (a {@link Statement} from a {@link
 * Connection}, a {@link ResultSet} from a {@link Statement}, …) must return the decorated child, or
 * it silently escapes the telemetry + exception-translation boundary; these factories are the
 * single place that happens.
 *
 * <p>No raw impl extends {@link AbstractDecorator}, so {@code instanceof AbstractDecorator} is a
 * sufficient "already decorated" guard, keeping pooling / {@code LogicalConnection} paths
 * idempotent. Dispatch is most-derived first ({@code Callable extends Prepared extends Statement}).
 */
public final class Decorators {

  private Decorators() {}

  public static Statement statement(Statement statement, Telemetry telemetry) {
    if (statement == null || statement instanceof AbstractDecorator) {
      return statement;
    }
    if (statement instanceof SnowflakeCallableStatementImpl) {
      return new DecoratedSnowflakeCallableStatementImpl(
          (SnowflakeCallableStatementImpl) statement, telemetry);
    }
    if (statement instanceof SnowflakePreparedStatementImpl) {
      return new DecoratedSnowflakePreparedStatementImpl(
          (SnowflakePreparedStatementImpl) statement, telemetry);
    }
    if (statement instanceof SnowflakeStatementImpl) {
      return new DecoratedSnowflakeStatementImpl((SnowflakeStatementImpl) statement, telemetry);
    }
    return statement;
  }

  public static ResultSet resultSet(ResultSet resultSet, Telemetry telemetry) {
    if (resultSet == null || resultSet instanceof AbstractDecorator) {
      return resultSet;
    }
    if (resultSet instanceof SnowflakeAsyncResultSetImpl) {
      return new DecoratedSnowflakeAsyncResultSetImpl(
          (SnowflakeAsyncResultSetImpl) resultSet, telemetry);
    }
    if (resultSet instanceof SnowflakeResultSetImpl) {
      return new DecoratedSnowflakeResultSetImpl((SnowflakeResultSetImpl) resultSet, telemetry);
    }
    return resultSet;
  }

  public static SnowflakeDataSource dataSource(
      SnowflakeBasicDataSource dataSource, Telemetry telemetry) {
    return new DecoratedSnowflakeBasicDataSource(dataSource, telemetry);
  }

  public static Telemetry telemetryOf(Connection connection) {
    try {
      return connection.unwrap(SnowflakeConnectionImpl.class).getTelemetry();
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  public static Telemetry telemetryOf(SnowflakeStatementImpl statement) {
    if (statement == null) {
      return Telemetry.NOOP;
    }
    InternalSnowflakeConnection connection = statement.getConnectionInternal();
    return connection == null ? Telemetry.NOOP : connection.getTelemetry();
  }

  public static Connection connection(InternalSnowflakeConnection connection, Telemetry telemetry) {
    if (connection == null || connection instanceof AbstractDecorator) {
      return (Connection) connection;
    }
    // Memoized on the impl so every decoration of the same connection returns the same wrapper.
    return ((SnowflakeConnectionImpl) connection).decoratedSelf(telemetry);
  }
}
