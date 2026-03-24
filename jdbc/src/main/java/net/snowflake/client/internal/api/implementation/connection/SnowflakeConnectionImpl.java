package net.snowflake.client.internal.api.implementation.connection;

import static java.sql.ResultSet.CONCUR_READ_ONLY;
import static java.sql.ResultSet.TYPE_FORWARD_ONLY;

import java.io.InputStream;
import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.ClientInfoStatus;
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
import java.util.concurrent.Executor;
import net.snowflake.client.api.connection.DownloadStreamConfig;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.api.connection.UploadStreamConfig;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.api.implementation.metadata.SnowflakeDatabaseMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakePreparedStatementImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionIntRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionStringRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewRequest;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Snowflake JDBC Connection implementation
 *
 * <p>This is a stub implementation that provides the basic JDBC Connection interface and delegates
 * to native Rust implementation via JNI.
 */
public class SnowflakeConnectionImpl implements SnowflakeConnection, Connection {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeConnectionImpl.class);
  private final String url;
  private final Properties properties;
  private boolean autoCommit = true;
  private boolean closed = false;
  private String catalog;
  private String schema;
  private int transactionIsolation = TRANSACTION_READ_COMMITTED;
  private int networkTimeoutInMilli = 0; //TODO not implemented
  private DatabaseHandle databaseHandle;
  public ConnectionHandle connectionHandle;

  public SnowflakeConnectionImpl(String url, Properties properties) throws SQLException {
    this.url = url;
    this.properties = properties;
    Properties connectionOptions = ConnectionOptionsResolver.resolve(url, properties);
    try {
      this.databaseHandle =
          ProtobufApis.databaseDriverV1
              .databaseNew(DatabaseNewRequest.getDefaultInstance())
              .getDbHandle();
      DatabaseInitRequest databaseInitRequest =
          DatabaseInitRequest.newBuilder().setDbHandle(databaseHandle).build();
      ProtobufApis.databaseDriverV1.databaseInit(databaseInitRequest);
      this.connectionHandle =
          ProtobufApis.databaseDriverV1
              .connectionNew(ConnectionNewRequest.getDefaultInstance())
              .getConnHandle();
      connectionOptions.forEach(
          (key, value) -> {
            if (!(key instanceof String)) {
              return;
            }

            String keyStr = (String) key;
            if (value instanceof String) {
              ConnectionSetOptionStringRequest request =
                  ConnectionSetOptionStringRequest.newBuilder()
                      .setConnHandle(connectionHandle)
                      .setKey(keyStr)
                      .setValue((String) value)
                      .build();
              try {
                ProtobufApis.databaseDriverV1.connectionSetOptionString(request);
              } catch (DatabaseDriverService.ServiceException e) {
                throw new RuntimeException(e);
              }
            }

            if (value instanceof Integer) {
              ConnectionSetOptionIntRequest request =
                  ConnectionSetOptionIntRequest.newBuilder()
                      .setConnHandle(connectionHandle)
                      .setKey(keyStr)
                      .setValue((Integer) value)
                      .build();
              try {
                ProtobufApis.databaseDriverV1.connectionSetOptionInt(request);
              } catch (DatabaseDriverService.ServiceException e) {
                throw new RuntimeException(e);
              }
            }
          });
      ConnectionInitRequest connectionInitRequest =
          ConnectionInitRequest.newBuilder()
              .setDbHandle(databaseHandle)
              .setConnHandle(connectionHandle)
              .build();
      ProtobufApis.databaseDriverV1.connectionInit(connectionInitRequest);
    } catch (DatabaseDriverService.ServiceException e) {
      throw new SQLException(e);
    }
  }

  @Override
  public Statement createStatement() throws SQLException {
    checkClosed();
    return new SnowflakeStatementImpl(this);
  }

  @Override
  public PreparedStatement prepareStatement(String sql) throws SQLException {
    checkClosed();
    return new SnowflakePreparedStatementImpl(this, sql);
  }

  @Override
  public CallableStatement prepareCall(String sql) throws SQLException {
    checkClosed();
    return new SnowflakeCallableStatementWrapper(prepareStatement(sql));
  }

  @Override
  public String nativeSQL(String sql) throws SQLException {
    checkClosed();
    return sql;
  }

  @Override
  public void setAutoCommit(boolean autoCommit) throws SQLException {
    checkClosed();
    if (autoCommit != this.autoCommit) {
      this.autoCommit = autoCommit;
      try (Statement stmt = createStatement()) {
        stmt.execute("alter session set autocommit=" + autoCommit);
      }
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
    try (Statement stmt = createStatement()) {
      stmt.execute("COMMIT");
    }
  }

  @Override
  public void rollback() throws SQLException {
    checkClosed();
    try (Statement stmt = createStatement()) {
      stmt.execute("ROLLBACK");
    }
  }

  @Override
  public void close() throws SQLException {
    if (!closed) {
      closed = true;
    }
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed;
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
    try (Statement stmt = createStatement()) {
      stmt.execute("use database \"" + catalog + "\"");
    }
  }

  @Override
  public String getCatalog() throws SQLException {
    checkClosed();
    try (Statement stmt = createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_DATABASE()")) {
      if (rs.next()) {
        return rs.getString(1);
      }
    }
    return null;
  }

  @Override
  public void setTransactionIsolation(int level) throws SQLException {
    checkClosed();
    if (level == Connection.TRANSACTION_NONE || level == TRANSACTION_READ_COMMITTED) {
      this.transactionIsolation = level;
    } else {
      throw new SQLFeatureNotSupportedException(
          "Transaction Isolation " + level + " not supported.");
    }
  }

  @Override
  public int getTransactionIsolation() throws SQLException {
    checkClosed();
    return this.transactionIsolation;
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    checkClosed();
    return null;
  }

  @Override
  public void clearWarnings() throws SQLException {
    checkClosed();
  }

  @Override
  public Statement createStatement(int resultSetType, int resultSetConcurrency)
      throws SQLException {
    if (TYPE_FORWARD_ONLY != resultSetType) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet type %d is not supported.", resultSetType));
    }
    if (CONCUR_READ_ONLY != resultSetConcurrency) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet concurrency %d is not supported.", resultSetConcurrency));
    }
    return createStatement();
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    if (TYPE_FORWARD_ONLY != resultSetType) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet type %d is not supported.", resultSetType));
    }
    if (CONCUR_READ_ONLY != resultSetConcurrency) {
      throw new SQLFeatureNotSupportedException(
          String.format("ResultSet concurrency %d is not supported.", resultSetConcurrency));
    }
    return prepareStatement(sql);
  }

  @Override
  public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency)
      throws SQLException {
    return prepareCall(sql);
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
    return createStatement();
  }

  @Override
  public PreparedStatement prepareStatement(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    return prepareStatement(sql, resultSetType, resultSetConcurrency);
  }

  @Override
  public CallableStatement prepareCall(
      String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability)
      throws SQLException {
    return prepareCall(sql);
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int autoGeneratedKeys) throws SQLException {
    throw new SQLFeatureNotSupportedException("prepareStatement not supported");
  }

  @Override
  public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException {
    throw new SQLFeatureNotSupportedException("prepareStatement not supported");
  }

  @Override
  public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException {
    throw new SQLFeatureNotSupportedException("prepareStatement not supported");
  }

  @Override
  public Clob createClob() throws SQLException {
    checkClosed();
    return null;
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
    return !closed;
  }

  @Override
  public void setClientInfo(String name, String value) throws SQLClientInfoException {
    Map<String, ClientInfoStatus> failedProps = new HashMap<>();
    failedProps.put(name, ClientInfoStatus.REASON_UNKNOWN_PROPERTY);
    throw new SQLClientInfoException(
        "The client property cannot be set by setClientInfo.", failedProps);
  }

  @Override
  public void setClientInfo(Properties properties) throws SQLClientInfoException {
    Map<String, ClientInfoStatus> failedProps = new HashMap<>();
    for (String name : properties.stringPropertyNames()) {
      failedProps.put(name, ClientInfoStatus.REASON_UNKNOWN_PROPERTY);
    }
    throw new SQLClientInfoException(
        "The client property cannot be set by setClientInfo.", failedProps);
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
    throw new SQLFeatureNotSupportedException("createArrayOf not supported");
  }

  @Override
  public Struct createStruct(String typeName, Object[] attributes) throws SQLException {
    throw new SQLFeatureNotSupportedException("createStruct not supported");
  }

  @Override
  public void setSchema(String schema) throws SQLException {
    checkClosed();
    String databaseName = getCatalog();
    if (databaseName == null) {
      try (Statement stmt = createStatement()) {
        stmt.execute("use schema \"" + schema + "\"");
      }
    } else {
      try (Statement stmt = createStatement()) {
        stmt.execute("use schema \"" + databaseName + "\".\"" + schema + "\"");
      }
    }
  }

  @Override
  public String getSchema() throws SQLException {
    checkClosed();
    try (Statement stmt = createStatement();
        ResultSet rs = stmt.executeQuery("SELECT CURRENT_SCHEMA()")) {
      if (rs.next()) {
        return rs.getString(1);
      }
    }
    return null;
  }

  @Override
  public void abort(Executor executor) throws SQLException {
    close();
  }

  @Override
  public void setNetworkTimeout(Executor executor, int milliseconds) throws SQLException {
    checkClosed();
    networkTimeoutInMilli = milliseconds;
  }

  @Override
  public int getNetworkTimeout() throws SQLException {
    checkClosed();
    return networkTimeoutInMilli;
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
    return iface.isInstance(this);
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
    throw new NotImplementedException();
  }

  @Override
  public QueryStatus getQueryStatus(String queryID) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public ResultSet createResultSet(String queryID) throws SQLException {
    throw new SQLFeatureNotSupportedException("createResultSet not supported");
  }

  @Override
  public String[] getChildQueryIds(String queryID) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getDatabaseMajorVersion() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public int getDatabaseMinorVersion() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public String getDatabaseVersion() throws SQLException {
    throw new NotImplementedException();
  }
}
