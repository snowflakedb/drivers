package net.snowflake.client.internal.api.implementation.metadata;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.RowIdLifetime;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import net.snowflake.client.api.connection.SnowflakeDatabaseMetaData;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataCapabilities;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataIdentity;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataLimits;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataObjects;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataObjects.ForeignKeyKind;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.NotImplementedException;

public class SnowflakeDatabaseMetaDataImpl
    implements DatabaseMetaData, SnowflakeDatabaseMetaData, DelegatingWrapper {

  private final SnowflakeConnectionImpl connection;
  private final MetaDataIdentity identity;
  private final MetaDataCapabilities capabilities;
  private final MetaDataLimits limits;
  private final MetaDataObjects objects;

  public SnowflakeDatabaseMetaDataImpl(
      SnowflakeConnectionImpl connection, Properties properties, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.identity = new MetaDataIdentity(connection, properties);
    this.capabilities = new MetaDataCapabilities(connection);
    this.limits = new MetaDataLimits(connection, coreDriverApi);
    this.objects = new MetaDataObjects(connection, coreDriverApi);
  }

  @Override
  public boolean allProceduresAreCallable() throws SQLException {
    return capabilities.allProceduresAreCallable();
  }

  @Override
  public boolean allTablesAreSelectable() throws SQLException {
    return capabilities.allTablesAreSelectable();
  }

  @Override
  public String getURL() throws SQLException {
    return identity.getURL();
  }

  @Override
  public String getUserName() throws SQLException {
    return identity.getUserName();
  }

  @Override
  public boolean isReadOnly() throws SQLException {
    return capabilities.isReadOnly();
  }

  @Override
  public boolean nullsAreSortedHigh() throws SQLException {
    return capabilities.nullsAreSortedHigh();
  }

  @Override
  public boolean nullsAreSortedLow() throws SQLException {
    return capabilities.nullsAreSortedLow();
  }

  @Override
  public boolean nullsAreSortedAtStart() throws SQLException {
    return capabilities.nullsAreSortedAtStart();
  }

  @Override
  public boolean nullsAreSortedAtEnd() throws SQLException {
    return capabilities.nullsAreSortedAtEnd();
  }

  @Override
  public String getDatabaseProductName() throws SQLException {
    return identity.getDatabaseProductName();
  }

  @Override
  public String getDatabaseProductVersion() throws SQLException {
    return identity.getDatabaseProductVersion();
  }

  @Override
  public String getDriverName() throws SQLException {
    return identity.getDriverName();
  }

  @Override
  public String getDriverVersion() throws SQLException {
    return identity.getDriverVersion();
  }

  @Override
  public int getDriverMajorVersion() {
    return identity.getDriverMajorVersion();
  }

  @Override
  public int getDriverMinorVersion() {
    return identity.getDriverMinorVersion();
  }

  @Override
  public boolean usesLocalFiles() throws SQLException {
    return capabilities.usesLocalFiles();
  }

  @Override
  public boolean usesLocalFilePerTable() throws SQLException {
    return capabilities.usesLocalFilePerTable();
  }

  @Override
  public boolean supportsMixedCaseIdentifiers() throws SQLException {
    return capabilities.supportsMixedCaseIdentifiers();
  }

  @Override
  public boolean storesUpperCaseIdentifiers() throws SQLException {
    return capabilities.storesUpperCaseIdentifiers();
  }

  @Override
  public boolean storesLowerCaseIdentifiers() throws SQLException {
    return capabilities.storesLowerCaseIdentifiers();
  }

  @Override
  public boolean storesMixedCaseIdentifiers() throws SQLException {
    return capabilities.storesMixedCaseIdentifiers();
  }

  @Override
  public boolean supportsMixedCaseQuotedIdentifiers() throws SQLException {
    return capabilities.supportsMixedCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesUpperCaseQuotedIdentifiers() throws SQLException {
    return capabilities.storesUpperCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesLowerCaseQuotedIdentifiers() throws SQLException {
    return capabilities.storesLowerCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesMixedCaseQuotedIdentifiers() throws SQLException {
    return capabilities.storesMixedCaseQuotedIdentifiers();
  }

  @Override
  public String getIdentifierQuoteString() throws SQLException {
    return identity.getIdentifierQuoteString();
  }

  @Override
  public String getSQLKeywords() throws SQLException {
    return identity.getSQLKeywords();
  }

  @Override
  public String getNumericFunctions() throws SQLException {
    return identity.getNumericFunctions();
  }

  @Override
  public String getStringFunctions() throws SQLException {
    return identity.getStringFunctions();
  }

  @Override
  public String getSystemFunctions() throws SQLException {
    return identity.getSystemFunctions();
  }

  @Override
  public String getTimeDateFunctions() throws SQLException {
    return identity.getTimeDateFunctions();
  }

  @Override
  public String getSearchStringEscape() throws SQLException {
    return identity.getSearchStringEscape();
  }

  @Override
  public String getExtraNameCharacters() throws SQLException {
    return identity.getExtraNameCharacters();
  }

  @Override
  public boolean supportsAlterTableWithAddColumn() throws SQLException {
    return capabilities.supportsAlterTableWithAddColumn();
  }

  @Override
  public boolean supportsAlterTableWithDropColumn() throws SQLException {
    return capabilities.supportsAlterTableWithDropColumn();
  }

  @Override
  public boolean supportsColumnAliasing() throws SQLException {
    return capabilities.supportsColumnAliasing();
  }

  @Override
  public boolean nullPlusNonNullIsNull() throws SQLException {
    return capabilities.nullPlusNonNullIsNull();
  }

  @Override
  public boolean supportsConvert() throws SQLException {
    return capabilities.supportsConvert();
  }

  @Override
  public boolean supportsConvert(int fromType, int toType) throws SQLException {
    return capabilities.supportsConvert(fromType, toType);
  }

  @Override
  public boolean supportsTableCorrelationNames() throws SQLException {
    return capabilities.supportsTableCorrelationNames();
  }

  @Override
  public boolean supportsDifferentTableCorrelationNames() throws SQLException {
    return capabilities.supportsDifferentTableCorrelationNames();
  }

  @Override
  public boolean supportsExpressionsInOrderBy() throws SQLException {
    return capabilities.supportsExpressionsInOrderBy();
  }

  @Override
  public boolean supportsOrderByUnrelated() throws SQLException {
    return capabilities.supportsOrderByUnrelated();
  }

  @Override
  public boolean supportsGroupBy() throws SQLException {
    return capabilities.supportsGroupBy();
  }

  @Override
  public boolean supportsGroupByUnrelated() throws SQLException {
    return capabilities.supportsGroupByUnrelated();
  }

  @Override
  public boolean supportsGroupByBeyondSelect() throws SQLException {
    return capabilities.supportsGroupByBeyondSelect();
  }

  @Override
  public boolean supportsLikeEscapeClause() throws SQLException {
    return capabilities.supportsLikeEscapeClause();
  }

  @Override
  public boolean supportsMultipleResultSets() throws SQLException {
    return capabilities.supportsMultipleResultSets();
  }

  @Override
  public boolean supportsMultipleTransactions() throws SQLException {
    return capabilities.supportsMultipleTransactions();
  }

  @Override
  public boolean supportsNonNullableColumns() throws SQLException {
    return capabilities.supportsNonNullableColumns();
  }

  @Override
  public boolean supportsMinimumSQLGrammar() throws SQLException {
    return capabilities.supportsMinimumSQLGrammar();
  }

  @Override
  public boolean supportsCoreSQLGrammar() throws SQLException {
    return capabilities.supportsCoreSQLGrammar();
  }

  @Override
  public boolean supportsExtendedSQLGrammar() throws SQLException {
    return capabilities.supportsExtendedSQLGrammar();
  }

  @Override
  public boolean supportsANSI92EntryLevelSQL() throws SQLException {
    return capabilities.supportsANSI92EntryLevelSQL();
  }

  @Override
  public boolean supportsANSI92IntermediateSQL() throws SQLException {
    return capabilities.supportsANSI92IntermediateSQL();
  }

  @Override
  public boolean supportsANSI92FullSQL() throws SQLException {
    return capabilities.supportsANSI92FullSQL();
  }

  @Override
  public boolean supportsIntegrityEnhancementFacility() throws SQLException {
    return capabilities.supportsIntegrityEnhancementFacility();
  }

  @Override
  public boolean supportsOuterJoins() throws SQLException {
    return capabilities.supportsOuterJoins();
  }

  @Override
  public boolean supportsFullOuterJoins() throws SQLException {
    return capabilities.supportsFullOuterJoins();
  }

  @Override
  public boolean supportsLimitedOuterJoins() throws SQLException {
    return capabilities.supportsLimitedOuterJoins();
  }

  @Override
  public String getSchemaTerm() throws SQLException {
    return identity.getSchemaTerm();
  }

  @Override
  public String getProcedureTerm() throws SQLException {
    return identity.getProcedureTerm();
  }

  @Override
  public String getCatalogTerm() throws SQLException {
    return identity.getCatalogTerm();
  }

  @Override
  public boolean isCatalogAtStart() throws SQLException {
    return capabilities.isCatalogAtStart();
  }

  @Override
  public String getCatalogSeparator() throws SQLException {
    return identity.getCatalogSeparator();
  }

  @Override
  public boolean supportsSchemasInDataManipulation() throws SQLException {
    return capabilities.supportsSchemasInDataManipulation();
  }

  @Override
  public boolean supportsSchemasInProcedureCalls() throws SQLException {
    return capabilities.supportsSchemasInProcedureCalls();
  }

  @Override
  public boolean supportsSchemasInTableDefinitions() throws SQLException {
    return capabilities.supportsSchemasInTableDefinitions();
  }

  @Override
  public boolean supportsSchemasInIndexDefinitions() throws SQLException {
    return capabilities.supportsSchemasInIndexDefinitions();
  }

  @Override
  public boolean supportsSchemasInPrivilegeDefinitions() throws SQLException {
    return capabilities.supportsSchemasInPrivilegeDefinitions();
  }

  @Override
  public boolean supportsCatalogsInDataManipulation() throws SQLException {
    return capabilities.supportsCatalogsInDataManipulation();
  }

  @Override
  public boolean supportsCatalogsInProcedureCalls() throws SQLException {
    return capabilities.supportsCatalogsInProcedureCalls();
  }

  @Override
  public boolean supportsCatalogsInTableDefinitions() throws SQLException {
    return capabilities.supportsCatalogsInTableDefinitions();
  }

  @Override
  public boolean supportsCatalogsInIndexDefinitions() throws SQLException {
    return capabilities.supportsCatalogsInIndexDefinitions();
  }

  @Override
  public boolean supportsCatalogsInPrivilegeDefinitions() throws SQLException {
    return capabilities.supportsCatalogsInPrivilegeDefinitions();
  }

  @Override
  public boolean supportsPositionedDelete() throws SQLException {
    return capabilities.supportsPositionedDelete();
  }

  @Override
  public boolean supportsPositionedUpdate() throws SQLException {
    return capabilities.supportsPositionedUpdate();
  }

  @Override
  public boolean supportsSelectForUpdate() throws SQLException {
    return capabilities.supportsSelectForUpdate();
  }

  @Override
  public boolean supportsStoredProcedures() throws SQLException {
    return capabilities.supportsStoredProcedures();
  }

  @Override
  public boolean supportsSubqueriesInComparisons() throws SQLException {
    return capabilities.supportsSubqueriesInComparisons();
  }

  @Override
  public boolean supportsSubqueriesInExists() throws SQLException {
    return capabilities.supportsSubqueriesInExists();
  }

  @Override
  public boolean supportsSubqueriesInIns() throws SQLException {
    return capabilities.supportsSubqueriesInIns();
  }

  @Override
  public boolean supportsSubqueriesInQuantifieds() throws SQLException {
    return capabilities.supportsSubqueriesInQuantifieds();
  }

  @Override
  public boolean supportsCorrelatedSubqueries() throws SQLException {
    return capabilities.supportsCorrelatedSubqueries();
  }

  @Override
  public boolean supportsUnion() throws SQLException {
    return capabilities.supportsUnion();
  }

  @Override
  public boolean supportsUnionAll() throws SQLException {
    return capabilities.supportsUnionAll();
  }

  @Override
  public boolean supportsOpenCursorsAcrossCommit() throws SQLException {
    return capabilities.supportsOpenCursorsAcrossCommit();
  }

  @Override
  public boolean supportsOpenCursorsAcrossRollback() throws SQLException {
    return capabilities.supportsOpenCursorsAcrossRollback();
  }

  @Override
  public boolean supportsOpenStatementsAcrossCommit() throws SQLException {
    return capabilities.supportsOpenStatementsAcrossCommit();
  }

  @Override
  public boolean supportsOpenStatementsAcrossRollback() throws SQLException {
    return capabilities.supportsOpenStatementsAcrossRollback();
  }

  @Override
  public int getMaxBinaryLiteralLength() throws SQLException {
    return limits.getMaxBinaryLiteralLength();
  }

  @Override
  public int getMaxCharLiteralLength() throws SQLException {
    return limits.getMaxCharLiteralLength();
  }

  @Override
  public int getMaxColumnNameLength() throws SQLException {
    return limits.getMaxColumnNameLength();
  }

  @Override
  public int getMaxColumnsInGroupBy() throws SQLException {
    return limits.getMaxColumnsInGroupBy();
  }

  @Override
  public int getMaxColumnsInIndex() throws SQLException {
    return limits.getMaxColumnsInIndex();
  }

  @Override
  public int getMaxColumnsInOrderBy() throws SQLException {
    return limits.getMaxColumnsInOrderBy();
  }

  @Override
  public int getMaxColumnsInSelect() throws SQLException {
    return limits.getMaxColumnsInSelect();
  }

  @Override
  public int getMaxColumnsInTable() throws SQLException {
    return limits.getMaxColumnsInTable();
  }

  @Override
  public int getMaxConnections() throws SQLException {
    return limits.getMaxConnections();
  }

  @Override
  public int getMaxCursorNameLength() throws SQLException {
    return limits.getMaxCursorNameLength();
  }

  @Override
  public int getMaxIndexLength() throws SQLException {
    return limits.getMaxIndexLength();
  }

  @Override
  public int getMaxSchemaNameLength() throws SQLException {
    return limits.getMaxSchemaNameLength();
  }

  @Override
  public int getMaxProcedureNameLength() throws SQLException {
    return limits.getMaxProcedureNameLength();
  }

  @Override
  public int getMaxCatalogNameLength() throws SQLException {
    return limits.getMaxCatalogNameLength();
  }

  @Override
  public int getMaxRowSize() throws SQLException {
    return limits.getMaxRowSize();
  }

  @Override
  public boolean doesMaxRowSizeIncludeBlobs() throws SQLException {
    return limits.doesMaxRowSizeIncludeBlobs();
  }

  @Override
  public int getMaxStatementLength() throws SQLException {
    return limits.getMaxStatementLength();
  }

  @Override
  public int getMaxStatements() throws SQLException {
    return limits.getMaxStatements();
  }

  @Override
  public int getMaxTableNameLength() throws SQLException {
    return limits.getMaxTableNameLength();
  }

  @Override
  public int getMaxTablesInSelect() throws SQLException {
    return limits.getMaxTablesInSelect();
  }

  @Override
  public int getMaxUserNameLength() throws SQLException {
    return limits.getMaxUserNameLength();
  }

  @Override
  public int getDefaultTransactionIsolation() throws SQLException {
    return capabilities.getDefaultTransactionIsolation();
  }

  @Override
  public boolean supportsTransactions() throws SQLException {
    return capabilities.supportsTransactions();
  }

  @Override
  public boolean supportsTransactionIsolationLevel(int level) throws SQLException {
    return capabilities.supportsTransactionIsolationLevel(level);
  }

  @Override
  public boolean supportsDataDefinitionAndDataManipulationTransactions() throws SQLException {
    return capabilities.supportsDataDefinitionAndDataManipulationTransactions();
  }

  @Override
  public boolean supportsDataManipulationTransactionsOnly() throws SQLException {
    return capabilities.supportsDataManipulationTransactionsOnly();
  }

  @Override
  public boolean dataDefinitionCausesTransactionCommit() throws SQLException {
    return capabilities.dataDefinitionCausesTransactionCommit();
  }

  @Override
  public boolean dataDefinitionIgnoredInTransactions() throws SQLException {
    return capabilities.dataDefinitionIgnoredInTransactions();
  }

  @Override
  public ResultSet getProcedures(String catalog, String schemaPattern, String procedureNamePattern)
      throws SQLException {
    connection.checkClosed();
    return objects.getProcedures(catalog, schemaPattern, procedureNamePattern);
  }

  @Override
  public ResultSet getProcedureColumns(
      String catalog, String schemaPattern, String procedureNamePattern, String columnNamePattern)
      throws SQLException {
    connection.checkClosed();
    return objects.getProcedureColumns(
        catalog, schemaPattern, procedureNamePattern, columnNamePattern);
  }

  @Override
  public ResultSet getTables(
      String catalog, String schemaPattern, String tableNamePattern, String[] types)
      throws SQLException {
    connection.checkClosed();
    return objects.getTables(catalog, schemaPattern, tableNamePattern, types);
  }

  @Override
  public ResultSet getSchemas() throws SQLException {
    connection.checkClosed();
    return getSchemas(null, null);
  }

  @Override
  public ResultSet getCatalogs() throws SQLException {
    connection.checkClosed();
    return objects.getCatalogs();
  }

  @Override
  public ResultSet getTableTypes() throws SQLException {
    connection.checkClosed();
    return objects.getTableTypes();
  }

  @Override
  public ResultSet getColumns(
      String catalog, String schemaPattern, String tableNamePattern, String columnNamePattern)
      throws SQLException {
    connection.checkClosed();
    return objects.getColumns(catalog, schemaPattern, tableNamePattern, columnNamePattern, false);
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
    return objects.getTablePrivileges(catalog, schemaPattern, tableNamePattern);
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
    connection.checkClosed();
    return objects.getPrimaryKeys(catalog, schema, table);
  }

  @Override
  public ResultSet getImportedKeys(String catalog, String schema, String table)
      throws SQLException {
    connection.checkClosed();
    return objects.getForeignKeys(
        ForeignKeyKind.IMPORTED, catalog, schema, table, null, null, null);
  }

  @Override
  public ResultSet getExportedKeys(String catalog, String schema, String table)
      throws SQLException {
    connection.checkClosed();
    return objects.getForeignKeys(
        ForeignKeyKind.EXPORTED, catalog, schema, table, null, null, null);
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
    connection.checkClosed();
    return objects.getForeignKeys(
        ForeignKeyKind.CROSS_REFERENCE,
        parentCatalog,
        parentSchema,
        parentTable,
        foreignCatalog,
        foreignSchema,
        foreignTable);
  }

  @Override
  public ResultSet getTypeInfo() throws SQLException {
    connection.checkClosed();
    return objects.getTypeInfo();
  }

  @Override
  public ResultSet getIndexInfo(
      String catalog, String schema, String table, boolean unique, boolean approximate)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public boolean supportsResultSetType(int type) throws SQLException {
    return capabilities.supportsResultSetType(type);
  }

  @Override
  public boolean supportsResultSetConcurrency(int type, int concurrency) throws SQLException {
    return capabilities.supportsResultSetConcurrency(type, concurrency);
  }

  @Override
  public boolean ownUpdatesAreVisible(int type) throws SQLException {
    return capabilities.ownUpdatesAreVisible(type);
  }

  @Override
  public boolean ownDeletesAreVisible(int type) throws SQLException {
    return capabilities.ownDeletesAreVisible(type);
  }

  @Override
  public boolean ownInsertsAreVisible(int type) throws SQLException {
    return capabilities.ownInsertsAreVisible(type);
  }

  @Override
  public boolean othersUpdatesAreVisible(int type) throws SQLException {
    return capabilities.othersUpdatesAreVisible(type);
  }

  @Override
  public boolean othersDeletesAreVisible(int type) throws SQLException {
    return capabilities.othersDeletesAreVisible(type);
  }

  @Override
  public boolean othersInsertsAreVisible(int type) throws SQLException {
    return capabilities.othersInsertsAreVisible(type);
  }

  @Override
  public boolean updatesAreDetected(int type) throws SQLException {
    return capabilities.updatesAreDetected(type);
  }

  @Override
  public boolean deletesAreDetected(int type) throws SQLException {
    return capabilities.deletesAreDetected(type);
  }

  @Override
  public boolean insertsAreDetected(int type) throws SQLException {
    return capabilities.insertsAreDetected(type);
  }

  @Override
  public boolean supportsBatchUpdates() throws SQLException {
    return capabilities.supportsBatchUpdates();
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
    return capabilities.supportsSavepoints();
  }

  @Override
  public boolean supportsNamedParameters() throws SQLException {
    return capabilities.supportsNamedParameters();
  }

  @Override
  public boolean supportsMultipleOpenResults() throws SQLException {
    return capabilities.supportsMultipleOpenResults();
  }

  @Override
  public boolean supportsGetGeneratedKeys() throws SQLException {
    return capabilities.supportsGetGeneratedKeys();
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
    return capabilities.supportsResultSetHoldability(holdability);
  }

  @Override
  public int getResultSetHoldability() throws SQLException {
    return capabilities.getResultSetHoldability();
  }

  @Override
  public int getDatabaseMajorVersion() throws SQLException {
    return identity.getDatabaseMajorVersion();
  }

  @Override
  public int getDatabaseMinorVersion() throws SQLException {
    return identity.getDatabaseMinorVersion();
  }

  @Override
  public int getJDBCMajorVersion() throws SQLException {
    return identity.getJDBCMajorVersion();
  }

  @Override
  public int getJDBCMinorVersion() throws SQLException {
    return identity.getJDBCMinorVersion();
  }

  @Override
  public int getSQLStateType() throws SQLException {
    return capabilities.getSQLStateType();
  }

  @Override
  public boolean locatorsUpdateCopy() throws SQLException {
    return capabilities.locatorsUpdateCopy();
  }

  @Override
  public boolean supportsStatementPooling() throws SQLException {
    return capabilities.supportsStatementPooling();
  }

  @Override
  public RowIdLifetime getRowIdLifetime() throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowIdLifetime not supported");
  }

  @Override
  public ResultSet getSchemas(String catalog, String schemaPattern) throws SQLException {
    connection.checkClosed();
    return objects.getSchemas(catalog, schemaPattern);
  }

  @Override
  public boolean supportsStoredFunctionsUsingCallSyntax() throws SQLException {
    return capabilities.supportsStoredFunctionsUsingCallSyntax();
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
    return objects.getFunctions(catalog, schemaPattern, functionNamePattern);
  }

  @Override
  public ResultSet getFunctionColumns(
      String catalog, String schemaPattern, String functionNamePattern, String columnNamePattern)
      throws SQLException {
    connection.checkClosed();
    return objects.getFunctionColumns(
        catalog, schemaPattern, functionNamePattern, columnNamePattern);
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
    connection.checkClosed();
    return objects.getColumns(
        catalog, schemaPattern, tableNamePattern, columnNamePattern, extendedSet);
  }
}
