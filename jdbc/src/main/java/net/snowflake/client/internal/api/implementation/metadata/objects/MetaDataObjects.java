package net.snowflake.client.internal.api.implementation.metadata.objects;

import static net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataResultSetFormat.GET_CATALOGS;

import java.sql.ResultSet;
import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.resultset.RowConverter;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;

/**
 * Owns the query-backed {@link java.sql.DatabaseMetaData} methods: building the {@code SHOW}
 * command, running it, filtering/projecting rows, and fabricating the JDBC-shaped result set. Keeps
 * {@link SnowflakeDatabaseMetaDataImpl} a thin delegating shell.
 */
@RequiredArgsConstructor
public class MetaDataObjects {

  // TODO(SNOW-3695645): maybe we should use rpc GetConnectionObjects instead of querying
  //  Then we can move escaping, etc. to the core and avoid those operations in wrapper.

  private final SnowflakeConnectionImpl connection;

  public ResultSet getCatalogs() throws SQLException {
    SnowflakeStatementImpl statement =
        connection.createStatement().unwrap(SnowflakeStatementImpl.class);
    try {
      String sqlQuery = "show databases in account";
      ResultSet showResult = statement.executeQuery(sqlQuery);
      String queryId = statement.getQueryID();
      SnowflakeResultSetMetaDataImpl metaData = GET_CATALOGS.metaData(queryId);
      RowConverter rowConverter = row -> new Object[] {row.getString("name")};

      return ResultSetFactory.wrapWithConverter(
          statement, showResult.unwrap(SnowflakeResultSetImpl.class), metaData, rowConverter);
    } catch (Throwable e) {
      statement.close();
      throw e;
    }
  }
}
