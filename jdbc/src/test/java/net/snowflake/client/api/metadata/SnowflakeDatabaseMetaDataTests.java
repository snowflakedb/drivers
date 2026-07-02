package net.snowflake.client.api.metadata;

import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import net.snowflake.jdbc.utils.TestParameters;
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
  void shouldReturnSnowflakeForDatabaseProductName() throws Exception {
    assertEquals("Snowflake", metaData().getDatabaseProductName());
  }

  @Test
  void shouldContainSnowflakeInDriverName() throws Exception {
    // Both drivers identify as a Snowflake driver: legacy returns "Snowflake",
    // universal-driver returns "Snowflake JDBC Driver". Substring is the
    // strictest assertion that still holds across both.
    assertTrue(metaData().getDriverName().contains("Snowflake"));
  }

  @Test
  void shouldReportConsistentVersionMetadata() throws Exception {
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
  void shouldReturnDoubleQuoteForIdentifierQuoteString() throws Exception {
    assertEquals("\"", metaData().getIdentifierQuoteString());
  }

  @Test
  void shouldReturnBackslashForSearchStringEscape() throws Exception {
    assertEquals("\\", metaData().getSearchStringEscape());
  }

  @Test
  void shouldReturnDollarForExtraNameCharacters() throws Exception {
    assertEquals("$", metaData().getExtraNameCharacters());
  }

  @Test
  void shouldReturnSchemaForSchemaTerm() throws Exception {
    assertEquals("schema", metaData().getSchemaTerm());
  }

  @Test
  void shouldReturnProcedureForProcedureTerm() throws Exception {
    assertEquals("procedure", metaData().getProcedureTerm());
  }

  @Test
  void shouldReturnDatabaseForCatalogTerm() throws Exception {
    assertEquals("database", metaData().getCatalogTerm());
  }

  @Test
  void shouldReturnDotForCatalogSeparator() throws Exception {
    assertEquals(".", metaData().getCatalogSeparator());
  }

  @Test
  void shouldReturnSqlStateSqlForSqlStateType() throws Exception {
    assertEquals(DatabaseMetaData.sqlStateSQL, metaData().getSQLStateType());
  }

  // ---------- Capabilities: procedures, tables, read-only ----------

  @Test
  void shouldReturnFalseForAllProceduresAreCallable() throws Exception {
    assertFalse(metaData().allProceduresAreCallable());
  }

  @Test
  void shouldReturnTrueForAllTablesAreSelectable() throws Exception {
    assertTrue(metaData().allTablesAreSelectable());
  }

  @Test
  void shouldReturnFalseForIsReadOnly() throws Exception {
    assertFalse(metaData().isReadOnly());
  }

  // ---------- Capabilities: null ordering ----------

  @Test
  void shouldReturnTrueForNullsAreSortedHigh() throws Exception {
    assertTrue(metaData().nullsAreSortedHigh());
  }

  @Test
  void shouldReturnFalseForNullsAreSortedLow() throws Exception {
    assertFalse(metaData().nullsAreSortedLow());
  }

  @Test
  void shouldReturnFalseForNullsAreSortedAtStart() throws Exception {
    assertFalse(metaData().nullsAreSortedAtStart());
  }

  @Test
  void shouldReturnFalseForNullsAreSortedAtEnd() throws Exception {
    assertFalse(metaData().nullsAreSortedAtEnd());
  }

  // ---------- Capabilities: file storage ----------

  @Test
  void shouldReturnFalseForUsesLocalFiles() throws Exception {
    assertFalse(metaData().usesLocalFiles());
  }

  @Test
  void shouldReturnFalseForUsesLocalFilePerTable() throws Exception {
    assertFalse(metaData().usesLocalFilePerTable());
  }

  // ---------- Capabilities: identifier case ----------

  @Test
  void shouldReturnFalseForSupportsMixedCaseIdentifiers() throws Exception {
    assertFalse(metaData().supportsMixedCaseIdentifiers());
  }

  @Test
  void shouldReturnTrueForStoresUpperCaseIdentifiers() throws Exception {
    assertTrue(metaData().storesUpperCaseIdentifiers());
  }

  @Test
  void shouldReturnFalseForStoresLowerCaseIdentifiers() throws Exception {
    assertFalse(metaData().storesLowerCaseIdentifiers());
  }

  @Test
  void shouldReturnFalseForStoresMixedCaseIdentifiers() throws Exception {
    assertFalse(metaData().storesMixedCaseIdentifiers());
  }

  @Test
  void shouldReturnTrueForSupportsMixedCaseQuotedIdentifiers() throws Exception {
    assertTrue(metaData().supportsMixedCaseQuotedIdentifiers());
  }

  @Test
  void shouldReturnFalseForStoresUpperCaseQuotedIdentifiers() throws Exception {
    assertFalse(metaData().storesUpperCaseQuotedIdentifiers());
  }

  @Test
  void shouldReturnFalseForStoresLowerCaseQuotedIdentifiers() throws Exception {
    assertFalse(metaData().storesLowerCaseQuotedIdentifiers());
  }

  @Test
  void shouldReturnTrueForStoresMixedCaseQuotedIdentifiers() throws Exception {
    assertTrue(metaData().storesMixedCaseQuotedIdentifiers());
  }

  // ---------- Capabilities: DDL/DML ----------

  @Test
  void shouldReturnTrueForSupportsAlterTableWithAddColumn() throws Exception {
    assertTrue(metaData().supportsAlterTableWithAddColumn());
  }

  @Test
  void shouldReturnTrueForSupportsAlterTableWithDropColumn() throws Exception {
    assertTrue(metaData().supportsAlterTableWithDropColumn());
  }

  @Test
  void shouldReturnTrueForSupportsColumnAliasing() throws Exception {
    assertTrue(metaData().supportsColumnAliasing());
  }

  @Test
  void shouldReturnTrueForNullPlusNonNullIsNull() throws Exception {
    assertTrue(metaData().nullPlusNonNullIsNull());
  }

  @Test
  void shouldReturnFalseForSupportsConvert() throws Exception {
    assertFalse(metaData().supportsConvert());
  }

  @Test
  void shouldReturnFalseForSupportsConvertWithTypes() throws Exception {
    assertFalse(metaData().supportsConvert(Types.INTEGER, Types.VARCHAR));
  }

  // ---------- Capabilities: correlation, ordering, grouping ----------

  @Test
  void shouldReturnTrueForSupportsTableCorrelationNames() throws Exception {
    assertTrue(metaData().supportsTableCorrelationNames());
  }

  @Test
  void shouldReturnFalseForSupportsDifferentTableCorrelationNames() throws Exception {
    assertFalse(metaData().supportsDifferentTableCorrelationNames());
  }

  @Test
  void shouldReturnTrueForSupportsExpressionsInOrderBy() throws Exception {
    assertTrue(metaData().supportsExpressionsInOrderBy());
  }

  @Test
  void shouldReturnTrueForSupportsOrderByUnrelated() throws Exception {
    assertTrue(metaData().supportsOrderByUnrelated());
  }

  @Test
  void shouldReturnTrueForSupportsGroupBy() throws Exception {
    assertTrue(metaData().supportsGroupBy());
  }

  @Test
  void shouldReturnFalseForSupportsGroupByUnrelated() throws Exception {
    assertFalse(metaData().supportsGroupByUnrelated());
  }

  @Test
  void shouldReturnTrueForSupportsGroupByBeyondSelect() throws Exception {
    assertTrue(metaData().supportsGroupByBeyondSelect());
  }

  @Test
  void shouldReturnFalseForSupportsLikeEscapeClause() throws Exception {
    assertFalse(metaData().supportsLikeEscapeClause());
  }

  // ---------- Capabilities: results, transactions, columns ----------

  @Test
  void shouldReturnFalseForSupportsMultipleResultSets() throws Exception {
    assertFalse(metaData().supportsMultipleResultSets());
  }

  @Test
  void shouldReturnTrueForSupportsMultipleTransactions() throws Exception {
    assertTrue(metaData().supportsMultipleTransactions());
  }

  @Test
  void shouldReturnTrueForSupportsNonNullableColumns() throws Exception {
    assertTrue(metaData().supportsNonNullableColumns());
  }

  // ---------- Capabilities: SQL grammar ----------

  @Test
  void shouldReturnFalseForSupportsMinimumSQLGrammar() throws Exception {
    assertFalse(metaData().supportsMinimumSQLGrammar());
  }

  @Test
  void shouldReturnFalseForSupportsCoreSQLGrammar() throws Exception {
    assertFalse(metaData().supportsCoreSQLGrammar());
  }

  @Test
  void shouldReturnFalseForSupportsExtendedSQLGrammar() throws Exception {
    assertFalse(metaData().supportsExtendedSQLGrammar());
  }

  @Test
  void shouldReturnTrueForSupportsANSI92EntryLevelSQL() throws Exception {
    assertTrue(metaData().supportsANSI92EntryLevelSQL());
  }

  @Test
  void shouldReturnFalseForSupportsANSI92IntermediateSQL() throws Exception {
    assertFalse(metaData().supportsANSI92IntermediateSQL());
  }

  @Test
  void shouldReturnFalseForSupportsANSI92FullSQL() throws Exception {
    assertFalse(metaData().supportsANSI92FullSQL());
  }

  @Test
  void shouldReturnFalseForSupportsIntegrityEnhancementFacility() throws Exception {
    assertFalse(metaData().supportsIntegrityEnhancementFacility());
  }

  // ---------- Capabilities: outer joins ----------

  @Test
  void shouldReturnTrueForSupportsOuterJoins() throws Exception {
    assertTrue(metaData().supportsOuterJoins());
  }

  @Test
  void shouldReturnTrueForSupportsFullOuterJoins() throws Exception {
    assertTrue(metaData().supportsFullOuterJoins());
  }

  @Test
  void shouldReturnTrueForSupportsLimitedOuterJoins() throws Exception {
    assertTrue(metaData().supportsLimitedOuterJoins());
  }

  // ---------- Capabilities: catalog/schema placement ----------

  @Test
  void shouldReturnTrueForIsCatalogAtStart() throws Exception {
    assertTrue(metaData().isCatalogAtStart());
  }

  @Test
  void shouldReturnTrueForSupportsSchemasInDataManipulation() throws Exception {
    assertTrue(metaData().supportsSchemasInDataManipulation());
  }

  @Test
  void shouldReturnFalseForSupportsSchemasInProcedureCalls() throws Exception {
    assertFalse(metaData().supportsSchemasInProcedureCalls());
  }

  @Test
  void shouldReturnTrueForSupportsSchemasInTableDefinitions() throws Exception {
    assertTrue(metaData().supportsSchemasInTableDefinitions());
  }

  @Test
  void shouldReturnFalseForSupportsSchemasInIndexDefinitions() throws Exception {
    assertFalse(metaData().supportsSchemasInIndexDefinitions());
  }

  @Test
  void shouldReturnFalseForSupportsSchemasInPrivilegeDefinitions() throws Exception {
    assertFalse(metaData().supportsSchemasInPrivilegeDefinitions());
  }

  @Test
  void shouldReturnTrueForSupportsCatalogsInDataManipulation() throws Exception {
    assertTrue(metaData().supportsCatalogsInDataManipulation());
  }

  @Test
  void shouldReturnFalseForSupportsCatalogsInProcedureCalls() throws Exception {
    assertFalse(metaData().supportsCatalogsInProcedureCalls());
  }

  @Test
  void shouldReturnTrueForSupportsCatalogsInTableDefinitions() throws Exception {
    assertTrue(metaData().supportsCatalogsInTableDefinitions());
  }

  @Test
  void shouldReturnFalseForSupportsCatalogsInIndexDefinitions() throws Exception {
    assertFalse(metaData().supportsCatalogsInIndexDefinitions());
  }

  @Test
  void shouldReturnFalseForSupportsCatalogsInPrivilegeDefinitions() throws Exception {
    assertFalse(metaData().supportsCatalogsInPrivilegeDefinitions());
  }

  // ---------- Capabilities: positioned/select-for-update/stored procedures ----------

  @Test
  void shouldReturnFalseForSupportsPositionedDelete() throws Exception {
    assertFalse(metaData().supportsPositionedDelete());
  }

  @Test
  void shouldReturnFalseForSupportsPositionedUpdate() throws Exception {
    assertFalse(metaData().supportsPositionedUpdate());
  }

  @Test
  void shouldReturnFalseForSupportsSelectForUpdate() throws Exception {
    assertFalse(metaData().supportsSelectForUpdate());
  }

  @Test
  void shouldReturnTrueForSupportsStoredProcedures() throws Exception {
    assertTrue(metaData().supportsStoredProcedures());
  }

  // ---------- Capabilities: subqueries, set operations ----------

  @Test
  void shouldReturnTrueForSupportsSubqueriesInComparisons() throws Exception {
    assertTrue(metaData().supportsSubqueriesInComparisons());
  }

  @Test
  void shouldReturnTrueForSupportsSubqueriesInExists() throws Exception {
    assertTrue(metaData().supportsSubqueriesInExists());
  }

  @Test
  void shouldReturnTrueForSupportsSubqueriesInIns() throws Exception {
    assertTrue(metaData().supportsSubqueriesInIns());
  }

  @Test
  void shouldReturnFalseForSupportsSubqueriesInQuantifieds() throws Exception {
    assertFalse(metaData().supportsSubqueriesInQuantifieds());
  }

  @Test
  void shouldReturnTrueForSupportsCorrelatedSubqueries() throws Exception {
    assertTrue(metaData().supportsCorrelatedSubqueries());
  }

  @Test
  void shouldReturnTrueForSupportsUnion() throws Exception {
    assertTrue(metaData().supportsUnion());
  }

  @Test
  void shouldReturnTrueForSupportsUnionAll() throws Exception {
    assertTrue(metaData().supportsUnionAll());
  }

  // ---------- Capabilities: cursors, statements across commit/rollback ----------

  @Test
  void shouldReturnFalseForSupportsOpenCursorsAcrossCommit() throws Exception {
    assertFalse(metaData().supportsOpenCursorsAcrossCommit());
  }

  @Test
  void shouldReturnFalseForSupportsOpenCursorsAcrossRollback() throws Exception {
    assertFalse(metaData().supportsOpenCursorsAcrossRollback());
  }

  @Test
  void shouldReturnFalseForSupportsOpenStatementsAcrossCommit() throws Exception {
    assertFalse(metaData().supportsOpenStatementsAcrossCommit());
  }

  @Test
  void shouldReturnFalseForSupportsOpenStatementsAcrossRollback() throws Exception {
    assertFalse(metaData().supportsOpenStatementsAcrossRollback());
  }

  // ---------- Capabilities: transactions ----------

  @Test
  void shouldReturnReadCommittedForDefaultTransactionIsolation() throws Exception {
    assertEquals(
        Connection.TRANSACTION_READ_COMMITTED, metaData().getDefaultTransactionIsolation());
  }

  @Test
  void shouldReturnTrueForSupportsTransactions() throws Exception {
    assertTrue(metaData().supportsTransactions());
  }

  @Test
  void shouldReturnTrueForSupportsTransactionIsolationLevelNone() throws Exception {
    assertTrue(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_NONE));
  }

  @Test
  void shouldReturnTrueForSupportsTransactionIsolationLevelReadCommitted() throws Exception {
    assertTrue(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_READ_COMMITTED));
  }

  @Test
  void shouldReturnFalseForSupportsTransactionIsolationLevelReadUncommitted() throws Exception {
    assertFalse(
        metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_READ_UNCOMMITTED));
  }

  @Test
  void shouldReturnFalseForSupportsTransactionIsolationLevelRepeatableRead() throws Exception {
    assertFalse(
        metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_REPEATABLE_READ));
  }

  @Test
  void shouldReturnFalseForSupportsTransactionIsolationLevelSerializable() throws Exception {
    assertFalse(metaData().supportsTransactionIsolationLevel(Connection.TRANSACTION_SERIALIZABLE));
  }

  @Test
  void shouldReturnTrueForSupportsDataDefinitionAndDataManipulationTransactions() throws Exception {
    assertTrue(metaData().supportsDataDefinitionAndDataManipulationTransactions());
  }

  @Test
  void shouldReturnFalseForSupportsDataManipulationTransactionsOnly() throws Exception {
    assertFalse(metaData().supportsDataManipulationTransactionsOnly());
  }

  @Test
  void shouldReturnTrueForDataDefinitionCausesTransactionCommit() throws Exception {
    assertTrue(metaData().dataDefinitionCausesTransactionCommit());
  }

  @Test
  void shouldReturnFalseForDataDefinitionIgnoredInTransactions() throws Exception {
    assertFalse(metaData().dataDefinitionIgnoredInTransactions());
  }

  // ---------- Capabilities: result sets ----------

  @Test
  void shouldReturnTrueForSupportsResultSetTypeForwardOnly() throws Exception {
    assertTrue(metaData().supportsResultSetType(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForSupportsResultSetTypeScrollInsensitive() throws Exception {
    assertFalse(metaData().supportsResultSetType(ResultSet.TYPE_SCROLL_INSENSITIVE));
  }

  @Test
  void shouldReturnFalseForSupportsResultSetTypeScrollSensitive() throws Exception {
    assertFalse(metaData().supportsResultSetType(ResultSet.TYPE_SCROLL_SENSITIVE));
  }

  @Test
  void shouldReturnTrueForSupportsResultSetConcurrencyForwardReadOnly() throws Exception {
    assertTrue(
        metaData()
            .supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
  }

  @Test
  void shouldReturnFalseForSupportsResultSetConcurrencyForwardUpdatable() throws Exception {
    assertFalse(
        metaData()
            .supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE));
  }

  @Test
  void shouldReturnFalseForOwnUpdatesAreVisible() throws Exception {
    assertFalse(metaData().ownUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForOwnDeletesAreVisible() throws Exception {
    assertFalse(metaData().ownDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForOwnInsertsAreVisible() throws Exception {
    assertFalse(metaData().ownInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForOthersUpdatesAreVisible() throws Exception {
    assertFalse(metaData().othersUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForOthersDeletesAreVisible() throws Exception {
    assertFalse(metaData().othersDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForOthersInsertsAreVisible() throws Exception {
    assertFalse(metaData().othersInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForUpdatesAreDetected() throws Exception {
    assertFalse(metaData().updatesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForDeletesAreDetected() throws Exception {
    assertFalse(metaData().deletesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnFalseForInsertsAreDetected() throws Exception {
    assertFalse(metaData().insertsAreDetected(ResultSet.TYPE_FORWARD_ONLY));
  }

  @Test
  void shouldReturnTrueForSupportsBatchUpdates() throws Exception {
    assertTrue(metaData().supportsBatchUpdates());
  }

  @Test
  void shouldReturnTrueForSupportsResultSetHoldabilityCloseAtCommit() throws Exception {
    assertTrue(metaData().supportsResultSetHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT));
  }

  @Test
  void shouldReturnFalseForSupportsResultSetHoldabilityHoldOverCommit() throws Exception {
    assertFalse(metaData().supportsResultSetHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT));
  }

  @Test
  void shouldReturnCloseCursorsAtCommitForResultSetHoldability() throws Exception {
    assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, metaData().getResultSetHoldability());
  }

  // ---------- Capabilities: JDBC 3.0+ misc ----------

  @Test
  void shouldReturnFalseForSupportsSavepoints() throws Exception {
    assertFalse(metaData().supportsSavepoints());
  }

  @Test
  void shouldReturnFalseForSupportsNamedParameters() throws Exception {
    assertFalse(metaData().supportsNamedParameters());
  }

  @Test
  void shouldReturnFalseForSupportsMultipleOpenResults() throws Exception {
    assertFalse(metaData().supportsMultipleOpenResults());
  }

  @Test
  void shouldReturnFalseForSupportsGetGeneratedKeys() throws Exception {
    assertFalse(metaData().supportsGetGeneratedKeys());
  }

  @Test
  void shouldReturnFalseForLocatorsUpdateCopy() throws Exception {
    assertFalse(metaData().locatorsUpdateCopy());
  }

  @Test
  void shouldReturnFalseForSupportsStatementPooling() throws Exception {
    assertFalse(metaData().supportsStatementPooling());
  }

  @Test
  void shouldReturnTrueForSupportsStoredFunctionsUsingCallSyntax() throws Exception {
    assertTrue(metaData().supportsStoredFunctionsUsingCallSyntax());
  }

  @Test
  void shouldReturnFalseForSupportsRefCursors() throws Exception {
    assertFalse(metaData().supportsRefCursors());
  }

  @Test
  void shouldReturnZeroForMaxLogicalLobSize() throws Exception {
    assertEquals(0L, metaData().getMaxLogicalLobSize());
  }

  // ---------- Limits ----------

  @Test
  void shouldReturnLimitForMaxCharLiteralLength() throws Exception {
    // VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT defaults to 16 MB; some accounts raise it.
    int len = metaData().getMaxCharLiteralLength();
    assertTrue(len >= 16_777_216, () -> "expected >= 16 MB, got " + len);
  }

  @Test
  void shouldReturnLimitForMaxBinaryLiteralLength() throws Exception {
    // Hex-encoded binary literal: two chars per byte, so half the char limit.
    DatabaseMetaData md = metaData();
    assertEquals(md.getMaxCharLiteralLength() / 2, md.getMaxBinaryLiteralLength());
  }

  @Test
  void shouldReturn255ForMaxColumnNameLength() throws Exception {
    assertEquals(255, metaData().getMaxColumnNameLength());
  }

  @Test
  void shouldReturnZeroForMaxColumnsInGroupBy() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInGroupBy());
  }

  @Test
  void shouldReturnZeroForMaxColumnsInIndex() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInIndex());
  }

  @Test
  void shouldReturnZeroForMaxColumnsInOrderBy() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInOrderBy());
  }

  @Test
  void shouldReturnZeroForMaxColumnsInSelect() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInSelect());
  }

  @Test
  void shouldReturnZeroForMaxColumnsInTable() throws Exception {
    assertEquals(0, metaData().getMaxColumnsInTable());
  }

  @Test
  void shouldReturnZeroForMaxConnections() throws Exception {
    assertEquals(0, metaData().getMaxConnections());
  }

  @Test
  void shouldReturnZeroForMaxCursorNameLength() throws Exception {
    assertEquals(0, metaData().getMaxCursorNameLength());
  }

  @Test
  void shouldReturnZeroForMaxIndexLength() throws Exception {
    assertEquals(0, metaData().getMaxIndexLength());
  }

  @Test
  void shouldReturn255ForMaxSchemaNameLength() throws Exception {
    assertEquals(255, metaData().getMaxSchemaNameLength());
  }

  @Test
  void shouldReturnZeroForMaxProcedureNameLength() throws Exception {
    assertEquals(0, metaData().getMaxProcedureNameLength());
  }

  @Test
  void shouldReturn255ForMaxCatalogNameLength() throws Exception {
    assertEquals(255, metaData().getMaxCatalogNameLength());
  }

  @Test
  void shouldReturnZeroForMaxRowSize() throws Exception {
    assertEquals(0, metaData().getMaxRowSize());
  }

  @Test
  void shouldReturnTrueForDoesMaxRowSizeIncludeBlobs() throws Exception {
    assertTrue(metaData().doesMaxRowSizeIncludeBlobs());
  }

  @Test
  void shouldReturnZeroForMaxStatementLength() throws Exception {
    assertEquals(0, metaData().getMaxStatementLength());
  }

  @Test
  void shouldReturnZeroForMaxStatements() throws Exception {
    assertEquals(0, metaData().getMaxStatements());
  }

  @Test
  void shouldReturn255ForMaxTableNameLength() throws Exception {
    assertEquals(255, metaData().getMaxTableNameLength());
  }

  @Test
  void shouldReturnZeroForMaxTablesInSelect() throws Exception {
    assertEquals(0, metaData().getMaxTablesInSelect());
  }

  @Test
  void shouldReturn255ForMaxUserNameLength() throws Exception {
    assertEquals(255, metaData().getMaxUserNameLength());
  }

  // ---------- Plumbing ----------

  @Test
  void shouldReturnUnderlyingConnectionForConnection() throws Exception {
    Connection conn = getDefaultConnection();
    assertSame(conn, conn.getMetaData().getConnection());
  }

  // ---------- Unsupported features (must throw SQLFeatureNotSupportedException) ----------

  @Test
  void shouldThrowUnsupportedForGetRowIdLifetime() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::getRowIdLifetime);
  }

  @Test
  void shouldThrowUnsupportedForGeneratedKeyAlwaysReturned() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::generatedKeyAlwaysReturned);
  }

  @Test
  void shouldThrowUnsupportedForAutoCommitFailureClosesAllResultSets() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::autoCommitFailureClosesAllResultSets);
  }

  @Test
  void shouldThrowUnsupportedForGetBestRowIdentifier() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> md.getBestRowIdentifier(null, null, "T", DatabaseMetaData.bestRowSession, true));
  }

  @Test
  void shouldThrowUnsupportedForGetVersionColumns() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getVersionColumns(null, null, "T"));
  }

  @Test
  void shouldThrowUnsupportedForGetSuperTypes() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, () -> md.getSuperTypes(null, null, "%"));
  }

  @Test
  void shouldThrowUnsupportedForGetSuperTables() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, () -> md.getSuperTables(null, null, "%"));
  }

  @Test
  void shouldThrowUnsupportedForGetAttributes() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getAttributes(null, null, "%", "%"));
  }

  @Test
  void shouldThrowUnsupportedForGetPseudoColumns() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(
        SQLFeatureNotSupportedException.class, () -> md.getPseudoColumns(null, null, "%", "%"));
  }

  @Test
  void shouldThrowUnsupportedForGetClientInfoProperties() throws Exception {
    DatabaseMetaData md = metaData();
    assertThrows(SQLFeatureNotSupportedException.class, md::getClientInfoProperties);
  }

  // ---------- Identity values from the connection ----------

  @Test
  void shouldReturnJdbcUrlForURL() throws Exception {
    String url = metaData().getURL();
    assertTrue(
        url != null && url.startsWith("jdbc:snowflake://"),
        () -> "getURL must return a jdbc:snowflake:// URL, got: " + url);

    // Both drivers should expose the connecting host in the URL. The legacy driver
    // derives it from SERVER_URL; universal-driver returns the URL the caller passed.
    // Snowflake normalizes underscores to hyphens in DNS hostnames (e.g.
    // sfengineering-drivers_aws_us_east_2 → sfengineering-drivers-aws-us-east-2), so
    // normalize both sides before comparing.
    Properties props = loadDefaultConnectionProperties();
    String host =
        props.getProperty("host", props.getProperty("account") + ".snowflakecomputing.com");
    assertTrue(
        url.toLowerCase().replace('_', '-').contains(host.toLowerCase().replace('_', '-')),
        () -> "getURL must contain the connecting host '" + host + "', got: " + url);
  }

  @Test
  void shouldReturnConnectedUserForUserName() throws Exception {
    String expected = TestParameters.get("SNOWFLAKE_TEST_USER");
    assertEquals(expected, metaData().getUserName(), "getUserName must match the 'user' property");
  }

  @Test
  void shouldReturnSnowflakeKeywordListForSQLKeywords() throws Exception {
    assertEquals(
        "ACCOUNT,ASOF,BIT,BYTEINT,CONNECTION,DATABASE,DATETIME,DATE_PART,FIXED,FOLLOWING,"
            + "GSCLUSTER,GSPACKAGE,IDENTIFIER,ILIKE,INCREMENT,ISSUE,LONG,MAP,MATCH_CONDITION,"
            + "MINUS,NUMBER,OBJECT,ORGANIZATION,QUALIFY,REFERENCE,REGEXP,RLIKE,SAMPLE,SCHEMA,"
            + "STRING,TEXT,TIMESTAMPLTZ,TIMESTAMPNTZ,TIMESTAMPTZ,TIMESTAMP_LTZ,TIMESTAMP_NTZ,"
            + "TIMESTAMP_TZ,TINYINT,TRANSIT,TRY_CAST,VARIANT,VECTOR,VIEW",
        metaData().getSQLKeywords());
  }

  @Test
  void shouldReturnKnownListForNumericFunctions() throws Exception {
    assertEquals(
        "ABS,ACOS,ASIN,ATAN,ATAN2,CBRT,CEILING,COS,COT,DEGREES,EXP,FACTORIAL,"
            + "FLOOR,HAVERSINE,LN,LOG,MOD,PI,POWER,RADIANS,RAND,"
            + "ROUND,SIGN,SIN,SQRT,SQUARE,TAN,TRUNCATE",
        metaData().getNumericFunctions());
  }

  @Test
  void shouldReturnKnownListForStringFunctions() throws Exception {
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
  void shouldReturnKnownListForSystemFunctions() throws Exception {
    assertEquals("DATABASE,IFNULL,USER", metaData().getSystemFunctions());
  }

  @Test
  void shouldReturnKnownListForTimeDateFunctions() throws Exception {
    assertEquals(
        "CURDATE,CURTIME,DAYNAME,DAYOFMONTH,DAYOFWEEK,DAYOFYEAR,HOUR,MINUTE,MONTH,"
            + "MONTHNAME,NOW,QUARTER,SECOND,TIMESTAMPADD,TIMESTAMPDIFF,WEEK,YEAR",
        metaData().getTimeDateFunctions());
  }

  // ---------- ResultSet-returning methods that issue Snowflake queries ----------

  @Test
  void shouldReturnCurrentDatabasesForCatalogs() throws Exception {
    Connection conn = getDefaultConnection();
    DatabaseMetaData metaData = conn.getMetaData();
    String currentDatabase;
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_DATABASE()")) {
      assertTrue(rs.next());
      currentDatabase = rs.getString(1);
    }

    try (ResultSet resultSet = metaData.getCatalogs()) {
      ResultSetMetaData rsMeta = resultSet.getMetaData();
      assertEquals(1, rsMeta.getColumnCount());
      assertMetadataColumn(rsMeta, 1, "TABLE_CAT");

      Set<String> databases = new HashSet<>();
      while (resultSet.next()) {
        databases.add(resultSet.getString(1));
      }
      assertFalse(databases.isEmpty());
      assertTrue(databases.contains(currentDatabase));
    }
  }

  @Test
  void shouldReturnAccessibleSchemasForSchemas() throws Exception {
    Connection conn = getDefaultConnection();
    DatabaseMetaData metaData = conn.getMetaData();
    String currentDatabase;
    String currentSchema;
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_DATABASE(), CURRENT_SCHEMA()")) {
      assertTrue(rs.next());
      currentDatabase = rs.getString(1);
      currentSchema = rs.getString(2);
    }

    try (ResultSet resultSet = metaData.getSchemas()) {
      ResultSetMetaData rsMeta = resultSet.getMetaData();
      assertEquals(2, rsMeta.getColumnCount());
      assertMetadataColumn(rsMeta, 1, "TABLE_SCHEM");
      assertMetadataColumn(rsMeta, 2, "TABLE_CATALOG");

      Map<String, List<String>> catalogToSchema = new HashMap<>();
      while (resultSet.next()) {
        String schema = resultSet.getString("TABLE_SCHEM");
        String catalog = resultSet.getString("TABLE_CATALOG");
        catalogToSchema.putIfAbsent(catalog, new ArrayList<>());
        catalogToSchema.get(catalog).add(schema);
      }
      assertFalse(catalogToSchema.isEmpty());
      assertTrue(catalogToSchema.containsKey(currentDatabase));
      assertTrue(catalogToSchema.get(currentDatabase).contains(currentSchema));
    }
  }

  @Test
  void shouldReturnMatchingSchemasForSchemasWithCatalogAndPattern() throws Exception {
    Connection conn = getDefaultConnection();
    DatabaseMetaData metaData = conn.getMetaData();
    String currentDatabase;
    String currentSchema;
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_DATABASE(), CURRENT_SCHEMA()")) {
      assertTrue(rs.next());
      currentDatabase = rs.getString(1);
      currentSchema = rs.getString(2);
    }

    try (ResultSet resultSet = metaData.getSchemas(currentDatabase, currentSchema)) {
      assertTrue(resultSet.next());
      assertEquals(currentSchema, resultSet.getString("TABLE_SCHEM"));
      assertEquals(currentDatabase, resultSet.getString("TABLE_CATALOG"));
      assertFalse(resultSet.next());
    }

    try (ResultSet resultSet = metaData.getSchemas(currentDatabase, "%")) {
      Set<String> schemas = new HashSet<>();
      while (resultSet.next()) {
        assertEquals(currentDatabase, resultSet.getString("TABLE_CATALOG"));
        schemas.add(resultSet.getString("TABLE_SCHEM"));
      }
      assertTrue(schemas.contains(currentSchema));
    }

    try (ResultSet resultSet = metaData.getSchemas("NONEXISTENT_DB_XYZ", "%")) {
      assertFalse(resultSet.next());
    }
  }

  @Test
  void shouldReturnMatchingTablesFromGetTables() throws Exception {
    try (Connection conn = openConnection()) {
      DatabaseMetaData metaData = conn.getMetaData();
      String currentDatabase;
      String currentSchema;
      try (Statement s = conn.createStatement();
          ResultSet rs = s.executeQuery("SELECT CURRENT_DATABASE(), CURRENT_SCHEMA()")) {
        assertTrue(rs.next());
        currentDatabase = rs.getString(1);
        currentSchema = rs.getString(2);
      }

      String suffix = UUID.randomUUID().toString().replace("-", "").toUpperCase();
      String targetTable = "T0_" + suffix;
      String targetView = "V0_" + suffix;
      try (Statement stmt = conn.createStatement()) {
        stmt.execute("create or replace table " + targetTable + "(C1 int)");
        stmt.execute("create or replace view " + targetView + " as select 1 as C");
        try {
          // column shape
          try (ResultSet resultSet =
              metaData.getTables(currentDatabase, currentSchema, "%", null)) {
            ResultSetMetaData rsMeta = resultSet.getMetaData();
            assertEquals(10, rsMeta.getColumnCount());
            assertEquals("TABLE_CAT", rsMeta.getColumnName(1));
            assertEquals("TABLE_SCHEM", rsMeta.getColumnName(2));
            assertEquals("TABLE_NAME", rsMeta.getColumnName(3));
            assertEquals("TABLE_TYPE", rsMeta.getColumnName(4));
            assertEquals("REMARKS", rsMeta.getColumnName(5));
          }

          // TABLE type filter
          try (ResultSet resultSet =
              metaData.getTables(currentDatabase, currentSchema, "%", new String[] {"TABLE"})) {
            Set<String> tables = new HashSet<>();
            while (resultSet.next()) {
              assertEquals(currentDatabase, resultSet.getString("TABLE_CAT"));
              assertEquals(currentSchema, resultSet.getString("TABLE_SCHEM"));
              assertEquals("TABLE", resultSet.getString("TABLE_TYPE"));
              tables.add(resultSet.getString("TABLE_NAME"));
            }
            assertTrue(tables.contains(targetTable));
            assertFalse(tables.contains(targetView));
          }

          // VIEW type filter
          try (ResultSet resultSet =
              metaData.getTables(currentDatabase, currentSchema, "%", new String[] {"VIEW"})) {
            Set<String> views = new HashSet<>();
            while (resultSet.next()) {
              assertEquals("VIEW", resultSet.getString("TABLE_TYPE"));
              views.add(resultSet.getString("TABLE_NAME"));
            }
            assertTrue(views.contains(targetView));
            assertFalse(views.contains(targetTable));
          }

          // exact name match
          try (ResultSet resultSet =
              metaData.getTables(
                  currentDatabase, currentSchema, targetTable, new String[] {"TABLE"})) {
            assertTrue(resultSet.next());
            assertEquals(targetTable, resultSet.getString("TABLE_NAME"));
            assertFalse(resultSet.next());
          }

          // invalid type returns empty
          try (ResultSet resultSet =
              metaData.getTables(
                  currentDatabase, currentSchema, "%", new String[] {"INVALID_TYPE"})) {
            assertFalse(resultSet.next());
          }

          // non-existent db returns empty
          try (ResultSet resultSet =
              metaData.getTables("DB_NOT_EXIST", "SCHEMA\\_NOT\\_EXIST", "%", null)) {
            assertFalse(resultSet.next());
          }
        } finally {
          stmt.execute("drop table if exists " + targetTable);
          stmt.execute("drop view if exists " + targetView);
        }
      }
    }
  }

  @Test
  void shouldReturnKnownTypesForTableTypes() throws Exception {
    try (ResultSet rs = metaData().getTableTypes()) {
      ResultSetMetaData rsMeta = rs.getMetaData();
      assertEquals(1, rsMeta.getColumnCount());
      assertEquals("TABLE_TYPE", rsMeta.getColumnName(1));

      Set<String> types = new HashSet<>();
      while (rs.next()) {
        types.add(rs.getString("TABLE_TYPE"));
      }
      assertTrue(types.contains("TABLE"));
      assertTrue(types.contains("VIEW"));
    }
  }

  @Test
  void shouldReturnTableColumnsForColumns() throws Exception {
    try (Connection conn = openConnection()) {
      DatabaseMetaData metaData = conn.getMetaData();
      String currentDatabase;
      String currentSchema;
      try (ResultSet rs =
          conn.createStatement().executeQuery("SELECT CURRENT_DATABASE(), CURRENT_SCHEMA()")) {
        assertTrue(rs.next());
        currentDatabase = rs.getString(1);
        currentSchema = rs.getString(2);
      }

      String suffix = UUID.randomUUID().toString().replace("-", "").toUpperCase();
      String targetTable = "T0_" + suffix;
      try (Statement stmt = conn.createStatement()) {
        stmt.execute(
            "create or replace table "
                + targetTable
                + "(C1 int, C2 varchar(100), C7 date not null)");
        try {
          try (ResultSet resultSet =
              metaData.getColumns(currentDatabase, currentSchema, targetTable, "%")) {
            ResultSetMetaData rsMeta = resultSet.getMetaData();
            assertEquals(24, rsMeta.getColumnCount());
            assertEquals("TABLE_CAT", rsMeta.getColumnName(1));
            assertEquals("TABLE_SCHEM", rsMeta.getColumnName(2));
            assertEquals("TABLE_NAME", rsMeta.getColumnName(3));
            assertEquals("COLUMN_NAME", rsMeta.getColumnName(4));
            assertEquals("DATA_TYPE", rsMeta.getColumnName(5));
            assertEquals("ORDINAL_POSITION", rsMeta.getColumnName(17));
            assertEquals("IS_NULLABLE", rsMeta.getColumnName(18));

            assertTrue(resultSet.next());
            assertEquals(currentDatabase, resultSet.getString("TABLE_CAT"));
            assertEquals(currentSchema, resultSet.getString("TABLE_SCHEM"));
            assertEquals(targetTable, resultSet.getString("TABLE_NAME"));
            assertEquals("C1", resultSet.getString("COLUMN_NAME"));
            assertEquals(Types.BIGINT, resultSet.getInt("DATA_TYPE"));
            assertEquals("NUMBER", resultSet.getString("TYPE_NAME"));
            assertEquals(38, resultSet.getInt("COLUMN_SIZE"));
            assertEquals(ResultSetMetaData.columnNullable, resultSet.getInt("NULLABLE"));
            assertEquals(1, resultSet.getInt("ORDINAL_POSITION"));
            assertEquals("YES", resultSet.getString("IS_NULLABLE"));

            assertTrue(resultSet.next());
            assertEquals("C2", resultSet.getString("COLUMN_NAME"));
            assertEquals(Types.VARCHAR, resultSet.getInt("DATA_TYPE"));
            assertEquals(100, resultSet.getInt("COLUMN_SIZE"));
            assertEquals(100, resultSet.getInt("CHAR_OCTET_LENGTH"));
            assertEquals(2, resultSet.getInt("ORDINAL_POSITION"));

            assertTrue(resultSet.next());
            assertEquals("C7", resultSet.getString("COLUMN_NAME"));
            assertEquals(Types.DATE, resultSet.getInt("DATA_TYPE"));
            assertEquals(ResultSetMetaData.columnNoNulls, resultSet.getInt("NULLABLE"));
            assertEquals("NO", resultSet.getString("IS_NULLABLE"));
            assertEquals(3, resultSet.getInt("ORDINAL_POSITION"));
            assertFalse(resultSet.next());
          }

          try (ResultSet resultSet =
              metaData.getColumns(currentDatabase, currentSchema, targetTable, "C2")) {
            assertTrue(resultSet.next());
            assertEquals("C2", resultSet.getString("COLUMN_NAME"));
            assertFalse(resultSet.next());
          }

          try (ResultSet resultSet =
              metaData.getColumns("DB_NOT_EXIST", "SCHEMA\\_NOT\\_EXIST", "%", "%")) {
            assertFalse(resultSet.next());
          }
          try (ResultSet resultSet =
              metaData.getColumns(currentDatabase, "SCHEMA\\_NOT\\_EXIST", "%", "%")) {
            assertFalse(resultSet.next());
          }
          try (ResultSet resultSet =
              metaData.getColumns(currentDatabase, currentSchema, "TBL\\_NOT\\_EXIST", "%")) {
            assertFalse(resultSet.next());
          }
        } finally {
          stmt.execute("drop table if exists " + targetTable);
        }
      }
    }
  }

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnPrivilegesForColumnPrivileges() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnPrivilegesForTablePrivileges() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnKeyColumnsForPrimaryKeys() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnForeignKeysForImportedKeys() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnForeignKeysForExportedKeys() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnRelationshipsForCrossReference() {}

  @Test
  void shouldReturnSupportedTypesForTypeInfo() throws Exception {
    try (ResultSet rs = metaData().getTypeInfo()) {
      ResultSetMetaData rsMeta = rs.getMetaData();
      assertEquals(18, rsMeta.getColumnCount());
      assertEquals("TYPE_NAME", rsMeta.getColumnName(1));
      assertEquals("DATA_TYPE", rsMeta.getColumnName(2));
      assertEquals("PRECISION", rsMeta.getColumnName(3));
      assertEquals("LITERAL_PREFIX", rsMeta.getColumnName(4));
      assertEquals("LITERAL_SUFFIX", rsMeta.getColumnName(5));
      assertEquals("CREATE_PARAMS", rsMeta.getColumnName(6));
      assertEquals("NULLABLE", rsMeta.getColumnName(7));
      assertEquals("CASE_SENSITIVE", rsMeta.getColumnName(8));
      assertEquals("SEARCHABLE", rsMeta.getColumnName(9));
      assertEquals("UNSIGNED_ATTRIBUTE", rsMeta.getColumnName(10));
      assertEquals("FIXED_PREC_SCALE", rsMeta.getColumnName(11));
      assertEquals("AUTO_INCREMENT", rsMeta.getColumnName(12));
      assertEquals("LOCAL_TYPE_NAME", rsMeta.getColumnName(13));
      assertEquals("MINIMUM_SCALE", rsMeta.getColumnName(14));
      assertEquals("MAXIMUM_SCALE", rsMeta.getColumnName(15));
      assertEquals("SQL_DATA_TYPE", rsMeta.getColumnName(16));
      assertEquals("SQL_DATETIME_SUB", rsMeta.getColumnName(17));
      assertEquals("NUM_PREC_RADIX", rsMeta.getColumnName(18));

      // NUMBER
      assertTrue(rs.next());
      assertEquals("NUMBER", rs.getString("TYPE_NAME"));
      assertEquals(Types.DECIMAL, rs.getInt("DATA_TYPE"));
      assertEquals(38, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(0, rs.getShort("MINIMUM_SCALE"));
      assertEquals(37, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // INTEGER
      assertTrue(rs.next());
      assertEquals("INTEGER", rs.getString("TYPE_NAME"));
      assertEquals(Types.INTEGER, rs.getInt("DATA_TYPE"));
      assertEquals(38, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(0, rs.getShort("MINIMUM_SCALE"));
      assertEquals(0, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // DOUBLE
      assertTrue(rs.next());
      assertEquals("DOUBLE", rs.getString("TYPE_NAME"));
      assertEquals(Types.DOUBLE, rs.getInt("DATA_TYPE"));
      assertEquals(38, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(0, rs.getShort("MINIMUM_SCALE"));
      assertEquals(37, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // VARCHAR
      assertTrue(rs.next());
      assertEquals("VARCHAR", rs.getString("TYPE_NAME"));
      assertEquals(Types.VARCHAR, rs.getInt("DATA_TYPE"));
      assertEquals(-1, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(-1, rs.getShort("MINIMUM_SCALE"));
      assertEquals(-1, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // DATE
      assertTrue(rs.next());
      assertEquals("DATE", rs.getString("TYPE_NAME"));
      assertEquals(Types.DATE, rs.getInt("DATA_TYPE"));
      assertEquals(-1, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(-1, rs.getShort("MINIMUM_SCALE"));
      assertEquals(-1, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // TIME
      assertTrue(rs.next());
      assertEquals("TIME", rs.getString("TYPE_NAME"));
      assertEquals(Types.TIME, rs.getInt("DATA_TYPE"));
      assertEquals(-1, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(-1, rs.getShort("MINIMUM_SCALE"));
      assertEquals(-1, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // TIMESTAMP
      assertTrue(rs.next());
      assertEquals("TIMESTAMP", rs.getString("TYPE_NAME"));
      assertEquals(Types.TIMESTAMP, rs.getInt("DATA_TYPE"));
      assertEquals(-1, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(-1, rs.getShort("MINIMUM_SCALE"));
      assertEquals(-1, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      // BOOLEAN
      assertTrue(rs.next());
      assertEquals("BOOLEAN", rs.getString("TYPE_NAME"));
      assertEquals(Types.BOOLEAN, rs.getInt("DATA_TYPE"));
      assertEquals(-1, rs.getInt("PRECISION"));
      assertNull(rs.getString("LITERAL_PREFIX"));
      assertNull(rs.getString("LITERAL_SUFFIX"));
      assertNull(rs.getString("CREATE_PARAMS"));
      assertEquals(DatabaseMetaData.typeNullable, rs.getShort("NULLABLE"));
      assertFalse(rs.getBoolean("CASE_SENSITIVE"));
      assertEquals(DatabaseMetaData.typeSearchable, rs.getShort("SEARCHABLE"));
      assertFalse(rs.getBoolean("UNSIGNED_ATTRIBUTE"));
      assertTrue(rs.getBoolean("FIXED_PREC_SCALE"));
      assertTrue(rs.getBoolean("AUTO_INCREMENT"));
      assertNull(rs.getString("LOCAL_TYPE_NAME"));
      assertEquals(-1, rs.getShort("MINIMUM_SCALE"));
      assertEquals(-1, rs.getShort("MAXIMUM_SCALE"));
      assertEquals(-1, rs.getInt("SQL_DATA_TYPE"));
      assertEquals(-1, rs.getInt("SQL_DATETIME_SUB"));
      assertEquals(-1, rs.getInt("NUM_PREC_RADIX"));

      assertFalse(rs.next());
    }
  }

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnIndexDescriptorsForIndexInfo() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnUserDefinedTypesForUDTs() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnProceduresForProcedures() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnProcedureColumnsForProcedureColumns() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnFunctionsForFunctions() {}

  @Test
  @Disabled("requires query against Snowflake; happy-path test pending")
  void shouldReturnFunctionColumnsForFunctionColumns() {}

  private static void assertMetadataColumn(ResultSetMetaData rsMeta, int col, String name)
      throws Exception {
    assertEquals(name, rsMeta.getColumnName(col));
    assertEquals(name, rsMeta.getColumnLabel(col));
    assertEquals(Types.VARCHAR, rsMeta.getColumnType(col));
    assertEquals("TEXT", rsMeta.getColumnTypeName(col));
    assertEquals("", rsMeta.getCatalogName(col));
    assertEquals("", rsMeta.getSchemaName(col));
    assertEquals("T", rsMeta.getTableName(col));
    assertEquals(25, rsMeta.getColumnDisplaySize(col));
    assertEquals(9, rsMeta.getPrecision(col));
    assertEquals(9, rsMeta.getScale(col));
    assertEquals(ResultSetMetaData.columnNullableUnknown, rsMeta.isNullable(col));
    assertFalse(rsMeta.isAutoIncrement(col));
    assertFalse(rsMeta.isSigned(col));
    assertTrue(rsMeta.isSearchable(col));
    assertTrue(rsMeta.isReadOnly(col));
    assertFalse(rsMeta.isWritable(col));
  }
}
