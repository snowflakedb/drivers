package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.client.api.exception.ErrorCode.FEATURE_UNSUPPORTED;

import java.io.InputStream;
import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.NClob;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLClientInfoException;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Savepoint;
import java.sql.Statement;
import java.sql.Struct;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.LegacyKeyNormalizer;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ValidationIssue;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.WrapperIdentity.Builder;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.NotImplementedException;

public class SnowflakeConnectionImpl implements InternalSnowflakeConnection, DelegatingWrapper {

  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeConnectionImpl.class);

  private final AtomicBoolean closed = new AtomicBoolean(false);
  private final Set<Statement> openStatements = ConcurrentHashMap.newKeySet();
  private final CoreDriverApi coreDriverApi;
  private final DatabaseHandle databaseHandle;
  private final ConnectionHandle connectionHandle;
  private final Properties resolvedProperties;

  private boolean autoCommit;
  private String catalog;
  private String schema;

  private volatile String cachedDatabaseVersion;
  private final Object databaseVersionLock = new Object();

  public SnowflakeConnectionImpl(String url, Properties properties) throws SQLException {
    this(url, properties, ProtobufApis.coreDriverApi);
  }

  SnowflakeConnectionImpl(String url, Properties properties, CoreDriverApi coreDriverApi)
      throws SQLException {
    this.resolvedProperties = ConnectionOptionsResolver.resolve(url, properties);
    this.coreDriverApi = coreDriverApi;

    DatabaseHandle dbHandle = null;
    ConnectionHandle connHandle = null;
    try {
      dbHandle = coreDriverApi.databaseNew().getDbHandle();
      coreDriverApi.databaseInit(dbHandle);
      connHandle = coreDriverApi.connectionNew().getConnHandle();

      setOptions(connHandle, resolvedProperties);

      WrapperIdentity identity = wrapperIdentity();
      coreDriverApi.connectionInit(connHandle, dbHandle, identity);

      this.databaseHandle = dbHandle;
      this.connectionHandle = connHandle;
      this.autoCommit = fetchAutoCommit(coreDriverApi, connHandle);
    } catch (SQLException e) {
      releaseHandlesQuietly(coreDriverApi, connHandle, dbHandle);
      throw e;
    }
  }

  private static boolean fetchAutoCommit(CoreDriverApi coreDriverApi, ConnectionHandle connHandle) {
    try {
      ConnectionGetParameterResponse response =
          coreDriverApi.connectionGetParameter(connHandle, "AUTOCOMMIT");
      if (response.hasValue()) {
        return Boolean.parseBoolean(response.getValue());
      }
    } catch (SQLException e) {
      logger.warn("Failed to read AUTOCOMMIT session parameter; defaulting to true", e);
    }
    return true;
  }

  private WrapperIdentity wrapperIdentity() {
    Builder identityBuilder =
        WrapperIdentity.newBuilder()
            .setDriverName("JDBC")
            .setDriverVersion(SnowflakeDriver.DRIVER_VERSION);
    String runtimeName = System.getProperty("java.vm.name");
    if (runtimeName != null && !runtimeName.trim().isEmpty()) {
      identityBuilder.setLanguageRuntime(runtimeName);
    }
    String runtimeVersion = System.getProperty("java.version");
    if (runtimeVersion != null && !runtimeVersion.trim().isEmpty()) {
      identityBuilder.setLanguageVersion(runtimeVersion);
    }
    return identityBuilder.build();
  }

  private void setOptions(ConnectionHandle connHandle, Properties connectionOptions)
      throws SQLException {
    Map<String, ConfigSetting> optionsMap = new HashMap<>();

    // JDBC convention: Connection.close() must not throw on logout failure.
    // Users can opt into Strict via the "logout_error_strategy" connection property.
    optionsMap.put(
        "logout_error_strategy", ConfigSetting.newBuilder().setStringValue("best_effort").build());

    connectionOptions.forEach(
        (key, value) -> {
          if (!(key instanceof String)) {
            return;
          }
          String keyStr = LegacyKeyNormalizer.normalize((String) key);
          ConfigSetting configSetting = ConfigSettingFactory.from(value);
          if (configSetting != null) {
            optionsMap.put(keyStr, configSetting);
          }
        });

    if (!optionsMap.isEmpty()) {
      ConnectionSetOptionsResponse response =
          coreDriverApi.connectionSetOptions(connHandle, optionsMap);
      logConnectionOptionWarnings(response);
    }
  }

  @Override
  public ConnectionHandle getHandle() {
    return connectionHandle;
  }

  @Override
  public Properties getResolvedProperties() {
    // Return a copy so callers can't mutate the connection's resolved properties.
    return (Properties) resolvedProperties.clone();
  }

  @Override
  public Statement createStatement() throws SQLException {
    checkClosed();
    Statement stmt = new SnowflakeStatementImpl(this, coreDriverApi);
    openStatements.add(stmt);
    return stmt;
  }

  @Override
  public PreparedStatement prepareStatement(String sql) throws SQLException {
    checkClosed();
    PreparedStatement stmt = new SnowflakePreparedStatementImpl(this, sql, coreDriverApi);
    openStatements.add(stmt);
    return stmt;
  }

  @Override
  public CallableStatement prepareCall(String sql) throws SQLException {
    checkClosed();
    CallableStatement stmt = new SnowflakeCallableStatementImpl(this, sql, coreDriverApi);
    openStatements.add(stmt);
    return stmt;
  }

  @Override
  public String nativeSQL(String sql) throws SQLException {
    checkClosed();
    return sql;
  }

  private static void logConnectionOptionWarnings(ConnectionSetOptionsResponse response) {
    for (ValidationIssue warning : response.getWarningsList()) {
      logger.warn(
          "Connection option warning: severity={}, parameter={}, code={}, message={}",
          warning.getSeverity(),
          warning.getParameter(),
          warning.getCode(),
          warning.getMessage());
    }
  }

  @Override
  public void setAutoCommit(boolean autoCommit) throws SQLException {
    boolean currentAutoCommit = getAutoCommit();
    if (autoCommit != currentAutoCommit) {
      this.autoCommit = autoCommit;
      coreDriverApi.connectionSetAutocommit(connectionHandle, autoCommit);
    }
  }

  @Override
  public boolean getAutoCommit() throws SQLException {
    checkClosed();
    return autoCommit;
  }

  @Override
  public void commit() throws SQLException {
    checkClosed();
    coreDriverApi.connectionCommit(connectionHandle);
  }

  @Override
  public void rollback() throws SQLException {
    checkClosed();
    coreDriverApi.connectionRollback(connectionHandle);
  }

  @Override
  public void close() throws SQLException {
    if (!closed.compareAndSet(false, true)) {
      return;
    }

    logger.debug("Closing connection");
    closeOpenStatements();

    try {
      coreDriverApi.connectionClose(connectionHandle);
    } catch (SQLException e) {
      logger.warn("Error during connection close: {}", e.getMessage());
      logger.debug("Connection close error details", e);
      throw e;
    } finally {
      releaseHandlesQuietly(coreDriverApi, connectionHandle, databaseHandle);
    }
  }

  @Override
  public void removeStatement(Statement stmt) {
    openStatements.remove(stmt);
  }

  private void closeOpenStatements() {
    for (Statement stmt : openStatements) {
      try {
        if (!stmt.isClosed()) {
          stmt.close();
        }
      } catch (SQLException e) {
        logger.debug("Error closing statement during connection close", e);
      }
    }
    openStatements.clear();
  }

  private static void releaseHandlesQuietly(
      CoreDriverApi driver, ConnectionHandle connHandle, DatabaseHandle dbHandle) {
    if (connHandle != null) {
      try {
        driver.connectionRelease(connHandle);
      } catch (SQLException e) {
        logger.debug("Error releasing connection handle", e);
      }
    }
    if (dbHandle != null) {
      try {
        driver.databaseRelease(dbHandle);
      } catch (SQLException e) {
        logger.debug("Error releasing database handle", e);
      }
    }
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed.get();
  }

  @Override
  public DatabaseMetaData getMetaData() throws SQLException {
    checkClosed();
    return new SnowflakeDatabaseMetaDataImpl(this, resolvedProperties, coreDriverApi);
  }

  @Override
  public void setReadOnly(boolean readOnly) throws SQLException {
    checkClosed();
    logger.debug("setReadOnly not supported.", false);
  }

  @Override
  public boolean isReadOnly() throws SQLException {
    checkClosed();
    return false;
  }

  @Override
  public void setCatalog(String catalog) throws SQLException {
    checkClosed();
    coreDriverApi.connectionUseDatabase(connectionHandle, catalog);
    this.catalog = readCurrentCatalog();
  }

  @Override
  public String getCatalog() throws SQLException {
    checkClosed();
    catalog = readCurrentCatalog();
    return catalog;
  }

  private String readCurrentCatalog() throws SQLException {
    ConnectionGetInfoResponse info = coreDriverApi.connectionGetInfo(connectionHandle);
    if (info == null || !info.hasDatabase() || info.getDatabase().isEmpty()) {
      return null;
    }
    return info.getDatabase();
  }

  @Override
  public void setTransactionIsolation(int level) throws SQLException {
    checkClosed();
    // Snowflake always runs at READ COMMITTED and ignores the requested level, so this is a
    // no-op. We still reject levels the driver doesn't advertise, deferring to
    // DatabaseMetaData.supportsTransactionIsolationLevel so the accepted set (NONE,
    // READ_COMMITTED) lives in a single place rather than being duplicated here.
    if (!getMetaData().supportsTransactionIsolationLevel(level)) {
      throw new SQLFeatureNotSupportedException(
          "Transaction Isolation " + level + " not supported.");
    }
  }

  @Override
  public int getTransactionIsolation() throws SQLException {
    checkClosed();
    return Connection.TRANSACTION_READ_COMMITTED;
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public void clearWarnings() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Statement createStatement(int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return createStatement(resultSetType, resultSetConcurrency, ResultSet.CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return prepareStatement(
        sql, resultSetType, resultSetConcurrency, ResultSet.CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return prepareCall(sql, resultSetType, resultSetConcurrency, ResultSet.CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public Map<String, Class<?>> getTypeMap() throws SQLException {
    checkClosed();
    return Collections.emptyMap(); // nop
  }

  @Override
  public void setTypeMap(Map<String, Class<?>> map) throws SQLException {
    throw new SQLFeatureNotSupportedException("setTypeMap not supported");
  }

  @Override
  public void setHoldability(int holdability) throws SQLException {
    throw new SQLFeatureNotSupportedException(
        "Holdability other than ResultSet.CLOSE_CURSORS_AT_COMMIT is not supported");
  }

  @Override
  public int getHoldability() throws SQLException {
    checkClosed();
    return ResultSet.CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public Savepoint setSavepoint() throws SQLException {
    throw new SQLFeatureNotSupportedException("setSavepoint not supported");
  }

  @Override
  public Savepoint setSavepoint(String name) throws SQLException {
    throw new SQLFeatureNotSupportedException("setSavepoint not supported");
  }

  @Override
  public void rollback(Savepoint savepoint) throws SQLException {
    throw new SQLFeatureNotSupportedException("rollback to savepoint not supported");
  }

  @Override
  public void releaseSavepoint(Savepoint savepoint) throws SQLException {
    throw new SQLFeatureNotSupportedException("releaseSavepoint not supported");
  }

  @Override
  public Statement createStatement(
      int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException {
    validateStmtType(resultSetType, resultSetConcurrency, resultSetHoldability);
    return createStatement();
  }

  @Override
  public PreparedStatement prepareStatement(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    validateStmtType(resultSetType, resultSetConcurrency, resultSetHoldability);
    return prepareStatement(sql);
  }

  @Override
  public CallableStatement prepareCall(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    validateStmtType(resultSetType, resultSetConcurrency, resultSetHoldability);
    return prepareCall(sql);
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int autoGeneratedKeys) throws SQLException {
    if (autoGeneratedKeys == Statement.NO_GENERATED_KEYS) {
      return prepareStatement(sql);
    }

    throw new SQLFeatureNotSupportedException(
        String.format("autoGeneratedKeys %s not supported", autoGeneratedKeys));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException {
    throw new SQLFeatureNotSupportedException("prepareStatement with columnIndexes not supported");
  }

  @Override
  public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException {
    throw new SQLFeatureNotSupportedException("prepareStatement with columnNames not supported");
  }

  @Override
  public Clob createClob() throws SQLException {
    checkClosed();
    // A growable Clob: callers build content via setString(1, ...). The JDK SerialClob cannot be
    // used because it is fixed-length and rejects writes past its (zero) initial length.
    return new SnowflakeClob();
  }

  @Override
  public Blob createBlob() throws SQLException {
    throw new SQLFeatureNotSupportedException("createBlob not supported");
  }

  @Override
  public NClob createNClob() throws SQLException {
    throw new SQLFeatureNotSupportedException("createNClob not supported");
  }

  @Override
  public SQLXML createSQLXML() throws SQLException {
    throw new SQLFeatureNotSupportedException("createSQLXML not supported");
  }

  @Override
  public boolean isValid(int timeout) throws SQLException {
    if (timeout < 0) {
      throw new SQLException("timeout is less than 0");
    }
    if (closed.get()) {
      return false;
    }

    try {
      return coreDriverApi.connectionHeartbeat(connectionHandle, timeout).getValid();
    } catch (Exception e) {
      logger.debug("isValid check failed", e);
      return false;
    }
  }

  @Override
  public void setClientInfo(String name, String value) throws SQLClientInfoException {
    throw new NotImplementedException();
  }

  @Override
  public void setClientInfo(Properties properties) throws SQLClientInfoException {
    throw new NotImplementedException();
  }

  @Override
  public String getClientInfo(String name) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Properties getClientInfo() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Array createArrayOf(String typeName, Object[] elements) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Struct createStruct(String typeName, Object[] attributes) throws SQLException {
    throw new SQLFeatureNotSupportedException("createStruct not supported");
  }

  @Override
  public void setSchema(String schema) throws SQLException {
    checkClosed();
    coreDriverApi.connectionUseSchema(connectionHandle, schema);
    this.schema = readCurrentSchema();
  }

  @Override
  public String getSchema() throws SQLException {
    checkClosed();
    schema = readCurrentSchema();
    return schema;
  }

  private String readCurrentSchema() throws SQLException {
    ConnectionGetInfoResponse info = coreDriverApi.connectionGetInfo(connectionHandle);
    if (info == null || !info.hasSchema() || info.getSchema().isEmpty()) {
      return null;
    }
    return info.getSchema();
  }

  @Override
  public void abort(Executor executor) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public void setNetworkTimeout(Executor executor, int milliseconds) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getNetworkTimeout() throws SQLException {
    throw new NotImplementedException();
  }

  public void checkClosed() throws SQLException {
    if (isClosed()) {
      throw new SQLException("Connection is closed");
    }
  }

  @Override
  public void uploadStream(String stageName, String destFileName, InputStream inputStream)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public void uploadStream(
      String stageName, String destFileName, InputStream inputStream, UploadStreamConfig config)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public InputStream downloadStream(String stageName, String sourceFileName) throws SQLException {
    return downloadStream(stageName, sourceFileName, DownloadStreamConfig.builder().build());
  }

  @Override
  public InputStream downloadStream(
      String stageName, String sourceFileName, DownloadStreamConfig config) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getSessionID() throws SQLException {
    checkClosed();
    ConnectionGetInfoResponse info = coreDriverApi.connectionGetInfo(connectionHandle);
    if (!info.hasSessionId()) {
      return null;
    }
    return Long.toString(info.getSessionId());
  }

  @Override
  public QueryStatus getQueryStatus(String queryID) throws SQLException {
    checkClosed();
    ConnectionGetQueryStatusResponse response =
        coreDriverApi.connectionGetQueryStatus(connectionHandle, queryID);
    return QueryStatusMapper.fromCoreResponse(response);
  }

  /**
   * {@inheritDoc}
   *
   * <p>Returns a {@link net.snowflake.client.api.resultset.SnowflakeAsyncResultSet} that lazily
   * polls for query completion and materializes results on first data access. This matches the old
   * JDBC driver behavior, allowing callers to reconnect and retrieve results for queries submitted
   * by a previous session.
   */
  @Override
  public ResultSet createResultSet(String queryID) throws SQLException {
    checkClosed();
    // This statement is owned exclusively by the async result set and will be closed
    // when the result set is closed.
    SnowflakeStatementImpl stmt = (SnowflakeStatementImpl) createStatement();
    return ResultSetFactory.createAsync(queryID, this, stmt, true);
  }

  @Override
  public InternalResultSet createResultSetFromSfqid(
      String queryID, SnowflakeStatementImpl statement) throws SQLException {
    ResultSetResponse rsResponse = coreDriverApi.connectionGetResultSet(connectionHandle, queryID);
    return ResultSetFactory.create(coreDriverApi, statement, queryID, rsResponse);
  }

  @Override
  public String[] getChildQueryIds(String queryID) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getDatabaseMajorVersion() throws SQLException {
    return SnowflakeDriver.parseVersionComponent(getDatabaseVersion(), 0);
  }

  @Override
  public int getDatabaseMinorVersion() throws SQLException {
    return SnowflakeDriver.parseVersionComponent(getDatabaseVersion(), 1);
  }

  /** Issues a single {@code SELECT CURRENT_VERSION()} on first call per connection; cached. */
  @Override
  public String getDatabaseVersion() throws SQLException {
    checkClosed();
    String cached = cachedDatabaseVersion;
    if (cached != null) {
      return cached;
    }
    synchronized (databaseVersionLock) {
      if (cachedDatabaseVersion == null) {
        cachedDatabaseVersion = fetchDatabaseVersion();
      }
      return cachedDatabaseVersion;
    }
  }

  private String fetchDatabaseVersion() throws SQLException {
    try (Statement stmt = createStatement();
        ResultSet rs = stmt.executeQuery(ConnectionQueries.CURRENT_VERSION)) {
      if (!rs.next()) {
        throw new SQLException("SELECT CURRENT_VERSION() returned no rows");
      }
      String raw = rs.getString(1);
      if (raw == null) {
        throw new SQLException("SELECT CURRENT_VERSION() returned NULL");
      }
      return stripVersionSuffix(raw);
    }
  }

  // The server returns e.g. "8.46.1 abcdef"; the build suffix is not part of the public version.
  static String stripVersionSuffix(String raw) {
    if (raw == null) {
      return null;
    }
    String trimmed = raw.trim();
    int spaceIdx = trimmed.indexOf(' ');
    return spaceIdx < 0 ? trimmed : trimmed.substring(0, spaceIdx);
  }

  private static void validateStmtType(
      int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLFeatureNotSupportedException {
    if (resultSetType != ResultSet.TYPE_FORWARD_ONLY) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet type %d is not supported.", resultSetType),
          FEATURE_UNSUPPORTED.getSqlState(),
          FEATURE_UNSUPPORTED.getMessageCode());
    }

    if (resultSetConcurrency != ResultSet.CONCUR_READ_ONLY) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet concurrency %d is not supported.", resultSetConcurrency),
          FEATURE_UNSUPPORTED.getSqlState(),
          FEATURE_UNSUPPORTED.getMessageCode());
    }

    if (resultSetHoldability != ResultSet.CLOSE_CURSORS_AT_COMMIT) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet holdability %d is not supported.", resultSetHoldability),
          FEATURE_UNSUPPORTED.getSqlState(),
          FEATURE_UNSUPPORTED.getMessageCode());
    }
  }
}
