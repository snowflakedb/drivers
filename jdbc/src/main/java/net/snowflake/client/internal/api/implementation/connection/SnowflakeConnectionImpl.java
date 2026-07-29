package net.snowflake.client.internal.api.implementation.connection;

import static java.sql.ClientInfoStatus.REASON_UNKNOWN_PROPERTY;
import static java.sql.Connection.TRANSACTION_NONE;
import static java.sql.Connection.TRANSACTION_READ_COMMITTED;
import static java.sql.ResultSet.CLOSE_CURSORS_AT_COMMIT;
import static java.sql.ResultSet.CONCUR_READ_ONLY;
import static java.sql.ResultSet.TYPE_FORWARD_ONLY;
import static java.util.Collections.emptyMap;
import static java.util.Collections.emptySet;
import static java.util.Collections.singleton;
import static net.snowflake.client.api.exception.ErrorCode.CONNECTION_CLOSED;
import static net.snowflake.client.api.exception.ErrorCode.FEATURE_UNSUPPORTED;
import static net.snowflake.client.api.exception.ErrorCode.INVALID_PARAMETER_VALUE;

import java.io.IOException;
import java.io.InputStream;
import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.ClientInfoStatus;
import java.sql.Clob;
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
import net.snowflake.client.internal.api.implementation.parameters.ConnectionOptionsResolver;
import net.snowflake.client.internal.api.implementation.parameters.CoreParametersRegistry;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.ParameterKeyNormalizer;
import net.snowflake.client.internal.api.implementation.parameters.ParameterValueNormalizer;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.InternalResultSet;
import net.snowflake.client.internal.api.implementation.resultset.ResultSetFactory;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeCallableStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.Jdk14LoggerBootstrap;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
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
  private final ParametersRegistry parametersRegistry;

  private boolean autoCommit;
  private String catalog;
  private String schema;
  private int transactionIsolation = TRANSACTION_NONE;

  private SQLWarning sqlWarnings;

  private volatile String cachedDatabaseVersion;
  private final Object databaseVersionLock = new Object();

  public SnowflakeConnectionImpl(String url, Properties properties) throws SQLException {
    this(url, properties, ProtobufApis.coreDriverApi);
  }

  SnowflakeConnectionImpl(String url, Properties properties, CoreDriverApi coreDriverApi)
      throws SQLException {
    try {
      Jdk14LoggerBootstrap.initFromConnectionIfConfigured(url, properties);
    } catch (IOException e) {
      throw new SQLException("Failed to initialize JDBC logging", e);
    }

    this.coreDriverApi = coreDriverApi;

    DatabaseHandle dbHandle = null;
    ConnectionHandle connHandle = null;
    try {
      dbHandle = coreDriverApi.databaseNew().getDbHandle();
      coreDriverApi.databaseInit(dbHandle);
      connHandle = coreDriverApi.connectionNew().getConnHandle();

      SQLWarning sqlWarnings = setOptions(connHandle, url, properties);

      WrapperIdentity identity = wrapperIdentity();
      coreDriverApi.connectionInit(connHandle, dbHandle, identity);

      this.databaseHandle = dbHandle;
      this.connectionHandle = connHandle;
      this.sqlWarnings = sqlWarnings;
      this.parametersRegistry = new CoreParametersRegistry(coreDriverApi, connHandle);
      this.autoCommit = parametersRegistry.getBool(Parameter.AUTOCOMMIT);
    } catch (SQLException e) {
      releaseHandlesQuietly(coreDriverApi, connHandle, dbHandle);
      throw e;
    }
  }

  private WrapperIdentity wrapperIdentity() {
    Builder identityBuilder =
        WrapperIdentity.newBuilder()
            .setDriverName("JDBC")
            .setDriverVersion(SnowflakeDriver.CLIENT_APP_VERSION);
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

  private SQLWarning setOptions(ConnectionHandle connHandle, String url, Properties properties)
      throws SQLException {
    Properties resolvedProperties = ConnectionOptionsResolver.resolve(url, properties);
    Map<String, ConfigSetting> options = new HashMap<>();

    // JDBC convention: Connection.close() must not throw on logout failure.
    // Users can opt into Strict via the "logout_error_strategy" connection property.
    options.put(
        "logout_error_strategy", ConfigSetting.newBuilder().setStringValue("best_effort").build());

    resolvedProperties.forEach(
        (key, value) -> {
          if (!(key instanceof String)) {
            return;
          }
          String keyStr = ParameterKeyNormalizer.normalize((String) key);
          Object normalizedValue = ParameterValueNormalizer.normalize(keyStr, value);
          ConfigSetting configSetting = ConfigSettingFactory.from(normalizedValue);
          if (configSetting != null) {
            options.put(keyStr, configSetting);
          }
        });

    if (!options.isEmpty()) {
      ConnectionSetOptionsResponse response =
          coreDriverApi.connectionSetOptions(connHandle, options);
      for (ValidationIssue warning : response.getWarningsList()) {
        logger.warn(
            "Connection option warning: severity={}, parameter={}, code={}, message={}",
            warning.getSeverity(),
            warning.getParameter(),
            warning.getCode(),
            warning.getMessage());
      }
    }

    return ConnectionEstablishedWarnings.compute(
        resolvedProperties, coreDriverApi.connectionGetInfo(connHandle));
  }

  @Override
  public ConnectionHandle getHandle() {
    return connectionHandle;
  }

  @Override
  public ParametersRegistry getParameters() {
    return parametersRegistry;
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
    return new SnowflakeDatabaseMetaDataImpl(this);
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
    if (level == TRANSACTION_NONE || level == TRANSACTION_READ_COMMITTED) {
      this.transactionIsolation = level;
      return;
    }
    throw featureNotSupported("Transaction Isolation " + level + " not supported.");
  }

  @Override
  public int getTransactionIsolation() throws SQLException {
    checkClosed();
    return transactionIsolation;
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    checkClosed();
    return sqlWarnings;
  }

  @Override
  public void clearWarnings() throws SQLException {
    checkClosed();
    sqlWarnings = null;
  }

  @Override
  public Statement createStatement(int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return createStatement(resultSetType, resultSetConcurrency, CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return prepareStatement(sql, resultSetType, resultSetConcurrency, CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return prepareCall(sql, resultSetType, resultSetConcurrency, CLOSE_CURSORS_AT_COMMIT);
  }

  @Override
  public Map<String, Class<?>> getTypeMap() throws SQLException {
    checkClosed();
    return emptyMap();
  }

  @Override
  public void setTypeMap(Map<String, Class<?>> map) throws SQLException {
    throw featureNotSupported("setTypeMap not supported");
  }

  @Override
  public void setHoldability(int holdability) throws SQLException {
    checkClosed();
    if (holdability != CLOSE_CURSORS_AT_COMMIT
        && holdability != ResultSet.HOLD_CURSORS_OVER_COMMIT) {
      throw new SQLException("The given parameter is not a ResultSet holdability constant.");
    }
    if (holdability == ResultSet.HOLD_CURSORS_OVER_COMMIT) {
      throw featureNotSupported("Holdability not supported");
    }
  }

  @Override
  public int getHoldability() throws SQLException {
    checkClosed();
    return CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public Savepoint setSavepoint() throws SQLException {
    throw featureNotSupported("setSavepoint not supported");
  }

  @Override
  public Savepoint setSavepoint(String name) throws SQLException {
    throw featureNotSupported("setSavepoint not supported");
  }

  @Override
  public void rollback(Savepoint savepoint) throws SQLException {
    throw featureNotSupported("rollback to savepoint not supported");
  }

  @Override
  public void releaseSavepoint(Savepoint savepoint) throws SQLException {
    throw featureNotSupported("releaseSavepoint not supported");
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

    throw featureNotSupported(
        String.format("autoGeneratedKeys %s not supported", autoGeneratedKeys));
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException {
    throw featureNotSupported("prepareStatement with columnIndexes not supported");
  }

  @Override
  public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException {
    throw featureNotSupported("prepareStatement with columnNames not supported");
  }

  @Override
  public Blob createBlob() throws SQLException {
    throw featureNotSupported("createBlob not supported");
  }

  @Override
  public Clob createClob() throws SQLException {
    checkClosed();
    return new SnowflakeClob();
  }

  @Override
  public NClob createNClob() throws SQLException {
    throw featureNotSupported("createNClob not supported");
  }

  @Override
  public SQLXML createSQLXML() throws SQLException {
    throw featureNotSupported("createSQLXML not supported");
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
    throwClientInfoIfClosed(singleton(name));
    Map<String, ClientInfoStatus> failedProps = new HashMap<>();
    failedProps.put(name, REASON_UNKNOWN_PROPERTY);
    raiseSetClientInfoException(failedProps);
  }

  @Override
  public void setClientInfo(Properties properties) throws SQLClientInfoException {
    Set<String> names = properties == null ? emptySet() : properties.stringPropertyNames();
    throwClientInfoIfClosed(names);
    if (names.isEmpty()) {
      return;
    }
    Map<String, ClientInfoStatus> failedProps = new HashMap<>();
    for (String name : names) {
      failedProps.put(name, REASON_UNKNOWN_PROPERTY);
    }
    raiseSetClientInfoException(failedProps);
  }

  private void throwClientInfoIfClosed(Set<String> names) throws SQLClientInfoException {
    if (closed.get()) {
      Map<String, ClientInfoStatus> failedProps = new HashMap<>();
      for (String name : names) {
        failedProps.put(name, REASON_UNKNOWN_PROPERTY);
      }
      throw new SQLClientInfoException(
          "The connection is not opened.",
          CONNECTION_CLOSED.getSqlState(),
          CONNECTION_CLOSED.getMessageCode(),
          failedProps);
    }
  }

  private static void raiseSetClientInfoException(Map<String, ClientInfoStatus> failedProps)
      throws SQLClientInfoException {
    throw new SQLClientInfoException(
        "The client property cannot be set by setClientInfo.",
        INVALID_PARAMETER_VALUE.getSqlState(),
        INVALID_PARAMETER_VALUE.getMessageCode(),
        failedProps);
  }

  @Override
  public String getClientInfo(String name) throws SQLException {
    checkClosed();
    return null;
  }

  @Override
  public Properties getClientInfo() throws SQLException {
    checkClosed();
    return new Properties();
  }

  @Override
  public Array createArrayOf(String typeName, Object[] elements) throws SQLException {
    checkClosed();
    throw new NotImplementedException();
  }

  @Override
  public Struct createStruct(String typeName, Object[] attributes) throws SQLException {
    throw featureNotSupported("createStruct not supported");
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
    close();
  }

  @Override
  public void setNetworkTimeout(Executor executor, int milliseconds) throws SQLException {
    checkClosed();
    // TODO: [retries&timeouts epic] delegate to sf_core once connection network timeout APIs land;
    // do not keep a JDBC-local cache here (legacy parity is out of scope for this PR).
  }

  @Override
  public int getNetworkTimeout() throws SQLException {
    checkClosed();
    // TODO: [retries&timeouts epic] read from sf_core once connection network timeout APIs land.
    return 0;
  }

  public void checkClosed() throws SQLException {
    if (isClosed()) {
      throw new SQLException(
          "Connection is closed",
          CONNECTION_CLOSED.getSqlState(),
          CONNECTION_CLOSED.getMessageCode());
    }
  }

  @Override
  public void uploadStream(String stageName, String destFileName, InputStream inputStream)
      throws SQLException {
    uploadStream(stageName, destFileName, inputStream, UploadStreamConfig.builder().build());
  }

  @Override
  public void uploadStream(
      String stageName, String destFileName, InputStream inputStream, UploadStreamConfig config)
      throws SQLException {
    checkClosed();
    logger.info("uploadStream: entry");
    try {
      byte[] data;
      try {
        java.io.ByteArrayOutputStream baos = new java.io.ByteArrayOutputStream();
        byte[] buf = new byte[8192];
        int n;
        while ((n = inputStream.read(buf)) != -1) {
          baos.write(buf, 0, n);
        }
        data = baos.toByteArray();
      } catch (java.io.IOException e) {
        throw new net.snowflake.client.api.exception.SnowflakeSQLException(
            "Failed to read input stream: " + e.getMessage(), e);
      }
      String destPrefix = config != null ? config.getDestPrefix() : null;
      boolean compressData = config == null || config.isCompressData();
      String sql = buildPutSql(stageName, destFileName, destPrefix, compressData);
      coreDriverApi.connectionUploadStream(connectionHandle, sql, data);
    } finally {
      logger.info("uploadStream: exit");
    }
  }

  /**
   * Synthesize a PUT SQL from the structured uploadStream parameters. The stage path is the user's
   * stage reference with {@code destPrefix} appended (so the file lands at {@code
   * <stage>/<prefix>/<destFile>}). The local file URI carries only the destination filename — its
   * basename is what GS uses to identify the stage object. {@code OVERWRITE = TRUE} mirrors the
   * reference JDBC contract that uploadStream always overwrites.
   *
   * <p>{@code compressData} maps to the server-side {@code AUTO_COMPRESS} clause rather than JDBC's
   * historical client-side gzip: when true (default) the clause is omitted so GS auto-compresses
   * and the object lands with a {@code .gz} suffix — functionally the same end state as the legacy
   * driver.
   */
  static String buildPutSql(
      String stageName, String destFileName, String destPrefix, boolean compressData) {
    String stagePath = stageName;
    if (destPrefix != null && !destPrefix.isEmpty()) {
      String trimmed = stagePath.endsWith("/") ? stagePath : stagePath + "/";
      stagePath = trimmed + destPrefix;
    }
    StringBuilder sql = new StringBuilder();
    // PUT syntax does not support IDENTIFIER(?) or ? bindings for stage paths or file
    // URIs; destFileName and stageName are caller-controlled values passed directly by
    // the JDBC user.
    sql.append("PUT 'file:///").append(destFileName).append("' ").append(stagePath);
    if (!compressData) {
      sql.append(" AUTO_COMPRESS = FALSE");
    }
    sql.append(" OVERWRITE = TRUE");
    return sql.toString();
  }

  @Override
  public InputStream downloadStream(String stageName, String sourceFileName) throws SQLException {
    return downloadStream(stageName, sourceFileName, DownloadStreamConfig.builder().build());
  }

  @Override
  public InputStream downloadStream(
      String stageName, String sourceFileName, DownloadStreamConfig config) throws SQLException {
    checkClosed();
    logger.info("downloadStream: entry");
    try {
      boolean decompress = config != null && config.isDecompress();
      net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1
              .ConnectionDownloadStreamResponse
          response =
              coreDriverApi.connectionDownloadStream(
                  connectionHandle, stageName, sourceFileName, decompress);
      return new java.io.ByteArrayInputStream(response.getData().toByteArray());
    } finally {
      logger.info("downloadStream: exit");
    }
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
    checkClosed();
    QueryStatus status = getQueryStatus(queryID);
    if (status.isStillRunning()) {
      throw new SQLException(
          "Status of query associated with resultSet is "
              + status.getDescription()
              + ". Results not generated.");
    }
    ExecuteQueryResponse result = coreDriverApi.connectionGetQueryResult(connectionHandle, queryID);
    // A single-statement query has no children; return the query ID itself.
    if (result.hasMulti() && result.getMulti().getQueryIdsCount() > 0) {
      return result.getMulti().getQueryIdsList().toArray(new String[0]);
    }
    return new String[] {queryID};
  }

  @Override
  public int getDatabaseMajorVersion() throws SQLException {
    return SnowflakeDriver.parseVersionComponent(getDatabaseVersion(), 0);
  }

  @Override
  public int getDatabaseMinorVersion() throws SQLException {
    return SnowflakeDriver.parseVersionComponent(getDatabaseVersion(), 1);
  }

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
    if (resultSetType != TYPE_FORWARD_ONLY) {
      throw featureNotSupported(
          String.format("ResultSet type %d is not supported.", resultSetType));
    }

    if (resultSetConcurrency != CONCUR_READ_ONLY) {
      throw featureNotSupported(
          String.format("ResultSet concurrency %d is not supported.", resultSetConcurrency));
    }

    if (resultSetHoldability != CLOSE_CURSORS_AT_COMMIT) {
      throw featureNotSupported(
          String.format("ResultSet holdability %d is not supported.", resultSetHoldability));
    }
  }

  private static SQLFeatureNotSupportedException featureNotSupported(String message) {
    return new SQLFeatureNotSupportedException(
        message, FEATURE_UNSUPPORTED.getSqlState(), FEATURE_UNSUPPORTED.getMessageCode());
  }
}
