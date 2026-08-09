package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import static java.sql.DatabaseMetaData.sqlStateSQL;

import java.sql.Connection;
import java.sql.ResultSet;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;

@RequiredArgsConstructor
public final class MetaDataCapabilities {

  private final InternalSnowflakeConnection connection;

  public boolean allProceduresAreCallable() {
    connection.checkClosed();
    return false;
  }

  public boolean allTablesAreSelectable() {
    connection.checkClosed();
    return true;
  }

  public boolean isReadOnly() {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedHigh() {
    connection.checkClosed();
    return true;
  }

  public boolean nullsAreSortedLow() {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedAtStart() {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedAtEnd() {
    connection.checkClosed();
    return false;
  }

  public boolean usesLocalFiles() {
    connection.checkClosed();
    return false;
  }

  public boolean usesLocalFilePerTable() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMixedCaseIdentifiers() {
    connection.checkClosed();
    return false;
  }

  public boolean storesUpperCaseIdentifiers() {
    connection.checkClosed();
    return true;
  }

  public boolean storesLowerCaseIdentifiers() {
    connection.checkClosed();
    return false;
  }

  public boolean storesMixedCaseIdentifiers() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMixedCaseQuotedIdentifiers() {
    connection.checkClosed();
    return true;
  }

  public boolean storesUpperCaseQuotedIdentifiers() {
    connection.checkClosed();
    return false;
  }

  public boolean storesLowerCaseQuotedIdentifiers() {
    connection.checkClosed();
    return false;
  }

  public boolean storesMixedCaseQuotedIdentifiers() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsAlterTableWithAddColumn() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsAlterTableWithDropColumn() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsColumnAliasing() {
    connection.checkClosed();
    return true;
  }

  public boolean nullPlusNonNullIsNull() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsConvert() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsConvert(int fromType, int toType) {
    connection.checkClosed();
    return false;
  }

  public boolean supportsTableCorrelationNames() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsDifferentTableCorrelationNames() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsExpressionsInOrderBy() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsOrderByUnrelated() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsGroupBy() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsGroupByUnrelated() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsGroupByBeyondSelect() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsLikeEscapeClause() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMultipleResultSets() {
    connection.checkClosed();
    // TODO: it should be true when we support multi statements
    return false;
  }

  public boolean supportsMultipleTransactions() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsNonNullableColumns() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsMinimumSQLGrammar() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCoreSQLGrammar() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsExtendedSQLGrammar() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsANSI92EntryLevelSQL() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsANSI92IntermediateSQL() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsANSI92FullSQL() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsIntegrityEnhancementFacility() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOuterJoins() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsFullOuterJoins() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsLimitedOuterJoins() {
    connection.checkClosed();
    return true;
  }

  public boolean isCatalogAtStart() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInDataManipulation() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInProcedureCalls() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSchemasInTableDefinitions() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInIndexDefinitions() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSchemasInPrivilegeDefinitions() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInDataManipulation() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsCatalogsInProcedureCalls() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInTableDefinitions() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsCatalogsInIndexDefinitions() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInPrivilegeDefinitions() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsPositionedDelete() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsPositionedUpdate() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSelectForUpdate() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsStoredProcedures() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInComparisons() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInExists() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInIns() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInQuantifieds() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCorrelatedSubqueries() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsUnion() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsUnionAll() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsOpenCursorsAcrossCommit() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenCursorsAcrossRollback() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenStatementsAcrossCommit() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenStatementsAcrossRollback() {
    connection.checkClosed();
    return false;
  }

  public int getDefaultTransactionIsolation() {
    connection.checkClosed();
    return Connection.TRANSACTION_READ_COMMITTED;
  }

  public boolean supportsTransactions() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsTransactionIsolationLevel(int level) {
    connection.checkClosed();
    return (level == Connection.TRANSACTION_NONE)
        || (level == Connection.TRANSACTION_READ_COMMITTED);
  }

  public boolean supportsDataDefinitionAndDataManipulationTransactions() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsDataManipulationTransactionsOnly() {
    connection.checkClosed();
    return false;
  }

  public boolean dataDefinitionCausesTransactionCommit() {
    connection.checkClosed();
    return true;
  }

  public boolean dataDefinitionIgnoredInTransactions() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsResultSetType(int type) {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY);
  }

  public boolean supportsResultSetConcurrency(int type, int concurrency) {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY && concurrency == ResultSet.CONCUR_READ_ONLY);
  }

  public boolean ownUpdatesAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean ownDeletesAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean ownInsertsAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean othersUpdatesAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean othersDeletesAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean othersInsertsAreVisible(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean updatesAreDetected(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean deletesAreDetected(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean insertsAreDetected(int type) {
    connection.checkClosed();
    return false;
  }

  public boolean supportsBatchUpdates() {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSavepoints() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsNamedParameters() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMultipleOpenResults() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsGetGeneratedKeys() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsResultSetHoldability(int holdability) {
    connection.checkClosed();
    return holdability == ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  public int getResultSetHoldability() {
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  public int getSQLStateType() {
    return sqlStateSQL;
  }

  public boolean locatorsUpdateCopy() {
    return false;
  }

  public boolean supportsStatementPooling() {
    connection.checkClosed();
    return false;
  }

  public boolean supportsStoredFunctionsUsingCallSyntax() {
    connection.checkClosed();
    return true;
  }
}
