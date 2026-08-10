package net.snowflake.client.api.statement;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.List;

/** This interface defines Snowflake specific APIs for Statement */
public interface SnowflakeStatement {
  /**
   * @return the Snowflake query ID of the latest executed query (even failed one) or null when the
   *     last query ID is not available
   * @throws SQLException if an error is encountered
   */
  String getQueryID() throws SQLException;

  /**
   * Returns the Snowflake query IDs of the latest executed batch.
   *
   * <ul>
   *   <li>{@link java.sql.Statement} batch: one ID per submitted entry, in order; {@code null} for
   *       failed entries (preserves positional alignment with {@code executeBatch()} counts).
   *   <li>{@link java.sql.PreparedStatement} array-bind batch: a single ID covering all rows.
   * </ul>
   *
   * <p>Populated at the start of every {@code executeBatch()}; {@code clearBatch()} does NOT clear
   * it.
   *
   * @return non-null list, possibly containing {@code null} entries for failed iterations
   * @throws SQLException if an error is encountered
   */
  List<String> getBatchQueryIDs() throws SQLException;

  /**
   * Set statement level parameter
   *
   * @param name parameter name
   * @param value parameter value
   * @throws SQLException if an error is encountered
   */
  void setParameter(String name, Object value) throws SQLException;

  /**
   * @param batchID the batch ID
   * @deprecated No-op. Only ever tagged client-side telemetry in legacy snowflake-jdbc; never
   *     affected query execution.
   */
  @Deprecated
  void setBatchID(String batchID);

  /**
   * Execute SQL query asynchronously
   *
   * @param sql sql statement
   * @return ResultSet
   * @throws SQLException if @link{#executeQueryInternal(String, Map)} throws an exception
   */
  // Should we return AsyncResultSet here? It would have to extend ResultSet
  ResultSet executeAsyncQuery(String sql) throws SQLException;

  /**
   * Sets the query timeout when running an async query.
   *
   * @param seconds The number of seconds until timeout.
   * @throws SQLException if an error is encountered
   */
  void setAsyncQueryTimeout(int seconds) throws SQLException;
}
