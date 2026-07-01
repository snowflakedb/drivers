package net.snowflake.client.internal.api.implementation.metadata.objects;

import static java.sql.ResultSetMetaData.columnNoNulls;
import static java.sql.ResultSetMetaData.columnNullable;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.regex.Pattern;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataParams.ContextAwareMetadataSearch;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.resultset.RowConverter;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeColumnMetadata;
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

  private static final ObjectMapper mapper = new ObjectMapper();

  /** Snowflake vendor code for "Object does not exist, or operation cannot be performed." */
  private static final int OBJECT_DOES_NOT_EXIST_VENDOR_CODE = 2043;

  // Legacy snowflake-jdbc executeAndReturnEmptyResultIfNotFound() SQL states.
  private static final String SQL_STATE_NO_DATA = "02000";
  private static final String SQL_STATE_BASE_TABLE_OR_VIEW_NOT_FOUND = "42S02";

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

  public ResultSet getTables(
      String originalCatalog,
      String originalSchemaPattern,
      final String tableNamePattern,
      final String[] types)
      throws SQLException {
    List<String> tableTypes = validateTableTypes(types);
    if (tableTypes.isEmpty()) {
      return emptyResultSet(MetaDataResultSetFormat.GET_TABLES);
    }

    ContextAwareMetadataSearch contextAware =
        params.applySessionContext(originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    boolean viewOnly = tableTypes.size() == 1 && "VIEW".equalsIgnoreCase(tableTypes.get(0));
    boolean tableOnly = tableTypes.size() == 1 && "TABLE".equalsIgnoreCase(tableTypes.get(0));
    String showType;
    if (viewOnly) {
      showType = "views";
    } else if (tableOnly) {
      showType = "tables";
    } else showType = "objects";

    String sqlQuery =
        queryBuilder(contextAware)
            .show(showType)
            .like(tableNamePattern)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_TABLES);
    }

    logger.debug("SQL query in getTables: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledTablePattern = Wildcard.toRegexPattern(tableNamePattern, true);
    RowConverter rowConverter =
        row -> {
          String tableName = row.getString(2);

          String dbName;
          String schemaName;
          String kind;
          String comment;

          if (viewOnly) {
            dbName = row.getString(4);
            schemaName = row.getString(5);
            kind = "VIEW";
            comment = row.getString(7);
          } else {
            dbName = row.getString(3);
            schemaName = row.getString(4);
            kind = row.getString(5);
            comment = row.getString(6);
          }

          if ((compiledTablePattern == null || compiledTablePattern.matcher(tableName).matches())
              && (compiledSchemaPattern == null
                  || compiledSchemaPattern.matcher(schemaName).matches())) {
            return new Object[] {
              dbName, schemaName, tableName, kind, comment, null, null, null, null, null
            };
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_TABLES);
  }

  public ResultSet getColumns(
      String originalCatalog,
      String originalSchemaPattern,
      String tableNamePattern,
      String columnNamePattern,
      boolean extendedSet)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        params.applySessionContext(originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    MetaDataResultSetFormat resultFormat =
        extendedSet
            ? MetaDataResultSetFormat.GET_COLUMNS_EXTENDED_SET
            : MetaDataResultSetFormat.GET_COLUMNS;

    String sqlQuery =
        queryBuilder(contextAware)
            .show("columns")
            .like(columnNamePattern)
            .in(catalog, schemaPattern, tableNamePattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(resultFormat);
    }

    logger.debug("SQL query in getColumns: {}", sqlQuery);

    boolean jdbcTreatDecimalAsInt = params.isJdbcTreatDecimalAsInt();
    boolean enableReturnTimestampWithTimeZone = params.isEnableReturnTimestampWithTimeZone();
    boolean stringsQuoted = params.isStringsQuoted();

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledTablePattern = Wildcard.toRegexPattern(tableNamePattern, true);
    Pattern compiledColumnPattern = Wildcard.toRegexPattern(columnNamePattern, true);
    ColumnOrdinalTracker ordinalTracker = new ColumnOrdinalTracker();
    RowConverter rowConverter =
        row -> {
          String tableName = row.getString(1);
          String schemaName = row.getString(2);
          String columnName = row.getString(3);
          String dataTypeStr = row.getString(4);
          String defaultValue = row.getString(6);
          if (defaultValue != null) {
            defaultValue = defaultValue.trim();
          }
          if (defaultValue == null || defaultValue.isEmpty()) {
            defaultValue = null;
          } else if (!stringsQuoted) {
            if (defaultValue.startsWith("'") && defaultValue.endsWith("'")) {
              defaultValue = defaultValue.substring(1, defaultValue.length() - 1);
              defaultValue = defaultValue.replace("''", "'");
            }
          }
          String comment = row.getString(9);
          String catalogName = row.getString(10);
          String autoIncrement = row.getString(11);

          if ((compiledTablePattern == null || compiledTablePattern.matcher(tableName).matches())
              && (compiledSchemaPattern == null
                  || compiledSchemaPattern.matcher(schemaName).matches())
              && (compiledColumnPattern == null
                  || compiledColumnPattern.matcher(columnName).matches())) {

            int ordinalPosition = ordinalTracker.nextOrdinalFor(tableName);

            JsonNode jsonNode;
            try {
              jsonNode = mapper.readTree(dataTypeStr);
            } catch (Exception ex) {
              logger.error("Exception when parsing column result", ex);
              throw new SnowflakeSQLException(
                  ErrorCode.INTERNAL_ERROR, "error parsing data type: " + dataTypeStr);
            }
            SnowflakeColumnMetadata columnMetadata =
                new SnowflakeColumnMetadata(jsonNode, jdbcTreatDecimalAsInt);

            Object[] nextRow = new Object[25];
            nextRow[0] = catalogName;
            nextRow[1] = schemaName;
            nextRow[2] = tableName;
            nextRow[3] = columnName;

            int internalColumnType = columnMetadata.getType();
            int externalColumnType = internalColumnType;

            if (internalColumnType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ) {
              externalColumnType = Types.TIMESTAMP;
            }
            if (internalColumnType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ) {
              externalColumnType =
                  enableReturnTimestampWithTimeZone
                      ? Types.TIMESTAMP_WITH_TIMEZONE
                      : Types.TIMESTAMP;
            }

            nextRow[4] = externalColumnType;
            nextRow[5] = columnMetadata.getTypeName();

            nextRow[6] = getColumnSize(columnMetadata);
            nextRow[7] = null;
            nextRow[8] = columnMetadata.getScale();
            nextRow[9] = null;
            nextRow[10] = (columnMetadata.isNullable() ? columnNullable : columnNoNulls);

            nextRow[11] = comment;
            nextRow[12] = defaultValue;
            nextRow[13] = externalColumnType;
            nextRow[14] = null;
            nextRow[15] =
                (columnMetadata.getType() == Types.VARCHAR
                        || columnMetadata.getType() == Types.CHAR)
                    ? columnMetadata.getLength()
                    : null;
            nextRow[16] = ordinalPosition;

            nextRow[17] = (columnMetadata.isNullable() ? "YES" : "NO");
            nextRow[18] = null;
            nextRow[19] = null;
            nextRow[20] = null;
            nextRow[21] = null;
            nextRow[22] = "".equals(autoIncrement) ? "NO" : "YES";
            nextRow[23] = "NO";
            if (extendedSet) {
              nextRow[24] = columnMetadata.getBase().name();
            }
            return nextRow;
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, resultFormat);
  }

  /** Ported from snowflake-jdbc SnowflakeDatabaseMetaDataImpl. */
  static Integer getColumnSize(SnowflakeColumnMetadata columnMetadata) {
    switch (columnMetadata.getType()) {
      case Types.CHAR:
      case Types.VARCHAR:
      case Types.BINARY:
      case Types.VARBINARY:
        return columnMetadata.getLength();
      case Types.DECIMAL:
      case Types.NUMERIC:
      case Types.BIGINT:
      case Types.INTEGER:
      case Types.SMALLINT:
      case Types.TINYINT:
      case Types.FLOAT:
      case Types.DOUBLE:
      case Types.REAL:
      case Types.DATE:
      case Types.TIME:
      case Types.TIMESTAMP:
      case Types.TIMESTAMP_WITH_TIMEZONE:
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ:
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ:
      case SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ:
      case SnowflakeType.EXTRA_TYPES_DECFLOAT:
        return columnMetadata.getPrecision();
      case SnowflakeType.EXTRA_TYPES_VECTOR:
        return columnMetadata.getDimension();
      default:
        return null;
    }
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
      String sqlState = sqlException.getSQLState();
      if (SQL_STATE_NO_DATA.equals(sqlState)
          || SQL_STATE_BASE_TABLE_OR_VIEW_NOT_FOUND.equals(sqlState)) {
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

  private static List<String> validateTableTypes(String[] types) {
    List<String> inputValidTableTypes = new ArrayList<>();
    if (types != null) {
      for (String t : types) {
        if (SUPPORTED_TABLE_TYPES.contains(t)) {
          inputValidTableTypes.add(t);
        }
      }
    } else {
      inputValidTableTypes = new ArrayList<>(SUPPORTED_TABLE_TYPES);
    }
    return inputValidTableTypes;
  }
}
