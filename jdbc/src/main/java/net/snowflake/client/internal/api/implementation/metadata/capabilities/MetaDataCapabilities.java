package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import static java.sql.DatabaseMetaData.sqlStateSQL;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;

@RequiredArgsConstructor
public final class MetaDataCapabilities {

  private final InternalSnowflakeConnection connection;

  public boolean allProceduresAreCallable() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean allTablesAreSelectable() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean isReadOnly() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedHigh() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean nullsAreSortedLow() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedAtStart() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean nullsAreSortedAtEnd() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean usesLocalFiles() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean usesLocalFilePerTable() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMixedCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean storesUpperCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean storesLowerCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean storesMixedCaseIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMixedCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean storesUpperCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean storesLowerCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean storesMixedCaseQuotedIdentifiers() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsAlterTableWithAddColumn() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsAlterTableWithDropColumn() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsColumnAliasing() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean nullPlusNonNullIsNull() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsConvert() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsConvert(int fromType, int toType) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsTableCorrelationNames() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsDifferentTableCorrelationNames() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsExpressionsInOrderBy() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsOrderByUnrelated() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsGroupBy() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsGroupByUnrelated() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsGroupByBeyondSelect() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsLikeEscapeClause() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMultipleResultSets() throws SQLException {
    connection.checkClosed();
    // TODO: it should be true when we support multi statements
    return false;
  }

  public boolean supportsMultipleTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsNonNullableColumns() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsMinimumSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCoreSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsExtendedSQLGrammar() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsANSI92EntryLevelSQL() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsANSI92IntermediateSQL() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsANSI92FullSQL() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsIntegrityEnhancementFacility() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsFullOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsLimitedOuterJoins() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean isCatalogAtStart() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInDataManipulation() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInProcedureCalls() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSchemasInTableDefinitions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSchemasInIndexDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSchemasInPrivilegeDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInDataManipulation() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsCatalogsInProcedureCalls() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInTableDefinitions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsCatalogsInIndexDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCatalogsInPrivilegeDefinitions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsPositionedDelete() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsPositionedUpdate() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsSelectForUpdate() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsStoredProcedures() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInComparisons() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInExists() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInIns() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSubqueriesInQuantifieds() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsCorrelatedSubqueries() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsUnion() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsUnionAll() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsOpenCursorsAcrossCommit() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenCursorsAcrossRollback() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenStatementsAcrossCommit() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsOpenStatementsAcrossRollback() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public int getDefaultTransactionIsolation() throws SQLException {
    connection.checkClosed();
    return Connection.TRANSACTION_READ_COMMITTED;
  }

  public boolean supportsTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsTransactionIsolationLevel(int level) throws SQLException {
    connection.checkClosed();
    return (level == Connection.TRANSACTION_NONE)
        || (level == Connection.TRANSACTION_READ_COMMITTED);
  }

  public boolean supportsDataDefinitionAndDataManipulationTransactions() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsDataManipulationTransactionsOnly() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean dataDefinitionCausesTransactionCommit() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean dataDefinitionIgnoredInTransactions() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsResultSetType(int type) throws SQLException {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY);
  }

  public boolean supportsResultSetConcurrency(int type, int concurrency) throws SQLException {
    connection.checkClosed();
    return (type == ResultSet.TYPE_FORWARD_ONLY && concurrency == ResultSet.CONCUR_READ_ONLY);
  }

  public boolean ownUpdatesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean ownDeletesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean ownInsertsAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean othersUpdatesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean othersDeletesAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean othersInsertsAreVisible(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean updatesAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean deletesAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean insertsAreDetected(int type) throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsBatchUpdates() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public boolean supportsSavepoints() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsNamedParameters() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsMultipleOpenResults() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsGetGeneratedKeys() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsResultSetHoldability(int holdability) throws SQLException {
    connection.checkClosed();
    return holdability == ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  public int getResultSetHoldability() throws SQLException {
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  public int getSQLStateType() throws SQLException {
    return sqlStateSQL;
  }

  public boolean locatorsUpdateCopy() throws SQLException {
    return false;
  }

  public boolean supportsStatementPooling() throws SQLException {
    connection.checkClosed();
    return false;
  }

  public boolean supportsStoredFunctionsUsingCallSyntax() throws SQLException {
    connection.checkClosed();
    return true;
  }
}
