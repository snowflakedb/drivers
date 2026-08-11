package net.snowflake.client.internal.api.implementation.connection;

import java.sql.Connection;
import java.sql.Statement;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.util.DelegatingWrapper;

/**
 * Internal interface combining JDBC Connection and SnowflakeConnection with handle access. Extends
 * {@link DelegatingWrapper} so callers holding this type get de-checked {@code unwrap}/{@code
 * isWrapperFor}.
 */
public interface InternalSnowflakeConnection
    extends Connection, SnowflakeConnection, DelegatingWrapper {

  ConnectionHandle getHandle();

  /**
   * Wrapper-telemetry emitter for this connection, threaded into the decorators of the children it
   * hands out so their boundary calls report api-usage and wrapper-errors to core.
   */
  Telemetry getTelemetry();

  /** Centralized access to session and client-only parameters for this connection. */
  ParametersRegistry getParameters();

  void checkClosed();

  /** Narrows {@link Connection#isClosed()} to drop {@code throws SQLException} */
  @Override
  boolean isClosed();

  /**
   * Narrows {@link SnowflakeConnection#getQueryStatus(String)} to drop {@code throws SQLException}
   */
  @Override
  QueryStatus getQueryStatus(String queryID);

  void removeStatement(Statement stmt);

  /**
   * Fetch results for a completed query and wrap them in a ResultSet bound to the given statement.
   */
  InternalResultSet createResultSetFromSfqid(String queryID, SnowflakeStatementImpl statement);

  /**
   * Public boundary: returns a decorated {@link Statement}. Internal callers that need the concrete
   * impl (casts, narrowed return type) must use {@link #createStatementInternal()} instead.
   */
  @Override
  Statement createStatement();

  /** Raw, undecorated statement for internal use — see {@link #createStatement()}. */
  SnowflakeStatementImpl createStatementInternal();

  @Override
  String getCatalog();

  @Override
  String getSchema();

  @Override
  String getDatabaseVersion();
}
