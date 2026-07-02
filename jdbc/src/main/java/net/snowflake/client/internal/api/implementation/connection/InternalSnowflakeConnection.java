package net.snowflake.client.internal.api.implementation.connection;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/** Internal interface combining JDBC Connection and SnowflakeConnection with handle access. */
public interface InternalSnowflakeConnection extends Connection, SnowflakeConnection {

  ConnectionHandle getHandle();

  /**
   * The resolved client connection {@link Properties} (URL params merged with the {@code
   * Properties} bag). Used to read client-only session properties that the server never echoes —
   * e.g. {@code JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC} — ahead of the server parameter map
   * (SNOW-3243330).
   */
  Properties getResolvedProperties();

  void removeStatement(Statement stmt);

  /**
   * Fetch results for a completed query and wrap them in a ResultSet bound to the given statement.
   */
  InternalResultSet createResultSetFromSfqid(String queryID, SnowflakeStatementImpl statement)
      throws SQLException;
}
