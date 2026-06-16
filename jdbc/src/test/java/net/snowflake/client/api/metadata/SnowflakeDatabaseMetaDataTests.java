package net.snowflake.client.api.metadata;

import static net.snowflake.jdbc.utils.TestParameters.loadConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Types;
import java.util.Properties;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

/**
 * Cross-driver parity tests for {@link DatabaseMetaData}.
 *
 * <p>These tests use only the public JDBC API so they run against both this module and the legacy
 * {@code snowflake-jdbc} JAR via the {@code referenceTest} task. They cover happy-path values for
 * methods that don't issue a query against Snowflake. ResultSet-returning methods and methods that
 * universal-driver has not yet implemented are present as {@link Disabled} stubs to be filled in
 * follow-up PRs.
 */
class SnowflakeDatabaseMetaDataTests extends SnowflakeIntegrationTestBase {

  private DatabaseMetaData metaData() throws Exception {
    return getDefaultConnection().getMetaData();
  }

  // ---------- Identity ----------

  @Test
  void getDatabaseProductNameReturnsSnowflake() throws Exception {
    assertEquals("Snowflake", metaData().getDatabaseProductName());
  }

  @Test
  void getDriverNameContainsSnowflake() throws Exception {
    // Both drivers identify as a Snowflake driver: legacy returns "Snowflake",
    // universal-driver returns "Snowflake JDBC Driver". Substring is the
    // strictest assertion that still holds across both.
    assertTrue(metaData().getDriverName().contains("Snowflake"));
  }

  @Test
  void versionMetadataIsConsistent() throws Exception {
    DatabaseMetaData md = metaData();

    String version = md.getDriverVersion();
    assertTrue(
        version.matches("\\d+\\.\\d+(\\.\\d+)?(-.*)?"),
        () -> "driver version not in major.minor[.patch][-suffix] form: " + version);

    int major = md.getDriverMajorVersion();
    int minor = md.getDriverMinorVersion();
    String[] parts = version.split("[.\\-]");
    assertEquals(Integer.parseInt(parts[0]), major, "driver major must match driver version");
    assertEquals(Integer.parseInt(parts[1]), minor, "driver minor must match driver version");

    // JDBC API level shipped by both drivers.
    assertEquals(4, md.getJDBCMajorVersion());
    assertEquals(2, md.getJDBCMinorVersion());
  }

  @Test
  void getIdentifierQuoteStringIsDoubleQuote() throws Exception {
    assertEquals("\"", metaData().getIdentifierQuoteString());
  }

  @Test
  void getSearchStringEscapeIsBackslash() throws Exception {
    assertEquals("\\", metaData().getSearchStringEscape());
  }

  @Test
  void getExtraNameCharactersIsDollar() throws Exception {
    assertEquals("$", metaData().getExtraNameCharacters());
  }

  @Test
  void getSchemaTermIsSchema() throws Exception {
    assertEquals("schema", metaData().getSchemaTerm());
  }

  @Test
  void getProcedureTermIsProcedure() throws Exception {
    assertEquals("procedure", metaData().getProcedureTerm());
  }

  @Test
  void getCatalogTermIsDatabase() throws Exception {
    assertEquals("database", metaData().getCatalogTerm());
  }

  @Test
  void getCatalogSeparatorIsDot() throws Exception {
    assertEquals(".", metaData().getCatalogSeparator());
  }

  @Test
  void getSQLStateTypeIsSqlStateSQL() throws Exception {
    assertEquals(DatabaseMetaData.sqlStateSQL, metaData().getSQLStateType());
  }

  // ---------- Capabilities: procedures, tables, read-only ----------

  @Test
  void allProceduresAreCallableIsFalse() throws Exception {
    assertFalse(metaData().allProceduresAreCallable());
  }

  @Test
  void allTablesAreSelectableIsTrue() throws Exception {
    assertTrue(metaData().allTablesAreSelectable());
  }

  @Test
  void isReadOnlyIsFalse() throws Exception {
    assertFalse(metaData().isReadOnly());
  }

  // ---------- Capabilities: null ordering ----------

  @Test
  void nullsAreSortedHighIsTrue() throws Exception {
    assertTrue(metaData().nullsAreSortedHigh());
  }

  @Test
  void nullsAreSortedLowIsFalse() throws Exception {
    assertFalse(metaData().nullsAreSortedLow());
  }

  @Test
  void nullsAreSortedAtStartIsFalse() throws Exception {
    assertFalse(metaData().nullsAreSortedAtStart());
  }

  @Test
  void nullsAreSortedAtEndIsFalse() throws Exception {
    assertFalse(metaData().nullsAreSortedAtEnd());
  }

  // ---------- Capabilities: file storage ----------

  @Test
  void usesLocalFilesIsFalse() throws Exception {
    assertFalse(metaData().usesLocalFiles());
  }

  @Test
  void usesLocalFilePerTableIsFalse() throws Exception {
    assertFalse(metaData().usesLocalFilePerTable());
  }

  // ---------- Capabilities: identifier case ----------

  @Test
  void supportsMixedCaseIdentifiersIsFalse() throws Exception {
    assertFalse(metaData().supportsMixedCaseIdentifiers());
  }

  @Test
  void storesUpperCaseIdentifiersIsTrue() throws Exception {
    assertTrue(metaData().storesUpperCaseIdentifiers());
  }

  @Test
  void storesLowerCaseIdentifiersIsFalse() throws Exception {
    assertFalse(metaData().storesLowerCaseIdentifiers());
  }

  @Test
  void storesMixedCaseIdentifiersIsFalse() throws Exception {
    assertFalse(metaData().storesMixedCaseIdentifiers());
  }

  @Test
  void supportsMixedCaseQuotedIdentifiersIsTrue() throws Exception {
    assertTrue(metaData().supportsMixedCaseQuotedIdentifiers());
  }

  @Test
  void storesUpperCaseQuotedIdentifiersIsFalse() throws Exception {
    assertFalse(metaData().storesUpperCaseQuotedIdentifiers());
  }

  @Test
  void storesLowerCaseQuotedIdentifiersIsFalse() throws Exception {
    assertFalse(metaData().storesLowerCaseQuotedIdentifiers());
  }

  @Test
  void storesMixedCaseQuotedIdentifiersIsTrue() throws Exception {
    assertTrue(metaData().storesMixedCaseQuotedIdentifiers());
  }

  // ---------- Capabilities: DDL/DML ----------

  @Test
  void supportsAlterTableWithAddColumnIsTrue() throws Exception {
    assertTrue(metaData().supportsAlterTableWithAddColumn());
  }

  @Test
  void supportsAlterTableWithDropColumnIsTrue() throws Exception {
    assertTrue(metaData().supportsAlterTableWithDropColumn());
  }

  @Test
  void supportsColumnAliasingIsTrue() throws Exception {
    assertTrue(metaData().supportsColumnAliasing());
  }

  @Test
  void nullPlusNonNullIsNullIsTrue() throws Exception {
    assertTrue(metaData().nullPlusNonNullIsNull());
  }

  @Test
  void supportsConvertIsFalse() throws Exception {
    assertFalse(metaData().supportsConvert());
  }

  @Test
  void supportsConvertWithTypesIsFalse() throws Exception {
    assertFalse(metaData().supportsConvert(Types.INTEGER, Types.VARCHAR));
  }

  // ---------- Capabilities: correlation, ordering, grouping ----------

  @Test
  void supportsTableCorrelationNamesIsTrue() throws Exception {
    assertTrue(metaData().supportsTableCorrelationNames());
  }

  @Test
  void supportsDifferentTableCorrelationNamesIsFalse() throws Exception {
    assertFalse(metaData().supportsDifferentTableCorrelationNames());
  }

  @Test
  void supportsExpressionsInOrderByIsTrue() throws Exception {
    assertTrue(metaData().supportsExpressionsInOrderBy());
  }

  @Test
  void supportsOrderByUnrelatedIsTrue() throws Exception {
    assertTrue(metaData().supportsOrderByUnrelated());
  }

  @Test
  void supportsGroupByIsTrue() throws Exception {
    assertTrue(metaData().supportsGroupBy());
  }

  @Test
  void supportsGroupByUnrelatedIsFalse() throws Exception {
    assertFalse(metaData().supportsGroupByUnrelated());
  }

  @Test
  void supportsGroupByBeyondSelectIsTrue() throws Exception {
    assertTrue(metaData().supportsGroupByBeyondSelect());
  }

  @Test
  void supportsLikeEscapeClauseIsFalse() throws Exception {
    assertFalse(metaData().supportsLikeEscapeClause());
  }

  // ---------- Capabilities: results, transactions, columns ----------

  @Test
  void supportsMultipleResultSetsIsFalse() throws Exception {
    assertFalse(metaData().supportsMultipleResultSets());
  }

  @Test
  void supportsMultipleTransactionsIsTrue() throws Exception {
    assertTrue(metaData().supportsMultipleTransactions());
  }

  @Test
  void supportsNonNullableColumnsIsTrue() throws Exception {
    assertTrue(metaData().supportsNonNullableColumns());
  }

  // ---------- Capabilities: SQL grammar ----------

  @Test
  void supportsMinimumSQLGrammarIsFalse() throws Exception {
    assertFalse(metaData().supportsMinimumSQLGrammar());
  }

  @Test
  void supportsCoreSQLGrammarIsFalse() throws Exception {
    assertFalse(metaData().supportsCoreSQLGrammar());
  }

  @Test
  void supportsExtendedSQLGrammarIsFalse() throws Exception {
    assertFalse(metaData().supportsExtendedSQLGrammar());
  }

  @Test
  void supportsANSI92EntryLevelSQLIsTrue() throws Exception {
    assertTrue(metaData().supportsANSI92EntryLevelSQL());
  }

  @Test
  void supportsANSI92IntermediateSQLIsFalse() throws Exception {
    assertFalse(metaData().supportsANSI92IntermediateSQL());
  }

  @Test
  void supportsANSI92FullSQLIsFalse() throws Exception {
    assertFalse(metaData().supportsANSI92FullSQL());
  }

  @Test
  void supportsIntegrityEnhancementFacilityIsFalse() throws Exception {
    assertFalse(metaData().supportsIntegrityEnhancementFacility());
  }

  // ---------- Capabilities: outer joins ----------

  @Test
  void supportsOuterJoinsIsTrue() throws Exception {
    assertTrue(metaData().supportsOuterJoins());
  }

  @Test
  void supportsFullOuterJoinsIsTrue() throws Exception {
    assertTrue(metaData().supportsFullOuterJoins());
  }

  @Test
  void supportsLimitedOuterJoinsIsTrue() throws Exception {
    assertTrue(metaData().supportsLimitedOuterJoins());
  }

  // ---------- Capabilities: catalog/schema placement ----------

  @Test
  void isCatalogAtStartIsTrue() throws Exception {
    assertTrue(metaData().isCatalogAtStart());
  }

  @Test
  void supportsSchemasInDataManipulationIsTrue() throws Exception {
    assertTrue(metaData().supportsSchemasInDataManipulation());
  }

  @Test
  void supportsSchemasInProcedureCallsIsFalse() throws Exception {
    assertFalse(metaData().supportsSchemasInProcedureCalls());
  }

  @Test
  void supportsSchemasInTableDefinitionsIsTrue() throws Exception {
    assertTrue(metaData().supportsSchemasInTableDefinitions());
  }

  @Test
  void supportsSchemasInIndexDefinitionsIsFalse() throws Exception {
    assertFalse(metaData().supportsSchemasInIndexDefinitions());
  }

  @Test
  void supportsSchemasInPrivilegeDefinitionsIsFalse() throws Exception {
    assertFalse(metaData().supportsSchemasInPrivilegeDefinitions());
  }

  @Test
  void supportsCatalogsInDataManipulationIsTrue() throws Exception {
    assertTrue(metaData().supportsCatalogsInDataManipulation());
  }

  @Test
  void supportsCatalogsInProcedureCallsIsFalse() throws Exception {
    assertFalse(metaData().supportsCatalogsInProcedureCalls());
  }

  @Test
  void supportsCatalogsInTableDefinitionsIsTrue() throws Exception {
    assertTrue(metaData().supportsCatalogsInTableDefinitions());
  }

  @Test
  void supportsCatalogsInIndexDefinitionsIsFalse() throws Exception {
    assertFalse(metaData().supportsCatalogsInIndexDefinitions());
  }

  @Test
  void supportsCatalogsInPrivilegeDefinitionsIsFalse() throws Exception {
    assertFalse(metaData().supportsCatalogsInPrivilegeDefinitions());
  }

  // ---------- Capabilities: positioned/select-for-update/stored procedures ----------

  @Test
  void supportsPositionedDeleteIsFalse() throws Exception {
    assertFalse(metaData().supportsPositionedDelete());
  }

  @Test
  void supportsPositionedUpdateIsFalse() throws Exception {
    assertFalse(metaData().supportsPositionedUpdate());
  }

  @Test
  void supportsSelectForUpdateIsFalse() throws Exception {
    assertFalse(metaData().supportsSelectForUpdate());
  }

  @Test
  void supportsStoredProceduresIsTrue() throws Exception {
    assertTrue(metaData().supportsStoredProcedures());
  }

  // ---------- Capabilities: subqueries, set operations ----------

  @Test
  void supportsSubqueriesInComparisonsIsTrue() throws Exception {
    assertTrue(metaData().supportsSubqueriesInComparisons());
  }

  @Test
  void supportsSubqueriesInExistsIsTrue() throws Exception {
    assertTrue(metaData().supportsSubqueriesInExists());
  }

  @Test
  void supportsSubqueriesInInsIsTrue() throws Exception {
    assertTrue(metaData().supportsSubqueriesInIns());
  }

  @Test
  void supportsSubqueriesInQuantifiedsIsFalse() throws Exception {
    assertFalse(metaData().supportsSubqueriesInQuantifieds());
  }

  @Test
  void supportsCorrelatedSubqueriesIsTrue() throws Exception {
    assertTrue(metaData().supportsCorrelatedSubqueries());
  }

  @Test
  void supportsUnionIsTrue() throws Exception {
    assertTrue(metaData().supportsUnion());
  }

  @Test
  void supportsUnionAllIsTrue() throws Exception {
    assertTrue(metaData().supportsUnionAll());
  }

  // ---------- Capabilities: cursors, statements across commit/rollback ----------

  @Test
  void supportsOpenCursorsAcrossCommitIsFalse() throws Exception {
    assertFalse(metaData().supportsOpenCursorsAcrossCommit());
  }

  @Test
  void supportsOpenCursorsAcrossRollbackIsFalse() throws Exception {
    assertFalse(metaData().supportsOpenCursorsAcrossRollback());
  }

  @Test
  void supportsOpenStatementsAcrossCommitIsFalse() throws Exception {
    assertFalse(metaData().supportsOpenStatementsAcrossCommit());
  }

  @Test
  void supportsOpenStatementsAcrossRollbackIsFalse() throws Exception {
    assertFalse(metaData().supportsOpenStatementsAcrossRollback());
  }

  // ---------- Capabilities: transactions ----------

  @Test
  void getDefaultTransactionIsolationIsReadCommitted() throws Exception {
    assertEquals(
        Connection.TRANSACTION_READ_COMMITTED, metaData().getDefaultTransactionIsolation());
  }

  @Test
  void supportsTransactionsIsTrue() throws Exception {
    assertTrue(metaData().supportsTransactions());
  }

  @Test
  void supportsTransactionIsolationLevelNoneIsTrue() throws Exception {
    assertTrue(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_NONE));
  }

  @Test
  void supportsTransactionIsolationLevelReadCommittedIsTrue() throws Exception {
    assertTrue(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_READ_COMMITTED));
  }

  @Test
  void supportsTransactionIsolationLevelReadUncommittedIsFalse() throws Exception {
    assertFalse(
        metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_READ_UNCOMMITTED));
  }

  @Test
  void supportsTransactionIsolationLevelRepeatableReadIsFalse() throws Exception {
    assertFalse(
        metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_REPEATABLE_READ));
  }

  @Test
  void supportsTransactionIsolationLevelSerializableIsFalse() throws Exception {
    assertFalse(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_SERIALIZABLE));
  }

  @Test
  void supportsDataDefinitionAndDataManipulationTransactionsIsTrue() throws Exception {
    assertTrue(metaData().supportsDataDefinitionAndDataManipulationTransactions());
  }

  @Test
  void supportsDataManipulationTransactionsOnlyIsFalse() throws Exception {
    assertFalse(metaData().supportsDataManipulationTransactionsOnly());
  }

  @Test
  void dataDefinitionCausesTransactionCommitIsTrue() throws Exception {
    assertTrue(metaData().dataDefinitionCausesTransactionCommit());
  }

  @Test
  void dataDefinitionIgnoredInTransactionsIsFalse() throws Exception {
    assertFalse(metaData().dataDefinitionIgnoredInTransactions());
  }

  // ---------- Capabilities: result sets ----------

  @Test
  void supportsResultSetTypeForwardOnlyIsTrue() throws Exception {
    assertTrue(metaData().supportsResultSetType(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void supportsResultSetTypeScrollInsensitiveIsFalse() throws Exception {
    assertFalse(metaData().supportsResultSetType(ResultSet.TYPE_SCROLL_INSENSITIVE));
  }

  @Test
  void supportsResultSetTypeScrollSensitiveIsFalse() throws Exception {
    assertFalse(metaData().supportsResultSetType(ResultSet.TYPE_SCROLL_SENSITIVE));
  }

  @Test
  void supportsResultSetConcurrencyForwardReadOnlyIsTrue() throws Exception {
    assertTrue(
        metaData()
            .supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
  }

  @Test
  void supportsResultSetConcurrencyForwardUpdatableIsFalse() throws Exception {
    assertFalse(
        metaData()
            .supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE));
  }

  @Test
  void ownUpdatesAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().ownUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void ownDeletesAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().ownDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void ownInsertsAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().ownInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void othersUpdatesAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().othersUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void othersDeletesAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().othersDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void othersInsertsAreVisibleIsFalse() throws Exception {
    assertFalse(metaData().othersInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void updatesAreDetectedIsFalse() throws Exception {
    assertFalse(metaData().updatesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void deletesAreDetectedIsFalse() throws Exception {
    assertFalse(metaData().deletesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void insertsAreDetectedIsFalse() throws Exception {
    assertFalse(metaData().insertsAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void supportsBatchUpdatesIsTrue() throws Exception {
    assertTrue(metaData().supportsBatchUpdates());
  }

  @Test
  void supportsResultSetHoldabilityCloseAtCommitIsTrue() throws Exception {
    assertTrue(metaData().supportsResultSetHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT));
  }

  @Test
  void supportsResultSetHoldabilityHoldOverCommitIsFalse() throws Exception {
    assertFalse(metaData().supportsResultSetHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT));
  }

  @Test
  void getResultSetHoldabilityIsCloseCursorsAtCommit() throws Exception {
    assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, metaData().getResultSetHoldability());
  }

  // ---------- Capabilities: JDBC 3.0+ misc ----------

  @Test
  void supportsSavepointsIsFalse() throws Exception {
    assertFalse(metaData().supportsSavepoints());
  }

  @Test
  void supportsNamedParametersIsFalse() throws Exception {
    assertFalse(metaData().supportsNamedParameters());
  }

  @Test
  void supportsMultipleOpenResultsIsFalse() throws Exception {
    assertFalse(metaData().supportsMultipleOpenResults());
  }

  @Test
  void supportsGetGeneratedKeysIsFalse() throws Exception {
    assertFalse(metaData().supportsGetGeneratedKeys());
  }

  @Test
  void locatorsUpdateCopyIsFalse() throws Exception {
    assertFalse(metaData().locatorsUpdateCopy());
  }

  @Test
  void supportsStatementPoolingIsFalse() throws Exception {
    assertFalse(metaData().supportsStatementPooling());
  }

  @Test
  void supportsStoredFunctionsUsingCallSyntaxIsTrue() throws Exception {
    assertTrue(metaData().supportsStoredFunctionsUsingCallSyntax());
  }

  @Test
  void supportsRefCursorsIsFalse() throws Exception {
    assertFalse(metaData().supportsRefCursors());
  }

  @Test
  void getMaxLogicalLobSizeIsZero() throws Exception {
    assertEquals(0L, metaData().getMaxLogicalLobSize());
  }

  // ---------- Limits ----------

  @Test
  void getMaxCharLiteralLengthReturnsLimit() throws Exception {
    // VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT defaults to 16 MB; some accounts raise it.
    int len = metaData().getMaxCharLiteralLength();
    assertTrue(len >= 16_777_216, () -> "expected >= 16 MB, got " + len);
  }

  @Test
  void getMaxBinaryLiteralLengthReturnsLimit() throws Exception {
    // Hex-encoded binary literal: two chars per byte, so half the char limit.
    DatabaseMetaData md = metaData();
    assertEquals(md.getMaxCharLiteralLength() / 2, md.getMaxBinaryLiteralLength());
  }

  @Test
  void getMaxColumnNameLengthIs255() throws Exception {
    assertEquals(255, metaData().getMaxColumnNameLength());
  }

  @Test
  void getMaxColumnsInGroupByIsZero() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInGroupBy());
  }

  @Test
  void getMaxColumnsInIndexIsZero() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInIndex());
  }

  @Test
  void getMaxColumnsInOrderByIsZero() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInOrderBy());
  }

  @Test
  void getMaxColumnsInSelectIsZero() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInSelect());
  }

  @Test
  void getMaxColumnsInTableIsZero() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInTable());
  }

  @Test
  void getMaxConnectionsIsZero() throws Exception {
    assertEquals(0, metaData().getMaxConnections());
  }

  @Test
  void getMaxCursorNameLengthIsZero() throws Exception {
    assertEquals(0, metaData().getMaxCursorNameLength());
  }

  @Test
  void getMaxIndexLengthIsZero() throws Exception {
    assertEquals(0, metaData().getMaxIndexLength());
  }

  @Test
  void getMaxSchemaNameLengthIs255() throws Exception {
    assertEquals(255, metaData().getMaxSchemaNameLength());
  }

  @Test
  void getMaxProcedureNameLengthIsZero() throws Exception {
    assertEquals(0, metaData().getMaxProcedureNameLength());
  }

  @Test
  void getMaxCatalogNameLengthIs255() throws Exception {
    assertEquals(255, metaData().getMaxCatalogNameLength());
  }

  @Test
  void getMaxRowSizeIsZero() throws Exception {
    assertEquals(0, metaData().getMaxRowSize());
  }

  @Test
  void doesMaxRowSizeIncludeBlobsIsTrue() throws Exception {
    assertTrue(metaData().doesMaxRowSizeIncludeBlobs());
  }

  @Test
  void getMaxStatementLengthIsZero() throws Exception {
    assertEquals(0, metaData().getMaxStatementLength());
  }

  @Test
  void getMaxStatementsIsZero() throws Exception {
    assertEquals(0, metaData().getMaxStatements());
  }

  @Test
  void getMaxTableNameLengthIs255() throws Exception {
    assertEquals(255, metaData().getMaxTableNameLength());
  }

  @Test
  void getMaxTablesInSelectIsZero() throws Exception {
    assertEquals(0, metaData().getMaxTablesInSelect());
  }

  @Test
  void getMaxUserNameLengthIs255() throws Exception {
    assertEquals(255, metaData().getMaxUserNameLength());
  }

  // ---------- Plumbing ----------

  @Test
  void getConnectionReturnsUnderlyingConnection() throws Exception {
    Connection conn = getDefaultConnection();
    assertSame(conn, conn.getMetaData().getConnection());
  }

  // ---------- Unsupported features (must throw SQLFeatureNotSupportedException) ----------

  @Test
  void getRowIdLifetimeThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::getRowIdLifetime);
  }

  @Test
  void generatedKeyAlwaysReturnedThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::generatedKeyAlwaysReturned);
  }

  @Test
  void autoCommitFailureClosesAllResultSetsThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::autoCommitFailureClosesAllResultSets);
  }

  @Test
  void getBestRowIdentifierThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> md.getBestRowIdentifier(null, null, "T", DatabaseMetaData.bestRowSession, true));
  }

  @Test
  void getVersionColumnsThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getVersionColumns(null, null, "T"));
  }

  @Test
  void getSuperTypesThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, () -> md.getSuperTypes(null, null, "%"));
  }

  @Test
  void getSuperTablesThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, () -> md.getSuperTables(null, null, "%"));
  }

  @Test
  void getAttributesThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getAttributes(null, null, "%", "%"));
  }

  @Test
  void getPseudoColumnsThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getPseudoColumns(null, null, "%", "%"));
  }

  @Test
  void getClientInfoPropertiesThrowsUnsupported() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::getClientInfoProperties);
  }

  // ---------- Identity values from the connection ----------

  @Test
  void getURLReturnsJdbcUrl() throws Exception {
    String url = metaData().getURL();
    assertTrue(
        url != null && url.startsWith("jdbc:snowflake://"),
        () -> "getURL must return a jdbc:snowflake:// URL, got: " + url);

    // Both drivers should expose the connecting host in the URL. The legacy driver
    // derives it from SERVER_URL; universal-driver returns the URL the caller passed.
    Properties props = loadConnectionProperties();
    String host =
        props.getProperty("host", props.getProperty("account") + ".snowflakecomputing.com");
    assertTrue(
        url.toLowerCase().contains(host.toLowerCase()),
        () -> "getURL must contain the connecting host '" + host + "', got: " + url);
  }

  @Test
  void getUserNameReturnsConnectedUser() throws Exception {
    String expected = loadConnectionProperties().getProperty("user");
    assertEquals(expected, metaData().getUserName(), "getUserName must match the 'user' property");
  }

  @Test
  void getSQLKeywordsReturnsSnowflakeKeywordList() throws Exception {
    assertEquals(
        "ACCOUNT,ASOF,BIT,BYTEINT,CONNECTION,DATABASE,DATETIME,DATE_PART,FIXED,FOLLOWING,"
            + "GSCLUSTER,GSPACKAGE,IDENTIFIER,ILIKE,INCREMENT,ISSUE,LONG,MAP,MATCH_CONDITION,"
            + "MINUS,NUMBER,OBJECT,ORGANIZATION,QUALIFY,REFERENCE,REGEXP,RLIKE,SAMPLE,SCHEMA,"
            + "STRING,TEXT,TIMESTAMPLTZ,TIMESTAMPNTZ,TIMESTAMPTZ,TIMESTAMP_LTZ,TIMESTAMP_NTZ,"
            + "TIMESTAMP_TZ,TINYINT,TRANSIT,TRY_CAST,VARIANT,VECTOR,VIEW",
        metaData().getSQLKeywords());
  }

  @Test
  void getNumericFunctionsReturnsKnownList() throws Exception {
    assertEquals(
        "ABS,ACOS,ASIN,ATAN,ATAN2,CBRT,CEILING,COS,COT,DEGREES,EXP,FACTORIAL,"
            + "FLOOR,HAVERSINE,LN,LOG,MOD,PI,POWER,RADIANS,RAND,"
            + "ROUND,SIGN,SIN,SQRT,SQUARE,TAN,TRUNCATE",
        metaData().getNumericFunctions());
  }

  @Test
  void getStringFunctionsReturnsKnownList() throws Exception {
    assertEquals(
        "ASCII,BIT_LENGTH,CHAR,CONCAT,INSERT,LCASE,LEFT,LENGTH,LPAD,"
            + "LOCATE,LTRIM,OCTET_LENGTH,PARSE_IP,PARSE_URL,REPEAT,REVERSE,"
            + "REPLACE,RPAD,RTRIMMED_LENGTH,SPACE,SPLIT,SPLIT_PART,"
            + "SPLIT_TO_TABLE,STRTOK,STRTOK_TO_ARRAY,STRTOK_SPLIT_TO_TABLE,"
            + "TRANSLATE,TRIM,UNICODE,UUID_STRING,INITCAP,LOWER,UPPER,REGEXP,"
            + "REGEXP_COUNT,REGEXP_INSTR,REGEXP_LIKE,REGEXP_REPLACE,"
            + "REGEXP_SUBSTR,RLIKE,CHARINDEX,CONTAINS,EDITDISTANCE,ENDSWITH,"
            + "ILIKE,ILIKE ANY,LIKE,LIKE ALL,LIKE ANY,POSITION,REPLACE,RIGHT,"
            + "STARTSWITH,SUBSTRING,COMPRESS,DECOMPRESS_BINARY,DECOMPRESS_STRING,"
            + "BASE64_DECODE_BINARY,BASE64_DECODE_STRING,BASE64_ENCODE,"
            + "HEX_DECODE_BINARY,HEX_DECODE_STRING,HEX_ENCODE,"
            + "TRY_BASE64_DECODE_BINARY,TRY_BASE64_DECODE_STRING,"
            + "TRY_HEX_DECODE_BINARY,TRY_HEX_DECODE_STRING,MD_5,MD5_HEX,"
            + "MD5_BINARY,SHA1,SHA1_HEX,SHA2,SHA1_BINARY,SHA2_HEX,SHA2_BINARY,"
            + " HASH,HASH_AGG,COLLATE,COLLATION",
        metaData().getStringFunctions());
  }

  @Test
  void getSystemFunctionsReturnsKnownList() throws Exception {
    assertEquals("DATABASE,IFNULL,USER", metaData().getSystemFunctions());
  }

  @Test
  void getTimeDateFunctionsReturnsKnownList() throws Exception {
    assertEquals(
        "CURDATE,CURTIME,DAYNAME,DAYOFMONTH,DAYOFWEEK,DAYOFYEAR,HOUR,MINUTE,MONTH,"
            + "MONTHNAME,NOW,QUARTER,SECOND,TIMESTAMPADD,TIMESTAMPDIFF,WEEK,YEAR",
        metaData().getTimeDateFunctions());
  }

  // ---------- Disabled: ResultSet-returning methods that issue Snowflake queries ----------

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getCatalogsReturnsCurrentDatabases() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getSchemasReturnsAccessibleSchemas() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getSchemasWithCatalogAndPatternReturnsMatchingSchemas() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getTablesReturnsMatchingTables() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getTableTypesReturnsKnownTypes() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getColumnsReturnsTableColumns() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getColumnPrivilegesReturnsPrivileges() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getTablePrivilegesReturnsPrivileges() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getPrimaryKeysReturnsKeyColumns() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getImportedKeysReturnsForeignKeys() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getExportedKeysReturnsForeignKeys() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getCrossReferenceReturnsRelationships() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getTypeInfoReturnsSupportedTypes() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getIndexInfoReturnsIndexDescriptors() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getUDTsReturnsUserDefinedTypes() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getProceduresReturnsProcedures() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getProcedureColumnsReturnsProcedureColumns() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getFunctionsReturnsFunctions() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void getFunctionColumnsReturnsFunctionColumns() {}
}
