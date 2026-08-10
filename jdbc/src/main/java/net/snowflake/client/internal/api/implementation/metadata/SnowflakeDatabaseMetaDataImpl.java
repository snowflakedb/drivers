package net.snowflake.client.internal.api.implementation.metadata;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.RowIdLifetime;
import net.snowflake.client.api.connection.SnowflakeDatabaseMetaData;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataCapabilities;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataIdentity;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataLimits;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataObjects;
import net.snowflake.client.internal.api.implementation.metadata.objects.MetaDataObjects.ForeignKeyKind;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.util.DelegatingWrapper;

@JdbcBoundary
public class SnowflakeDatabaseMetaDataImpl
    implements DatabaseMetaData, SnowflakeDatabaseMetaData, DelegatingWrapper {

  private final InternalSnowflakeConnection connection;
  private final MetaDataIdentity identity;
  private final MetaDataCapabilities capabilities;
  private final MetaDataLimits limits;
  private final MetaDataObjects objects;

  public SnowflakeDatabaseMetaDataImpl(InternalSnowflakeConnection connection) {
    this.connection = connection;
    this.identity = new MetaDataIdentity(connection);
    this.capabilities = new MetaDataCapabilities(connection);
    this.limits = new MetaDataLimits(connection);
    this.objects = new MetaDataObjects(connection);
  }

  @Override
  public boolean allProceduresAreCallable() {
    return capabilities.allProceduresAreCallable();
  }

  @Override
  public boolean allTablesAreSelectable() {
    return capabilities.allTablesAreSelectable();
  }

  @Override
  public String getURL() {
    return identity.getURL();
  }

  @Override
  public String getUserName() {
    return identity.getUserName();
  }

  @Override
  public boolean isReadOnly() {
    return capabilities.isReadOnly();
  }

  @Override
  public boolean nullsAreSortedHigh() {
    return capabilities.nullsAreSortedHigh();
  }

  @Override
  public boolean nullsAreSortedLow() {
    return capabilities.nullsAreSortedLow();
  }

  @Override
  public boolean nullsAreSortedAtStart() {
    return capabilities.nullsAreSortedAtStart();
  }

  @Override
  public boolean nullsAreSortedAtEnd() {
    return capabilities.nullsAreSortedAtEnd();
  }

  @Override
  public String getDatabaseProductName() {
    return identity.getDatabaseProductName();
  }

  @Override
  public String getDatabaseProductVersion() {
    return identity.getDatabaseProductVersion();
  }

  @Override
  public String getDriverName() {
    return identity.getDriverName();
  }

  @Override
  public String getDriverVersion() {
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
  public boolean usesLocalFiles() {
    return capabilities.usesLocalFiles();
  }

  @Override
  public boolean usesLocalFilePerTable() {
    return capabilities.usesLocalFilePerTable();
  }

  @Override
  public boolean supportsMixedCaseIdentifiers() {
    return capabilities.supportsMixedCaseIdentifiers();
  }

  @Override
  public boolean storesUpperCaseIdentifiers() {
    return capabilities.storesUpperCaseIdentifiers();
  }

  @Override
  public boolean storesLowerCaseIdentifiers() {
    return capabilities.storesLowerCaseIdentifiers();
  }

  @Override
  public boolean storesMixedCaseIdentifiers() {
    return capabilities.storesMixedCaseIdentifiers();
  }

  @Override
  public boolean supportsMixedCaseQuotedIdentifiers() {
    return capabilities.supportsMixedCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesUpperCaseQuotedIdentifiers() {
    return capabilities.storesUpperCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesLowerCaseQuotedIdentifiers() {
    return capabilities.storesLowerCaseQuotedIdentifiers();
  }

  @Override
  public boolean storesMixedCaseQuotedIdentifiers() {
    return capabilities.storesMixedCaseQuotedIdentifiers();
  }

  @Override
  public String getIdentifierQuoteString() {
    return identity.getIdentifierQuoteString();
  }

  @Override
  public String getSQLKeywords() {
    return identity.getSQLKeywords();
  }

  @Override
  public String getNumericFunctions() {
    return identity.getNumericFunctions();
  }

  @Override
  public String getStringFunctions() {
    return identity.getStringFunctions();
  }

  @Override
  public String getSystemFunctions() {
    return identity.getSystemFunctions();
  }

  @Override
  public String getTimeDateFunctions() {
    return identity.getTimeDateFunctions();
  }

  @Override
  public String getSearchStringEscape() {
    return identity.getSearchStringEscape();
  }

  @Override
  public String getExtraNameCharacters() {
    return identity.getExtraNameCharacters();
  }

  @Override
  public boolean supportsAlterTableWithAddColumn() {
    return capabilities.supportsAlterTableWithAddColumn();
  }

  @Override
  public boolean supportsAlterTableWithDropColumn() {
    return capabilities.supportsAlterTableWithDropColumn();
  }

  @Override
  public boolean supportsColumnAliasing() {
    return capabilities.supportsColumnAliasing();
  }

  @Override
  public boolean nullPlusNonNullIsNull() {
    return capabilities.nullPlusNonNullIsNull();
  }

  @Override
  public boolean supportsConvert() {
    return capabilities.supportsConvert();
  }

  @Override
  public boolean supportsConvert(int fromType, int toType) {
    return capabilities.supportsConvert(fromType, toType);
  }

  @Override
  public boolean supportsTableCorrelationNames() {
    return capabilities.supportsTableCorrelationNames();
  }

  @Override
  public boolean supportsDifferentTableCorrelationNames() {
    return capabilities.supportsDifferentTableCorrelationNames();
  }

  @Override
  public boolean supportsExpressionsInOrderBy() {
    return capabilities.supportsExpressionsInOrderBy();
  }

  @Override
  public boolean supportsOrderByUnrelated() {
    return capabilities.supportsOrderByUnrelated();
  }

  @Override
  public boolean supportsGroupBy() {
    return capabilities.supportsGroupBy();
  }

  @Override
  public boolean supportsGroupByUnrelated() {
    return capabilities.supportsGroupByUnrelated();
  }

  @Override
  public boolean supportsGroupByBeyondSelect() {
    return capabilities.supportsGroupByBeyondSelect();
  }

  @Override
  public boolean supportsLikeEscapeClause() {
    return capabilities.supportsLikeEscapeClause();
  }

  @Override
  public boolean supportsMultipleResultSets() {
    return capabilities.supportsMultipleResultSets();
  }

  @Override
  public boolean supportsMultipleTransactions() {
    return capabilities.supportsMultipleTransactions();
  }

  @Override
  public boolean supportsNonNullableColumns() {
    return capabilities.supportsNonNullableColumns();
  }

  @Override
  public boolean supportsMinimumSQLGrammar() {
    return capabilities.supportsMinimumSQLGrammar();
  }

  @Override
  public boolean supportsCoreSQLGrammar() {
    return capabilities.supportsCoreSQLGrammar();
  }

  @Override
  public boolean supportsExtendedSQLGrammar() {
    return capabilities.supportsExtendedSQLGrammar();
  }

  @Override
  public boolean supportsANSI92EntryLevelSQL() {
    return capabilities.supportsANSI92EntryLevelSQL();
  }

  @Override
  public boolean supportsANSI92IntermediateSQL() {
    return capabilities.supportsANSI92IntermediateSQL();
  }

  @Override
  public boolean supportsANSI92FullSQL() {
    return capabilities.supportsANSI92FullSQL();
  }

  @Override
  public boolean supportsIntegrityEnhancementFacility() {
    return capabilities.supportsIntegrityEnhancementFacility();
  }

  @Override
  public boolean supportsOuterJoins() {
    return capabilities.supportsOuterJoins();
  }

  @Override
  public boolean supportsFullOuterJoins() {
    return capabilities.supportsFullOuterJoins();
  }

  @Override
  public boolean supportsLimitedOuterJoins() {
    return capabilities.supportsLimitedOuterJoins();
  }

  @Override
  public String getSchemaTerm() {
    return identity.getSchemaTerm();
  }

  @Override
  public String getProcedureTerm() {
    return identity.getProcedureTerm();
  }

  @Override
  public String getCatalogTerm() {
    return identity.getCatalogTerm();
  }

  @Override
  public boolean isCatalogAtStart() {
    return capabilities.isCatalogAtStart();
  }

  @Override
  public String getCatalogSeparator() {
    return identity.getCatalogSeparator();
  }

  @Override
  public boolean supportsSchemasInDataManipulation() {
    return capabilities.supportsSchemasInDataManipulation();
  }

  @Override
  public boolean supportsSchemasInProcedureCalls() {
    return capabilities.supportsSchemasInProcedureCalls();
  }

  @Override
  public boolean supportsSchemasInTableDefinitions() {
    return capabilities.supportsSchemasInTableDefinitions();
  }

  @Override
  public boolean supportsSchemasInIndexDefinitions() {
    return capabilities.supportsSchemasInIndexDefinitions();
  }

  @Override
  public boolean supportsSchemasInPrivilegeDefinitions() {
    return capabilities.supportsSchemasInPrivilegeDefinitions();
  }

  @Override
  public boolean supportsCatalogsInDataManipulation() {
    return capabilities.supportsCatalogsInDataManipulation();
  }

  @Override
  public boolean supportsCatalogsInProcedureCalls() {
    return capabilities.supportsCatalogsInProcedureCalls();
  }

  @Override
  public boolean supportsCatalogsInTableDefinitions() {
    return capabilities.supportsCatalogsInTableDefinitions();
  }

  @Override
  public boolean supportsCatalogsInIndexDefinitions() {
    return capabilities.supportsCatalogsInIndexDefinitions();
  }

  @Override
  public boolean supportsCatalogsInPrivilegeDefinitions() {
    return capabilities.supportsCatalogsInPrivilegeDefinitions();
  }

  @Override
  public boolean supportsPositionedDelete() {
    return capabilities.supportsPositionedDelete();
  }

  @Override
  public boolean supportsPositionedUpdate() {
    return capabilities.supportsPositionedUpdate();
  }

  @Override
  public boolean supportsSelectForUpdate() {
    return capabilities.supportsSelectForUpdate();
  }

  @Override
  public boolean supportsStoredProcedures() {
    return capabilities.supportsStoredProcedures();
  }

  @Override
  public boolean supportsSubqueriesInComparisons() {
    return capabilities.supportsSubqueriesInComparisons();
  }

  @Override
  public boolean supportsSubqueriesInExists() {
    return capabilities.supportsSubqueriesInExists();
  }

  @Override
  public boolean supportsSubqueriesInIns() {
    return capabilities.supportsSubqueriesInIns();
  }

  @Override
  public boolean supportsSubqueriesInQuantifieds() {
    return capabilities.supportsSubqueriesInQuantifieds();
  }

  @Override
  public boolean supportsCorrelatedSubqueries() {
    return capabilities.supportsCorrelatedSubqueries();
  }

  @Override
  public boolean supportsUnion() {
    return capabilities.supportsUnion();
  }

  @Override
  public boolean supportsUnionAll() {
    return capabilities.supportsUnionAll();
  }

  @Override
  public boolean supportsOpenCursorsAcrossCommit() {
    return capabilities.supportsOpenCursorsAcrossCommit();
  }

  @Override
  public boolean supportsOpenCursorsAcrossRollback() {
    return capabilities.supportsOpenCursorsAcrossRollback();
  }

  @Override
  public boolean supportsOpenStatementsAcrossCommit() {
    return capabilities.supportsOpenStatementsAcrossCommit();
  }

  @Override
  public boolean supportsOpenStatementsAcrossRollback() {
    return capabilities.supportsOpenStatementsAcrossRollback();
  }

  @Override
  public int getMaxBinaryLiteralLength() {
    return limits.getMaxBinaryLiteralLength();
  }

  @Override
  public int getMaxCharLiteralLength() {
    return limits.getMaxCharLiteralLength();
  }

  @Override
  public int getMaxColumnNameLength() {
    return limits.getMaxColumnNameLength();
  }

  @Override
  public int getMaxColumnsInGroupBy() {
    return limits.getMaxColumnsInGroupBy();
  }

  @Override
  public int getMaxColumnsInIndex() {
    return limits.getMaxColumnsInIndex();
  }

  @Override
  public int getMaxColumnsInOrderBy() {
    return limits.getMaxColumnsInOrderBy();
  }

  @Override
  public int getMaxColumnsInSelect() {
    return limits.getMaxColumnsInSelect();
  }

  @Override
  public int getMaxColumnsInTable() {
    return limits.getMaxColumnsInTable();
  }

  @Override
  public int getMaxConnections() {
    return limits.getMaxConnections();
  }

  @Override
  public int getMaxCursorNameLength() {
    return limits.getMaxCursorNameLength();
  }

  @Override
  public int getMaxIndexLength() {
    return limits.getMaxIndexLength();
  }

  @Override
  public int getMaxSchemaNameLength() {
    return limits.getMaxSchemaNameLength();
  }

  @Override
  public int getMaxProcedureNameLength() {
    return limits.getMaxProcedureNameLength();
  }

  @Override
  public int getMaxCatalogNameLength() {
    return limits.getMaxCatalogNameLength();
  }

  @Override
  public int getMaxRowSize() {
    return limits.getMaxRowSize();
  }

  @Override
  public boolean doesMaxRowSizeIncludeBlobs() {
    return limits.doesMaxRowSizeIncludeBlobs();
  }

  @Override
  public int getMaxStatementLength() {
    return limits.getMaxStatementLength();
  }

  @Override
  public int getMaxStatements() {
    return limits.getMaxStatements();
  }

  @Override
  public int getMaxTableNameLength() {
    return limits.getMaxTableNameLength();
  }

  @Override
  public int getMaxTablesInSelect() {
    return limits.getMaxTablesInSelect();
  }

  @Override
  public int getMaxUserNameLength() {
    return limits.getMaxUserNameLength();
  }

  @Override
  public int getDefaultTransactionIsolation() {
    return capabilities.getDefaultTransactionIsolation();
  }

  @Override
  public boolean supportsTransactions() {
    return capabilities.supportsTransactions();
  }

  @Override
  public boolean supportsTransactionIsolationLevel(int level) {
    return capabilities.supportsTransactionIsolationLevel(level);
  }

  @Override
  public boolean supportsDataDefinitionAndDataManipulationTransactions() {
    return capabilities.supportsDataDefinitionAndDataManipulationTransactions();
  }

  @Override
  public boolean supportsDataManipulationTransactionsOnly() {
    return capabilities.supportsDataManipulationTransactionsOnly();
  }

  @Override
  public boolean dataDefinitionCausesTransactionCommit() {
    return capabilities.dataDefinitionCausesTransactionCommit();
  }

  @Override
  public boolean dataDefinitionIgnoredInTransactions() {
    return capabilities.dataDefinitionIgnoredInTransactions();
  }

  @Override
  public ResultSet getProcedures(
      String catalog, String schemaPattern, String procedureNamePattern) {
    connection.checkClosed();
    return decorated(objects.getProcedures(catalog, schemaPattern, procedureNamePattern));
  }

  @Override
  public ResultSet getProcedureColumns(
      String catalog, String schemaPattern, String procedureNamePattern, String columnNamePattern) {
    connection.checkClosed();
    return decorated(
        objects.getProcedureColumns(
            catalog, schemaPattern, procedureNamePattern, columnNamePattern));
  }

  @Override
  public ResultSet getTables(
      String catalog, String schemaPattern, String tableNamePattern, String[] types) {
    connection.checkClosed();
    return decorated(objects.getTables(catalog, schemaPattern, tableNamePattern, types));
  }

  @Override
  public ResultSet getSchemas() {
    connection.checkClosed();
    return getSchemas(null, null);
  }

  @Override
  public ResultSet getCatalogs() {
    connection.checkClosed();
    return decorated(objects.getCatalogs());
  }

  @Override
  public ResultSet getTableTypes() {
    connection.checkClosed();
    return decorated(objects.getTableTypes());
  }

  @Override
  public ResultSet getColumns(
      String catalog, String schemaPattern, String tableNamePattern, String columnNamePattern) {
    connection.checkClosed();
    return decorated(
        objects.getColumns(catalog, schemaPattern, tableNamePattern, columnNamePattern, false));
  }

  @Override
  public ResultSet getColumnPrivileges(
      String catalog, String schema, String table, String columnNamePattern) {
    connection.checkClosed();
    return decorated(objects.getColumnPrivileges(catalog, schema, table, columnNamePattern));
  }

  @Override
  public ResultSet getTablePrivileges(
      String catalog, String schemaPattern, String tableNamePattern) {
    connection.checkClosed();
    return decorated(objects.getTablePrivileges(catalog, schemaPattern, tableNamePattern));
  }

  @Override
  public ResultSet getBestRowIdentifier(
      String catalog, String schema, String table, int scope, boolean nullable) {
    throw new SFSQLFeatureNotSupportedException("getBestRowIdentifier not supported");
  }

  @Override
  public ResultSet getVersionColumns(String catalog, String schema, String table) {
    throw new SFSQLFeatureNotSupportedException("getVersionColumns not supported");
  }

  @Override
  public ResultSet getPrimaryKeys(String catalog, String schema, String table) {
    connection.checkClosed();
    return decorated(objects.getPrimaryKeys(catalog, schema, table));
  }

  @Override
  public ResultSet getImportedKeys(String catalog, String schema, String table) {
    connection.checkClosed();
    return decorated(
        objects.getForeignKeys(ForeignKeyKind.IMPORTED, catalog, schema, table, null, null, null));
  }

  @Override
  public ResultSet getExportedKeys(String catalog, String schema, String table) {
    connection.checkClosed();
    return decorated(
        objects.getForeignKeys(ForeignKeyKind.EXPORTED, catalog, schema, table, null, null, null));
  }

  @Override
  public ResultSet getCrossReference(
      String parentCatalog,
      String parentSchema,
      String parentTable,
      String foreignCatalog,
      String foreignSchema,
      String foreignTable) {
    connection.checkClosed();
    return decorated(
        objects.getForeignKeys(
            ForeignKeyKind.CROSS_REFERENCE,
            parentCatalog,
            parentSchema,
            parentTable,
            foreignCatalog,
            foreignSchema,
            foreignTable));
  }

  @Override
  public ResultSet getTypeInfo() {
    connection.checkClosed();
    return decorated(objects.getTypeInfo());
  }

  @Override
  public ResultSet getIndexInfo(
      String catalog, String schema, String table, boolean unique, boolean approximate) {
    connection.checkClosed();
    return decorated(objects.getIndexInfo(catalog, schema, table, unique, approximate));
  }

  @Override
  public boolean supportsResultSetType(int type) {
    return capabilities.supportsResultSetType(type);
  }

  @Override
  public boolean supportsResultSetConcurrency(int type, int concurrency) {
    return capabilities.supportsResultSetConcurrency(type, concurrency);
  }

  @Override
  public boolean ownUpdatesAreVisible(int type) {
    return capabilities.ownUpdatesAreVisible(type);
  }

  @Override
  public boolean ownDeletesAreVisible(int type) {
    return capabilities.ownDeletesAreVisible(type);
  }

  @Override
  public boolean ownInsertsAreVisible(int type) {
    return capabilities.ownInsertsAreVisible(type);
  }

  @Override
  public boolean othersUpdatesAreVisible(int type) {
    return capabilities.othersUpdatesAreVisible(type);
  }

  @Override
  public boolean othersDeletesAreVisible(int type) {
    return capabilities.othersDeletesAreVisible(type);
  }

  @Override
  public boolean othersInsertsAreVisible(int type) {
    return capabilities.othersInsertsAreVisible(type);
  }

  @Override
  public boolean updatesAreDetected(int type) {
    return capabilities.updatesAreDetected(type);
  }

  @Override
  public boolean deletesAreDetected(int type) {
    return capabilities.deletesAreDetected(type);
  }

  @Override
  public boolean insertsAreDetected(int type) {
    return capabilities.insertsAreDetected(type);
  }

  @Override
  public boolean supportsBatchUpdates() {
    return capabilities.supportsBatchUpdates();
  }

  @Override
  public ResultSet getUDTs(
      String catalog, String schemaPattern, String typeNamePattern, int[] types) {
    connection.checkClosed();
    return decorated(objects.getUDTs(catalog, schemaPattern, typeNamePattern, types));
  }

  @Override
  public Connection getConnection() {
    connection.checkClosed();
    return Decorators.connection(connection, connection.getTelemetry());
  }

  /** Wraps a raw metadata result set in its decorated boundary before it leaves this class. */
  private ResultSet decorated(ResultSet resultSet) {
    return Decorators.resultSet(resultSet, connection.getTelemetry());
  }

  // Additional JDBC 3.0+ methods (stubs)
  @Override
  public boolean supportsSavepoints() {
    return capabilities.supportsSavepoints();
  }

  @Override
  public boolean supportsNamedParameters() {
    return capabilities.supportsNamedParameters();
  }

  @Override
  public boolean supportsMultipleOpenResults() {
    return capabilities.supportsMultipleOpenResults();
  }

  @Override
  public boolean supportsGetGeneratedKeys() {
    return capabilities.supportsGetGeneratedKeys();
  }

  @Override
  public ResultSet getSuperTypes(String catalog, String schemaPattern, String typeNamePattern) {
    throw new SFSQLFeatureNotSupportedException("getSuperTypes not supported");
  }

  @Override
  public ResultSet getSuperTables(String catalog, String schemaPattern, String tableNamePattern) {
    throw new SFSQLFeatureNotSupportedException("getSuperTables not supported");
  }

  @Override
  public ResultSet getAttributes(
      String catalog, String schemaPattern, String typeNamePattern, String attributeNamePattern) {
    throw new SFSQLFeatureNotSupportedException("getAttributes not supported");
  }

  @Override
  public boolean supportsResultSetHoldability(int holdability) {
    return capabilities.supportsResultSetHoldability(holdability);
  }

  @Override
  public int getResultSetHoldability() {
    return capabilities.getResultSetHoldability();
  }

  @Override
  public int getDatabaseMajorVersion() {
    return identity.getDatabaseMajorVersion();
  }

  @Override
  public int getDatabaseMinorVersion() {
    return identity.getDatabaseMinorVersion();
  }

  @Override
  public int getJDBCMajorVersion() {
    return identity.getJDBCMajorVersion();
  }

  @Override
  public int getJDBCMinorVersion() {
    return identity.getJDBCMinorVersion();
  }

  @Override
  public int getSQLStateType() {
    return capabilities.getSQLStateType();
  }

  @Override
  public boolean locatorsUpdateCopy() {
    return capabilities.locatorsUpdateCopy();
  }

  @Override
  public boolean supportsStatementPooling() {
    return capabilities.supportsStatementPooling();
  }

  @Override
  public RowIdLifetime getRowIdLifetime() {
    throw new SFSQLFeatureNotSupportedException("getRowIdLifetime not supported");
  }

  @Override
  public ResultSet getSchemas(String catalog, String schemaPattern) {
    connection.checkClosed();
    return decorated(objects.getSchemas(catalog, schemaPattern));
  }

  @Override
  public boolean supportsStoredFunctionsUsingCallSyntax() {
    return capabilities.supportsStoredFunctionsUsingCallSyntax();
  }

  @Override
  public boolean autoCommitFailureClosesAllResultSets() {
    throw new SFSQLFeatureNotSupportedException(
        "autoCommitFailureClosesAllResultSets not supported");
  }

  @Override
  public ResultSet getClientInfoProperties() {
    throw new SFSQLFeatureNotSupportedException("getClientInfoProperties not supported");
  }

  @Override
  public ResultSet getFunctions(String catalog, String schemaPattern, String functionNamePattern) {
    connection.checkClosed();
    return decorated(objects.getFunctions(catalog, schemaPattern, functionNamePattern));
  }

  @Override
  public ResultSet getFunctionColumns(
      String catalog, String schemaPattern, String functionNamePattern, String columnNamePattern) {
    connection.checkClosed();
    return decorated(
        objects.getFunctionColumns(catalog, schemaPattern, functionNamePattern, columnNamePattern));
  }

  @Override
  public ResultSet getPseudoColumns(
      String catalog, String schemaPattern, String tableNamePattern, String columnNamePattern) {
    throw new SFSQLFeatureNotSupportedException("getPseudoColumns not supported");
  }

  @Override
  public boolean generatedKeyAlwaysReturned() {
    throw new SFSQLFeatureNotSupportedException("generatedKeyAlwaysReturned not supported");
  }

  @Override
  public ResultSet getStreams(String catalog, String schemaPattern, String streamName) {
    connection.checkClosed();
    return decorated(objects.getStreams(catalog, schemaPattern, streamName));
  }

  @Override
  public ResultSet getColumns(
      String catalog,
      String schemaPattern,
      String tableNamePattern,
      String columnNamePattern,
      boolean extendedSet) {
    connection.checkClosed();
    return decorated(
        objects.getColumns(
            catalog, schemaPattern, tableNamePattern, columnNamePattern, extendedSet));
  }
}
