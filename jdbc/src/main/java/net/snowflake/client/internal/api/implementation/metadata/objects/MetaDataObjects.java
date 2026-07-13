package net.snowflake.client.internal.api.implementation.metadata.objects;

import static java.sql.ResultSetMetaData.columnNoNulls;
import static java.sql.ResultSetMetaData.columnNullable;
import static net.snowflake.client.internal.api.implementation.metadata.objects.ErrorUtils.isMissingMetadataObject;
import static net.snowflake.client.internal.api.implementation.metadata.objects.MatchingUtils.matches;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.regex.Pattern;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataLimits;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.resultset.RowConverter;
import net.snowflake.client.internal.api.implementation.resultset.SnowflakeResultSetImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeColumnMetadata;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.common.util.Wildcard;

/**
 * Owns the query-backed {@link java.sql.DatabaseMetaData} methods: building the {@code SHOW}
 * command, running it, filtering/projecting rows, and fabricating the JDBC-shaped result set. Keeps
 * {@link SnowflakeDatabaseMetaDataImpl} a thin delegating shell.
 */
public class MetaDataObjects {

  // TODO(SNOW-3740738): maybe we should use rpc GetConnectionObjects instead of querying
  //  Then we can move escaping, etc. to the core and avoid those operations in wrapper.

  // TODO(SNOW-3740739): using column labels is cleaner than positional arguments, consider changing

  private static final SFLogger logger = SFLoggerFactory.getLogger(MetaDataObjects.class);

  private static final ObjectMapper mapper = new ObjectMapper();

  private static final String TABLE_TYPE_TABLE = "TABLE";
  private static final String TABLE_TYPE_VIEW = "VIEW";
  private static final List<String> SUPPORTED_TABLE_TYPES =
      Arrays.asList(TABLE_TYPE_TABLE, TABLE_TYPE_VIEW);

  private final InternalSnowflakeConnection connection;
  private final MetaDataLimits limits;

  public MetaDataObjects(InternalSnowflakeConnection connection) {
    this.connection = connection;
    this.limits = new MetaDataLimits(connection);
  }

  public ResultSet getCatalogs() throws SQLException {
    String sqlQuery = queryBuilder().show("databases").inAccount().build();
    RowConverter rowConverter = row -> new Object[] {row.getString("name")};
    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_CATALOGS);
  }

  public ResultSet getSchemas(String originalCatalog, String originalSchemaPattern)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware).show("schemas").likeSchema(schemaPattern).in(catalog).build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_SCHEMAS);
    }

    logger.debug("SQL query in getSchemas: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    RowConverter rowConverter =
        row -> {
          String schemaName = row.getString(2);
          String dbName = row.getString(5);
          if (contextAware.schemaMatches(compiledSchemaPattern, schemaName)) {
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
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    boolean viewOnly = tableTypes.size() == 1 && "VIEW".equalsIgnoreCase(tableTypes.get(0));
    boolean tableOnly = tableTypes.size() == 1 && "TABLE".equalsIgnoreCase(tableTypes.get(0));
    String showType;
    if (viewOnly) {
      showType = "views";
    } else if (tableOnly) {
      showType = "tables";
    } else {
      showType = "objects";
    }

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

          // TODO(SNOW-3740734): why don't we have exact schema matching case here?
          if (matches(compiledTablePattern, tableName)
              && matches(compiledSchemaPattern, schemaName)) {
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
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
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

    ParametersRegistry params = connection.getParameters();
    boolean jdbcTreatDecimalAsInt = params.getBool(Parameter.JDBC_TREAT_DECIMAL_AS_INT);
    boolean enableReturnTimestampWithTimeZone =
        params.getBool(Parameter.ENABLE_RETURN_TIMESTAMP_WITH_TIMEZONE);
    boolean stringsQuoted = params.getBool(Parameter.STRINGS_QUOTED_FOR_COLUMN_DEF);

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
          // in the legacy driver trim() result was discarded and null default value caused NPE,
          // anyway - no difference in practice for data returned by backend
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

          // TODO(SNOW-3740734): why don't we have exact schema matching case here?
          if (matches(compiledTablePattern, tableName)
              && matches(compiledSchemaPattern, schemaName)
              && matches(compiledColumnPattern, columnName)) {

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

  public ResultSet getTableTypes() throws SQLException {
    Object[][] rows =
        SUPPORTED_TABLE_TYPES.stream().map(t -> new Object[] {t}).toArray(Object[][]::new);
    return createResultSet(rows, MetaDataResultSetFormat.GET_TABLE_TYPES);
  }

  public ResultSet getTypeInfo() throws SQLException {
    return createResultSet(TYPE_INFO, MetaDataResultSetFormat.GET_TYPE_INFO);
  }

  public ResultSet getProcedures(
      String originalCatalog, String originalSchemaPattern, String procedureNamePattern)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show("procedures")
            .like(procedureNamePattern)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_PROCEDURES);
    }

    logger.debug("SQL query in getProcedures: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledProcedurePattern = Wildcard.toRegexPattern(procedureNamePattern, true);
    RowConverter rowConverter =
        row -> {
          String catalogName = row.getString("catalog_name");
          String schemaName = row.getString("schema_name");
          String procedureName = row.getString("name");
          String remarks = row.getString("description");
          String specificName = row.getString("arguments");
          if (matches(compiledProcedurePattern, procedureName)
              && contextAware.schemaMatches(compiledSchemaPattern, schemaName)) {

            return new Object[] {
              catalogName,
              schemaName,
              procedureName,
              remarks,
              DatabaseMetaData.procedureReturnsResult,
              specificName
            };
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_PROCEDURES);
  }

  public ResultSet getFunctions(
      String originalCatalog, String originalSchemaPattern, String functionNamePattern)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show("functions")
            .like(functionNamePattern)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_FUNCTIONS);
    }

    logger.debug("SQL query in getFunctions: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledFunctionPattern = Wildcard.toRegexPattern(functionNamePattern, true);

    RowConverter rowConverter =
        row -> {
          String catalogName = row.getString(11);
          String schemaName = row.getString(3);
          String functionName = row.getString(2);
          String remarks = row.getString(10);
          int functionType =
              ("Y".equals(row.getString(12))
                  ? DatabaseMetaData.functionReturnsTable
                  : DatabaseMetaData.functionNoTable);
          // TODO(SNOW-3740737): getProcedures has correct behavior of using getString("arguments")
          //  for "specificName", consider to fix it here as well
          String specificName = functionName;
          if (matches(compiledFunctionPattern, functionName)
              && contextAware.schemaMatches(compiledSchemaPattern, schemaName)) {

            return new Object[] {
              catalogName, schemaName, functionName, remarks, functionType, specificName
            };
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_FUNCTIONS);
  }

  public ResultSet getProcedureColumns(
      String originalCatalog,
      String originalSchemaPattern,
      String procedureNamePattern,
      String columnNamePattern)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show("procedures")
            .like(procedureNamePattern)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_PROCEDURE_COLUMNS);
    }

    logger.debug("SQL query in getProcedureColumns: {}", sqlQuery);

    try (Statement stmt = connection.createStatement()) {
      Object[][] rows =
          new ObjectsByDescribe(limits, stmt, sqlQuery)
              .showAndDescribeProcedures(
                  catalog, schemaPattern, procedureNamePattern, columnNamePattern);
      return createResultSet(rows, MetaDataResultSetFormat.GET_PROCEDURE_COLUMNS);
    }
  }

  public ResultSet getFunctionColumns(
      String originalCatalog,
      String originalSchemaPattern,
      String functionNamePattern,
      String columnNamePattern)
      throws SQLException {
    // BD#19: result rows used raw params instead of session-resolved values.
    // null catalog produced null FUNCTION_CAT even when session context had a real database
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show("functions")
            .like(functionNamePattern)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_FUNCTION_COLUMNS);
    }

    logger.debug("SQL query in getFunctionColumns: {}", sqlQuery);

    try (Statement stmt = connection.createStatement()) {
      Object[][] rows =
          new ObjectsByDescribe(limits, stmt, sqlQuery)
              .showAndDescribeFunctions(
                  catalog, schemaPattern, functionNamePattern, columnNamePattern);
      return createResultSet(rows, MetaDataResultSetFormat.GET_FUNCTION_COLUMNS);
    }
  }

  public ResultSet getTablePrivileges(
      String originalCatalog, String originalSchemaPattern, String tableNamePattern)
      throws SQLException {
    // TODO(SNOW-3740736): only this method null-guards tableNamePattern; others pass null to the
    //  query builder. Align in one direction or the other.
    if (tableNamePattern == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_TABLE_PRIVILEGES);
    }
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware).showTablePrivileges(catalog, schemaPattern, tableNamePattern);

    RowConverter rowConverter =
        row -> {
          String table_cat = row.getString("TABLE_CATALOG");
          String table_schema = row.getString("TABLE_SCHEMA");
          String table_name = row.getString("TABLE_NAME");
          String grantor = row.getString("GRANTOR");
          String grantee = row.getString("GRANTEE");
          String privilege = row.getString("PRIVILEGE_TYPE");
          String is_grantable = row.getString("IS_GRANTABLE");

          // TODO(SNOW-3740736): unlike other methods, this post-filters with string equality + "%"
          //  literal, not Wildcard.toRegexPattern() + matches(). Patterns like "MY_SCHEMA%" fail.
          if ((catalog == null || catalog.trim().equals("%") || catalog.trim().equals(table_cat))
              && (schemaPattern == null
                  || schemaPattern.trim().equals("%")
                  || schemaPattern.trim().equals(table_schema))
              && (tableNamePattern.trim().equals(table_name)
                  || tableNamePattern.trim().equals("%"))) {
            return new Object[] {
              table_cat, table_schema, table_name, grantor, grantee, privilege, is_grantable,
            };
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_TABLE_PRIVILEGES);
  }

  public ResultSet getPrimaryKeys(String originalCatalog, String originalSchema, String table)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchema);
    String catalog = contextAware.getDatabase();
    String schema = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware).show("primary keys").in(catalog, schema, table).build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_PRIMARY_KEYS);
    }

    logger.debug("SQL query in getPrimaryKeys: {}", sqlQuery);

    // TODO(SNOW-3740735): getPrimaryKeys and getForeignKeys gate pattern matching on
    //  enablePatternSearch, while all other methods use isExactSchema (via contextAware). These are
    //  different session parameters and produce different behavior for the same inputs.
    boolean patternSearch = connection.getParameters().getBool(Parameter.ENABLE_PATTERN_SEARCH);
    // Patterns are only consulted when enablePatternSearch=true; otherwise exact equality is used.
    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schema, true);
    Pattern compiledTablePattern = Wildcard.toRegexPattern(table, true);

    RowConverter rowConverter =
        row -> {
          String tableCat = row.getString(2);
          String tableSchem = row.getString(3);
          String tableName = row.getString(4);
          String columnName = row.getString(5);
          int keySeq = row.getInt(6);
          String pkName = row.getString(7);

          // Catalog is always matched by exact equality, mirroring the legacy driver.
          boolean catalogMatches = catalog == null || catalog.equals(tableCat);
          boolean schemaMatches;
          boolean tableMatches;
          if (patternSearch) {
            schemaMatches = matches(compiledSchemaPattern, tableSchem);
            tableMatches = matches(compiledTablePattern, tableName);
          } else {
            schemaMatches = schema == null || schema.equals(tableSchem);
            tableMatches = table == null || table.equals(tableName);
          }

          // Pattern.equals(String) guards were always false (dead code); removed.
          if (catalogMatches && schemaMatches && tableMatches) {
            return new Object[] {
              tableCat, tableSchem, tableName, columnName, keySeq, pkName,
            };
          }
          return null;
        };

    // TODO(SNOW-3695645): Rows are returned in SHOW PRIMARY KEYS order (key sequence).
    //  The JDBC spec says COLUMN_NAME order, but snowflake-jdbc also omits the sort.
    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_PRIMARY_KEYS);
  }

  @RequiredArgsConstructor
  public enum ForeignKeyKind {
    IMPORTED("imported keys"),
    // Exported and cross-reference both emit "exported keys" and post-filter on the primary table
    // (cross-reference additionally filters on the foreign table).
    EXPORTED("exported keys"),
    CROSS_REFERENCE("exported keys");

    private final String showType;
  }

  public ResultSet getForeignKeys(
      ForeignKeyKind kind,
      String originalParentCatalog,
      String originalParentSchema,
      String parentTable,
      String foreignCatalog,
      String foreignSchema,
      String foreignTable)
      throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(
            connection, originalParentCatalog, originalParentSchema);
    String parentCatalog = contextAware.getDatabase();
    String parentSchema = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show(kind.showType)
            .in(parentCatalog, parentSchema, parentTable)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_FOREIGN_KEYS);
    }

    logger.debug("SQL query in getForeignKeys: {}", sqlQuery);

    // TODO(SNOW-3740735): see getPrimaryKeys - same enablePatternSearch vs isExactSchema mismatch.
    boolean patternSearch = connection.getParameters().getBool(Parameter.ENABLE_PATTERN_SEARCH);
    // Patterns are only consulted when enablePatternSearch=true; otherwise exact equality is used.
    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(parentSchema, true);
    Pattern compiledParentTablePattern = Wildcard.toRegexPattern(parentTable, true);
    Pattern compiledForeignSchemaPattern = Wildcard.toRegexPattern(foreignSchema, true);
    Pattern compiledForeignTablePattern = Wildcard.toRegexPattern(foreignTable, true);

    RowConverter rowConverter =
        row -> {
          String pktableCat = row.getString(2);
          String pktableSchem = row.getString(3);
          String pktableName = row.getString(4);
          String pkcolumnName = row.getString(5);
          String fktableCat = row.getString(6);
          String fktableSchem = row.getString(7);
          String fktableName = row.getString(8);
          String fkcolumnName = row.getString(9);
          int keySeq = row.getInt(10);
          short updateRule = getForeignKeyConstraintProperty("update", row.getString(11));
          short deleteRule = getForeignKeyConstraintProperty("delete", row.getString(12));
          String fkName = row.getString(13);
          String pkName = row.getString(14);
          short deferrability = getForeignKeyConstraintProperty("deferrability", row.getString(15));

          boolean passedFilter;
          if (patternSearch) {
            passedFilter =
                foreignKeyPatternMatch(
                    kind,
                    parentCatalog,
                    compiledSchemaPattern,
                    compiledParentTablePattern,
                    foreignCatalog,
                    compiledForeignSchemaPattern,
                    compiledForeignTablePattern,
                    pktableCat,
                    pktableSchem,
                    pktableName,
                    fktableCat,
                    fktableSchem,
                    fktableName);
          } else {
            passedFilter =
                foreignKeyExactMatch(
                    kind,
                    parentCatalog,
                    parentSchema,
                    parentTable,
                    foreignCatalog,
                    foreignSchema,
                    foreignTable,
                    pktableCat,
                    pktableSchem,
                    pktableName,
                    fktableCat,
                    fktableSchem,
                    fktableName);
          }

          if (passedFilter) {
            return new Object[] {
              pktableCat,
              pktableSchem,
              pktableName,
              pkcolumnName,
              fktableCat,
              fktableSchem,
              fktableName,
              fkcolumnName,
              keySeq,
              updateRule,
              deleteRule,
              fkName,
              pkName,
              deferrability,
            };
          }
          return null;
        };

    // TODO(SNOW-3695645): Rows are returned in SHOW ... KEYS order (key sequence).
    //  The JDBC spec says order by FK table columns + KEY_SEQ, but snowflake-jdbc omits the sort.
    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_FOREIGN_KEYS);
  }

  private static boolean foreignKeyExactMatch(
      ForeignKeyKind kind,
      String parentCatalog,
      String parentSchema,
      String parentTable,
      String foreignCatalog,
      String foreignSchema,
      String foreignTable,
      String pktableCat,
      String pktableSchem,
      String pktableName,
      String fktableCat,
      String fktableSchem,
      String fktableName) {
    switch (kind) {
      case IMPORTED:
        // For imported keys, filter on the foreign key table.
        return (parentCatalog == null || parentCatalog.equals(fktableCat))
            && (parentSchema == null || parentSchema.equals(fktableSchem))
            && (parentTable == null || parentTable.equals(fktableName));
      case EXPORTED:
        // For exported keys, filter on the primary key table.
        return (parentCatalog == null || parentCatalog.equals(pktableCat))
            && (parentSchema == null || parentSchema.equals(pktableSchem))
            && (parentTable == null || parentTable.equals(pktableName));
      case CROSS_REFERENCE:
        // For cross references, filter on both the primary key and foreign key table.
        return (parentCatalog == null || parentCatalog.equals(pktableCat))
            && (parentSchema == null || parentSchema.equals(pktableSchem))
            && (parentTable == null || parentTable.equals(pktableName))
            && (foreignCatalog == null || foreignCatalog.equals(fktableCat))
            && (foreignSchema == null || foreignSchema.equals(fktableSchem))
            && (foreignTable == null || foreignTable.equals(fktableName));
      default:
        return false;
    }
  }

  // Pattern.equals(String) guards were always false (dead code) for all three FK kinds; removed.
  private static boolean foreignKeyPatternMatch(
      ForeignKeyKind kind,
      String parentCatalog,
      Pattern compiledSchemaPattern,
      Pattern compiledParentTablePattern,
      String foreignCatalog,
      Pattern compiledForeignSchemaPattern,
      Pattern compiledForeignTablePattern,
      String pktableCat,
      String pktableSchem,
      String pktableName,
      String fktableCat,
      String fktableSchem,
      String fktableName) {
    switch (kind) {
      case IMPORTED:
        // For imported keys, filter on the foreign key table.
        return (parentCatalog == null || parentCatalog.equals(fktableCat))
            && matches(compiledSchemaPattern, fktableSchem)
            && matches(compiledParentTablePattern, fktableName);
      case EXPORTED:
        // For exported keys, filter on the primary key table.
        return (parentCatalog == null || parentCatalog.equals(pktableCat))
            && matches(compiledSchemaPattern, pktableSchem)
            && matches(compiledParentTablePattern, pktableName);
      case CROSS_REFERENCE:
        // For cross references, filter on both the primary key and foreign key table.
        return (parentCatalog == null || parentCatalog.equals(pktableCat))
            && matches(compiledSchemaPattern, pktableSchem)
            && matches(compiledParentTablePattern, pktableName)
            && (foreignCatalog == null || foreignCatalog.equals(fktableCat))
            && matches(compiledForeignSchemaPattern, fktableSchem)
            && matches(compiledForeignTablePattern, fktableName);
      default:
        return false;
    }
  }

  /** Ported from snowflake-jdbc SnowflakeDatabaseMetaDataImpl. */
  private static short getForeignKeyConstraintProperty(String propertyName, String property) {
    if (property == null) {
      return 0;
    }
    switch (propertyName) {
      case "update":
      case "delete":
        switch (property) {
          case "NO ACTION":
            return DatabaseMetaData.importedKeyNoAction;
          case "CASCADE":
            return DatabaseMetaData.importedKeyCascade;
          case "SET NULL":
            return DatabaseMetaData.importedKeySetNull;
          case "SET DEFAULT":
            return DatabaseMetaData.importedKeySetDefault;
          case "RESTRICT":
            return DatabaseMetaData.importedKeyRestrict;
          default:
            return 0;
        }
      case "deferrability":
        switch (property) {
          case "INITIALLY DEFERRED":
            return DatabaseMetaData.importedKeyInitiallyDeferred;
          case "INITIALLY IMMEDIATE":
            return DatabaseMetaData.importedKeyInitiallyImmediate;
          case "NOT DEFERRABLE":
            return DatabaseMetaData.importedKeyNotDeferrable;
          default:
            return 0;
        }
      default:
        return 0;
    }
  }

  public ResultSet getStreams(
      String originalCatalog, String originalSchemaPattern, String streamName) throws SQLException {
    ContextAwareMetadataSearch contextAware =
        ContextAwareMetadataSearch.fromSession(connection, originalCatalog, originalSchemaPattern);
    String catalog = contextAware.getDatabase();
    String schemaPattern = contextAware.getSchema();

    String sqlQuery =
        queryBuilder(contextAware)
            .show("streams")
            .like(streamName)
            .in(catalog, schemaPattern)
            .build();

    if (sqlQuery == null) {
      return emptyResultSet(MetaDataResultSetFormat.GET_STREAMS);
    }

    logger.debug("SQL query in getStreams: {}", sqlQuery);

    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledStreamNamePattern = Wildcard.toRegexPattern(streamName, true);
    RowConverter rowConverter =
        row -> {
          String name = row.getString("name");
          String schemaName = row.getString("schema_name");
          // TODO(SNOW-3740734): why don't we have exact schema matching case here?
          if (matches(compiledStreamNamePattern, name)
              && matches(compiledSchemaPattern, schemaName)) {
            return new Object[] {
              name,
              row.getString("database_name"),
              schemaName,
              row.getString("owner"),
              row.getString("comment"),
              row.getString("table_name"),
              row.getString("source_type"),
              row.getString("base_tables"),
              row.getString("type"),
              row.getString("stale"),
              row.getString("mode")
            };
          }
          return null;
        };

    return createResultSet(sqlQuery, rowConverter, MetaDataResultSetFormat.GET_STREAMS);
  }

  public ResultSet getColumnPrivileges(
      String catalog, String schema, String table, String columnNamePattern) throws SQLException {
    return emptyResultSet(MetaDataResultSetFormat.GET_COLUMN_PRIVILEGES);
  }

  public ResultSet getIndexInfo(
      String catalog, String schema, String table, boolean unique, boolean approximate)
      throws SQLException {
    return emptyResultSet(MetaDataResultSetFormat.GET_INDEX_INFO);
  }

  public ResultSet getUDTs(
      String catalog, String schemaPattern, String typeNamePattern, int[] types)
      throws SQLException {
    return emptyResultSet(MetaDataResultSetFormat.GET_UDTS);
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

  private MetaDataQueryBuilder queryBuilder() {
    return new MetaDataQueryBuilder(
        false,
        false,
        connection.getParameters().getBool(Parameter.ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS));
  }

  private MetaDataQueryBuilder queryBuilder(ContextAwareMetadataSearch ctx) {
    return new MetaDataQueryBuilder(
        ctx.isExactSchema(), ctx.isUseSessionSchema(), ctx.isEnableWildcards());
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

  private ResultSet createResultSet(Object[][] rows, MetaDataResultSetFormat format)
      throws SQLException {
    SnowflakeStatementImpl statement =
        connection.createStatement().unwrap(SnowflakeStatementImpl.class);
    try {
      return ResultSetFactory.createFromRows(statement, format.metaData(null), rows, true);
    } catch (SQLException | RuntimeException e) {
      statement.close();
      throw e;
    }
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

  private static final Object[][] TYPE_INFO =
      new Object[][] {
        {
          "NUMBER",
          Types.DECIMAL,
          38,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          0,
          37,
          -1,
          -1,
          -1
        },
        {
          "INTEGER",
          Types.INTEGER,
          38,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          0,
          0,
          -1,
          -1,
          -1
        },
        {
          "DOUBLE",
          Types.DOUBLE,
          38,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          0,
          37,
          -1,
          -1,
          -1
        },
        {
          "VARCHAR",
          Types.VARCHAR,
          -1,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          -1,
          -1,
          -1,
          -1,
          -1
        },
        {
          "DATE",
          Types.DATE,
          -1,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          -1,
          -1,
          -1,
          -1,
          -1
        },
        {
          "TIME",
          Types.TIME,
          -1,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          -1,
          -1,
          -1,
          -1,
          -1
        },
        {
          "TIMESTAMP",
          Types.TIMESTAMP,
          -1,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          -1,
          -1,
          -1,
          -1,
          -1
        },
        {
          "BOOLEAN",
          Types.BOOLEAN,
          -1,
          null,
          null,
          null,
          DatabaseMetaData.typeNullable,
          false,
          DatabaseMetaData.typeSearchable,
          false,
          true,
          true,
          null,
          -1,
          -1,
          -1,
          -1,
          -1
        }
      };
}
