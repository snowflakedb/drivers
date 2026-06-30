package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.List;
import java.util.regex.Pattern;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataParams.ContextAwareMetadataSearch;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.resultset.RowConverter;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.common.util.Wildcard;

/**
 * Owns the query-backed {@link java.sql.DatabaseMetaData} methods: building the {@code SHOW}
 * command, running it, filtering/projecting rows, and fabricating the JDBC-shaped result set. Keeps
 * {@link SnowflakeDatabaseMetaDataImpl} a thin delegating shell.
 */
public class MetaDataObjects {

  // TODO(SNOW-3695645): maybe we should use rpc GetConnectionObjects instead of querying
  //  Then we can move escaping, etc. to the core and avoid those operations in wrapper.

  // TODO(SNOW-3695645): using column labels is cleaner than positional arguments, consider changing

  private static final SFLogger logger = SFLoggerFactory.getLogger(MetaDataObjects.class);

  /** Snowflake vendor code for "Object does not exist, or operation cannot be performed." */
  private static final int OBJECT_DOES_NOT_EXIST_VENDOR_CODE = 2043;

  private static final String TABLE_TYPE_TABLE = "TABLE";
  private static final String TABLE_TYPE_VIEW = "VIEW";
  private static final List<String> SUPPORTED_TABLE_TYPES =
      Arrays.asList(TABLE_TYPE_TABLE, TABLE_TYPE_VIEW);

  private final SnowflakeConnectionImpl connection;
  private final MetaDataParams params;

  public MetaDataObjects(SnowflakeConnectionImpl connection, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.params = new MetaDataParams(connection, coreDriverApi);
  }

  public ResultSet getCatalogs() throws SQLException {
    String sqlQuery = queryBuilder().show("databases").inAccount().build();
    RowConverter rowConverter = row -> new Object[] {row.getString("name")};
    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_CATALOGS);
  }

  public ResultSet getSchemas(String originalCatalog, String originalSchemaPattern)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        params.applySessionContext(originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();
    boolean isExactSchema = contextAware.isExactSchema();

    MetaDataQueryBuilder sqlQueryBuilder = queryBuilder(contextAware).show("schemas");
    if (isExactSchema
        && schemaPattern != null
        && params.isEnableWildcardsInShowMetadataCommands()) {
      String escapedSchemaPattern =
          schemaPattern.replaceAll("_", "\\\\\\\\_").replaceAll("%", "\\\\\\\\%");
      sqlQueryBuilder.likeWithWildcards(escapedSchemaPattern);
    } else {
      sqlQueryBuilder.like(schemaPattern);
    }
    String sqlQuery = sqlQueryBuilder.in(catalog).build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_SCHEMAS);
    }

    logger.debug("SQL query in getSchemas: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    RowConverter rowConverter =
        row -> {
          String schemaName = row.getString(2);
          String dbName = row.getString(5);
          if (compiledSchemaPattern == null
              || compiledSchemaPattern.matcher(schemaName).matches()
              || isExactSchema && schemaPattern.equals(schemaName)) {
            return new Object[] {schemaName, dbName};
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_SCHEMAS);
  }

  private MetaDataQueryBuilder queryBuilder() throws SQLException {
    return new MetaDataQueryBuilder(false, false, params.isEnableWildcardsInShowMetadataCommands());
  }

  private MetaDataQueryBuilder queryBuilder(ContextAwareMetadataSearch ctx) throws SQLException {
    return new MetaDataQueryBuilder(
        ctx.isExactSchema(),
        ctx.isUseSessionSchema(),
        params.isEnableWildcardsInShowMetadataCommands());
  }

  private ResultSet createResultSet(
      String sqlQuery, RowConverter rowConverter, MetaDataResultSetFormat rsFormat)
      throws SQLException {
    SnowflakeStatementImpl statement =
        connection.createStatement().unwrap(SnowflakeStatementImpl.class);
    try {
      ResultSet showResult = statement.executeQuery(sqlQuery);
      String queryId = statement.getQueryID();
      SnowflakeResultSetMetaDataImpl metaData = rsFormat.metaData(queryId);

      return ResultSetFactory.wrapWithConverter(
          statement, showResult.unwrap(SnowflakeResultSetImpl.class), metaData, rowConverter);
    } catch (Throwable e) {
      statement.close();
      if (isMissingMetadataObject(e)) {
        return emptyResultSet(rsFormat);
      }
      throw e;
    }
  }

  private static boolean isMissingMetadataObject(Throwable error) {
    for (SQLException sqlException = findSQLException(error);
        sqlException != null;
        sqlException = sqlException.getNextException()) {
      if (sqlException.getErrorCode() == OBJECT_DOES_NOT_EXIST_VENDOR_CODE) {
        return true;
      }
    }
    return false;
  }

  private static SQLException findSQLException(Throwable error) {
    Throwable current = error;
    while (current != null) {
      if (current instanceof SQLException) {
        return (SQLException) current;
      }
      current = current.getCause();
    }
    return null;
  }

  private ResultSet emptyResultSet(MetaDataResultSetFormat format) throws SQLException {
    SnowflakeStatementImpl statement =
        connection.createStatement().unwrap(SnowflakeStatementImpl.class);
    try {
      return ResultSetFactory.createEmpty(statement, format.metaData(null), true);
    } catch (SQLException | RuntimeException e) {
      statement.close();
      throw e;
    }
  }
}
