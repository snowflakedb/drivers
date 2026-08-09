package net.snowflake.client.internal.api.implementation.statement;

import java.sql.ResultSet;
import java.sql.Statement;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;

public interface InternalStatement extends Statement, SnowflakeStatement {
  @Override
  void close();

  /**
   * Public boundary: returns a decorated {@link ResultSet}. Internal callers that need the concrete
   * result set (casts, narrowed return type) must use {@link #executeQueryInternal(String)}.
   */
  @Override
  ResultSet executeQuery(String sql);

  /** Raw, undecorated result set for internal use — see {@link #executeQuery(String)}. */
  InternalResultSet executeQueryInternal(String sql);
}
