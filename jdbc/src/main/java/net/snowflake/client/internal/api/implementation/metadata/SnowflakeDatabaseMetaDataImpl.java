package net.snowflake.client.internal.api.implementation.metadata;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.RowIdLifetime;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.client.api.connection.SnowflakeDatabaseMetaData;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Snowflake JDBC DatabaseMetaData implementation
 *
 * <p>This is a stub implementation that provides basic database metadata information.
 */
public class SnowflakeDatabaseMetaDataImpl implements DatabaseMetaData, SnowflakeDatabaseMetaData {
  private static final String DatabaseProductName = "Snowflake";
  private static final String DriverName = "Snowflake";
  private static final char SEARCH_STRING_ESCAPE = '\\';
  private static final String JDBCVersion = "4.2";

  private final SnowflakeConnectionImpl connection;

  public SnowflakeDatabaseMetaDataImpl(SnowflakeConnectionImpl connection) {
    this.connection = connection;
  }

  @Override
  public boolean allProceduresAreCallable() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean allTablesAreSelectable() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public String getURL() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getUserName() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  /** Read only mode is not supported. */
  @Override
  public boolean isReadOnly() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean nullsAreSortedHigh() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean nullsAreSortedLow() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean nullsAreSortedAtStart() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean nullsAreSortedAtEnd() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public String getDatabaseProductName() throws SQLException {
    connection.checkClosed();
    return DatabaseProductName;
  }

  @Override
  public String getDatabaseProductVersion() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getDriverName() throws SQLException {
    connection.checkClosed();
    return DriverName;
  }

  @Override
  public String getDriverVersion() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public int getDriverMajorVersion() {
    throw new NotImplementedException();
  }

  @Override
  public int getDriverMinorVersion() {
    throw new NotImplementedException();
  }

  @Override
  public boolean usesLocalFiles() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean usesLocalFilePerTable() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsMixedCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean storesUpperCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean storesLowerCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean storesMixedCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsMixedCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean storesUpperCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean storesLowerCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean storesMixedCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public String getIdentifierQuoteString() throws SQLException {
    connection.checkClosed();
    return "\"";
  }

  @Override
  public String getSQLKeywords() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getNumericFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getStringFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getSystemFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getTimeDateFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public String getSearchStringEscape() throws SQLException {
    connection.checkClosed();
    return Character.toString(SEARCH_STRING_ESCAPE);
  }

  @Override
  public String getExtraNameCharacters() throws SQLException {
    connection.checkClosed();
    return "$";
  }

  @Override
  public boolean supportsAlterTableWithAddColumn() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsAlterTableWithDropColumn() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsColumnAliasing() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean nullPlusNonNullIsNull() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsConvert() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsConvert(int fromType, int toType) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsTableCorrelationNames() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsDifferentTableCorrelationNames() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsExpressionsInOrderBy() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsOrderByUnrelated() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsGroupBy() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsGroupByUnrelated() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsGroupByBeyondSelect() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsLikeEscapeClause() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsMultipleResultSets() throws SQLException {
    connection.checkClosed();
    // TODO: should it be true since we support multi statements?
    return false;
  }

  @Override
  public boolean supportsMultipleTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsNonNullableColumns() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsMinimumSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsCoreSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsExtendedSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsANSI92EntryLevelSQL() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsANSI92IntermediateSQL() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsANSI92FullSQL() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsIntegrityEnhancementFacility() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsFullOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsLimitedOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public String getSchemaTerm() throws SQLException {
    connection.checkClosed();
    return "schema";
  }

  @Override
  public String getProcedureTerm() throws SQLException {
    connection.checkClosed();
    return "procedure";
  }

  @Override
  public String getCatalogTerm() throws SQLException {
    connection.checkClosed();
    return "database";
  }

  @Override
  public boolean isCatalogAtStart() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public String getCatalogSeparator() throws SQLException {
    connection.checkClosed();
    return ".";
  }

  @Override
  public boolean supportsSchemasInDataManipulation() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSchemasInProcedureCalls() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsSchemasInTableDefinitions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSchemasInIndexDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsSchemasInPrivilegeDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsCatalogsInDataManipulation() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsCatalogsInProcedureCalls() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsCatalogsInTableDefinitions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsCatalogsInIndexDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsCatalogsInPrivilegeDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsPositionedDelete() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsPositionedUpdate() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsSelectForUpdate() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsStoredProcedures() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSubqueriesInComparisons() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSubqueriesInExists() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSubqueriesInIns() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsSubqueriesInQuantifieds() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsCorrelatedSubqueries() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsUnion() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsUnionAll() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsOpenCursorsAcrossCommit() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsOpenCursorsAcrossRollback() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsOpenStatementsAcrossCommit() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsOpenStatementsAcrossRollback() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public int getMaxBinaryLiteralLength() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public int getMaxCharLiteralLength() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public int getMaxColumnNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  @Override
  public int getMaxColumnsInGroupBy() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxColumnsInIndex() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxColumnsInOrderBy() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxColumnsInSelect() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxColumnsInTable() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxConnections() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxCursorNameLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxIndexLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxSchemaNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  @Override
  public int getMaxProcedureNameLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxCatalogNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  @Override
  public int getMaxRowSize() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public boolean doesMaxRowSizeIncludeBlobs() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public int getMaxStatementLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxStatements() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxTableNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  @Override
  public int getMaxTablesInSelect() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  @Override
  public int getMaxUserNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  @Override
  public int getDefaultTransactionIsolation() throws SQLException {
    connection.checkClosed();
    return Connection.TRANSACTION_READ_COMMITTED;
  }

  @Override
  public boolean supportsTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsTransactionIsolationLevel(int level) throws SQLException {
    connection.checkClosed();
    return (level == Connection.TRANSACTION_NONE)
        || (level == Connection.TRANSACTION_READ_COMMITTED);
  }

  @Override
  public boolean supportsDataDefinitionAndDataManipulationTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean supportsDataManipulationTransactionsOnly() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean dataDefinitionCausesTransactionCommit() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean dataDefinitionIgnoredInTransactions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  // Stub implementations for remaining methods
  @Override
  public ResultSet getProcedures(String catalog, String schemaPattern, String procedureNamePattern)
      throws SQLException {
    connection.checkClosed();
    throw new SQLFeatureNotSupportedException("getProcedures not supported");
  }

  @Override
  public ResultSet getProcedureColumns(
      String catalog, String schemaPattern, String procedureNamePattern, String columnNamePattern)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getTables(
      String catalog, String schemaPattern, String tableNamePattern, String[] types)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getSchemas() throws SQLException {
    connection.checkClosed();
    return getSchemas(null, null);
  }

  @Override
  public ResultSet getCatalogs() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getTableTypes() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getColumns(
      String catalog, String schemaPattern, String tableNamePattern, String columnNamePattern)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getColumnPrivileges(
      String catalog, String schema, String table, String columnNamePattern) throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getTablePrivileges(String catalog, String schemaPattern, String tableNamePattern)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getBestRowIdentifier(
      String catalog, String schema, String table, int scope, boolean nullable)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getBestRowIdentifier not supported");
  }

  @Override
  public ResultSet getVersionColumns(String catalog, String schema, String table)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getVersionColumns not supported");
  }

  @Override
  public ResultSet getPrimaryKeys(String catalog, String schema, String table) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getImportedKeys(String catalog, String schema, String table)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getExportedKeys(String catalog, String schema, String table)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getCrossReference(
      String parentCatalog,
      String parentSchema,
      String parentTable,
      String foreignCatalog,
      String foreignSchema,
      String foreignTable)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getTypeInfo() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getIndexInfo(
      String catalog, String schema, String table, boolean unique, boolean approximate)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public boolean supportsResultSetType(int type) throws SQLException {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY);
  }

  @Override
  public boolean supportsResultSetConcurrency(int type, int concurrency) throws SQLException {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY && concurrency == ResultSet.CONCUR_READ_ONLY);
  }

  @Override
  public boolean ownUpdatesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean ownDeletesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean ownInsertsAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean othersUpdatesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean othersDeletesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean othersInsertsAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean updatesAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean deletesAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean insertsAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsBatchUpdates() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public ResultSet getUDTs(
      String catalog, String schemaPattern, String typeNamePattern, int[] types)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public Connection getConnection() throws SQLException {
    connection.checkClosed();
    return connection;
  }

  // Additional JDBC 3.0+ methods (stubs)
  @Override
  public boolean supportsSavepoints() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsNamedParameters() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsMultipleOpenResults() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public boolean supportsGetGeneratedKeys() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public ResultSet getSuperTypes(String catalog, String schemaPattern, String typeNamePattern)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getSuperTypes not supported");
  }

  @Override
  public ResultSet getSuperTables(String catalog, String schemaPattern, String tableNamePattern)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getSuperTables not supported");
  }

  @Override
  public ResultSet getAttributes(
      String catalog, String schemaPattern, String typeNamePattern, String attributeNamePattern)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getAttributes not supported");
  }

  @Override
  public boolean supportsResultSetHoldability(int holdability) throws SQLException {
    connection.checkClosed();
    return holdability == ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public int getResultSetHoldability() throws SQLException {
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public int getDatabaseMajorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMajorVersion();
  }

  @Override
  public int getDatabaseMinorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMinorVersion();
  }

  @Override
  public int getJDBCMajorVersion() throws SQLException {
    connection.checkClosed();
    return Integer.parseInt(JDBCVersion.split("\\.", 2)[0]);
  }

  @Override
  public int getJDBCMinorVersion() throws SQLException {
    connection.checkClosed();
    return Integer.parseInt(JDBCVersion.split("\\.", 2)[1]);
  }

  @Override
  public int getSQLStateType() throws SQLException {
    return sqlStateSQL;
  }

  @Override
  public boolean locatorsUpdateCopy() throws SQLException {
    return false;
  }

  @Override
  public boolean supportsStatementPooling() throws SQLException {
    connection.checkClosed();
    return false;
  }

  @Override
  public RowIdLifetime getRowIdLifetime() throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowIdLifetime not supported");
  }

  @Override
  public ResultSet getSchemas(String catalog, String schemaPattern) throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public boolean supportsStoredFunctionsUsingCallSyntax() throws SQLException {
    connection.checkClosed();
    return true;
  }

  @Override
  public boolean autoCommitFailureClosesAllResultSets() throws SQLException {
    throw new SQLFeatureNotSupportedException("autoCommitFailureClosesAllResultSets not supported");
  }

  @Override
  public ResultSet getClientInfoProperties() throws SQLException {
    throw new SQLFeatureNotSupportedException("getClientInfoProperties not supported");
  }

  @Override
  public ResultSet getFunctions(String catalog, String schemaPattern, String functionNamePattern)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getFunctionColumns(
      String catalog, String schemaPattern, String functionNamePattern, String columnNamePattern)
      throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getPseudoColumns(
      String catalog, String schemaPattern, String tableNamePattern, String columnNamePattern)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("getPseudoColumns not supported");
  }

  @Override
  public boolean generatedKeyAlwaysReturned() throws SQLException {
    throw new SQLFeatureNotSupportedException("generatedKeyAlwaysReturned not supported");
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    if (iface.isAssignableFrom(getClass())) {
      return iface.cast(this);
    }
    throw new SQLException("Cannot unwrap to " + iface.getName());
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    throw new SQLFeatureNotSupportedException("isWrapperFor not supported");
  }

  @Override
  public ResultSet getStreams(String catalog, String schemaPattern, String streamName)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet getColumns(
      String catalog,
      String schemaPattern,
      String tableNamePattern,
      String columnNamePattern,
      boolean extendedSet)
      throws SQLException {
    throw new NotImplementedException();
  }
}
