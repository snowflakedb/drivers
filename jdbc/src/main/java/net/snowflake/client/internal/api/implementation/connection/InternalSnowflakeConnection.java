package net.snowflake.client.internal.api.implementation.connection;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/** Internal interface combining JDBC Connection and SnowflakeConnection with handle access. */
public interface InternalSnowflakeConnection extends Connection, SnowflakeConnection {

  ConnectionHandle getHandle();

  /** Centralized access to session and client-only parameters for this connection. */
  ParametersRegistry getParameters();

  void checkClosed() throws SQLException;

  void removeStatement(Statement stmt);

  /**
   * Fetch results for a completed query and wrap them in a ResultSet bound to the given statement.
   */
  InternalResultSet createResultSetFromSfqid(String queryID, SnowflakeStatementImpl statement)
      throws SQLException;
}
