package com.snowflake.jdbc;

import com.snowflake.unicore.ProtobufApis;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverService;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.DatabaseNewRequest;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.DatabaseHandle;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.DatabaseInitRequest;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ConnectionNewRequest;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitRequest;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionStringRequest;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionIntRequest;

import java.sql.*;
import java.util.Map;
import java.util.Properties;
import java.util.concurrent.Executor;

/**
 * Snowflake JDBC Connection implementation
 * 
 * This is a stub implementation that provides the basic JDBC Connection interface
 * and delegates to native Rust implementation via JNI.
 */
public class SnowflakeConnection implements Connection {

    private final String url;
    private final Properties properties;
    private boolean autoCommit = true;
    private boolean closed = false;
    private String catalog;
    private String schema;
    private DatabaseHandle databaseHandle;
    public ConnectionHandle connectionHandle;

    public SnowflakeConnection(String url, Properties properties) throws SQLException {
        this.url = url;
        this.properties = properties;
        try {
            this.databaseHandle = ProtobufApis.databaseDriverV1.databaseNew(DatabaseNewRequest.getDefaultInstance()).getDbHandle();
            DatabaseInitRequest databaseInitRequest =
                    DatabaseInitRequest.newBuilder().setDbHandle(databaseHandle).build();
            ProtobufApis.databaseDriverV1.databaseInit(databaseInitRequest);
            this.connectionHandle = ProtobufApis.databaseDriverV1.connectionNew(ConnectionNewRequest.getDefaultInstance()).getConnHandle();
            properties.forEach((key, value) -> {
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
        return new SnowflakeStatement(this);
    }

    @Override
    public PreparedStatement prepareStatement(String sql) throws SQLException {
        checkClosed();
        return new SnowflakePreparedStatement(this, sql);
    }

    @Override
    public CallableStatement prepareCall(String sql) throws SQLException {
        throw new SQLFeatureNotSupportedException("prepareCall not supported");
    }

    @Override
    public String nativeSQL(String sql) throws SQLException {
        checkClosed();
        return sql; // No translation needed in stub implementation
    }

    @Override
    public void setAutoCommit(boolean autoCommit) throws SQLException {
        checkClosed();
        this.autoCommit = autoCommit;
    }

    @Override
    public boolean getAutoCommit() throws SQLException {
        checkClosed();
        return autoCommit;
    }

    @Override
    public void commit() throws SQLException {
        checkClosed();
        if (autoCommit) {
            throw new SQLException("Cannot commit when autocommit is enabled");
        }
        // Stub implementation - no actual commit logic
    }

    @Override
    public void rollback() throws SQLException {
        checkClosed();
        if (autoCommit) {
            throw new SQLException("Cannot rollback when autocommit is enabled");
        }
        // Stub implementation - no actual rollback logic
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
        return new SnowflakeDatabaseMetaData(this);
    }

    @Override
    public void setReadOnly(boolean readOnly) throws SQLException {
        checkClosed();
        // Stub implementation - ignore readonly setting
    }

    @Override
    public boolean isReadOnly() throws SQLException {
        checkClosed();
        return false;
    }

    @Override
    public void setCatalog(String catalog) throws SQLException {
        checkClosed();
        this.catalog = catalog;
    }

    @Override
    public String getCatalog() throws SQLException {
        checkClosed();
        return catalog;
    }

    @Override
    public void setTransactionIsolation(int level) throws SQLException {
        checkClosed();
        // Stub implementation - ignore transaction isolation level
    }

    @Override
    public int getTransactionIsolation() throws SQLException {
        checkClosed();
        return Connection.TRANSACTION_READ_COMMITTED;
    }

    @Override
    public SQLWarning getWarnings() throws SQLException {
        checkClosed();
        return null;
    }

    @Override
    public void clearWarnings() throws SQLException {
        checkClosed();
        // Stub implementation - no warnings to clear
    }

    // Additional methods required by JDBC interface (stubs)
    @Override
    public Statement createStatement(int resultSetType, int resultSetConcurrency) throws SQLException {
        return createStatement();
    }

    @Override
    public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency) throws SQLException {
        return prepareStatement(sql);
    }

    @Override
    public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency) throws SQLException {
        throw new SQLFeatureNotSupportedException("prepareCall not supported");
    }

    @Override
    public Map<String, Class<?>> getTypeMap() throws SQLException {
        throw new SQLFeatureNotSupportedException("getTypeMap not supported");
    }

    @Override
    public void setTypeMap(Map<String, Class<?>> map) throws SQLException {
        throw new SQLFeatureNotSupportedException("setTypeMap not supported");
    }

    @Override
    public void setHoldability(int holdability) throws SQLException {
        checkClosed();
        // Stub implementation
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
    public Statement createStatement(int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException {
        return createStatement();
    }

    @Override
    public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException {
        return prepareStatement(sql);
    }

    @Override
    public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException {
        throw new SQLFeatureNotSupportedException("prepareCall not supported");
    }

    @Override
    public PreparedStatement prepareStatement(String sql, int autoGeneratedKeys) throws SQLException {
        return prepareStatement(sql);
    }

    @Override
    public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException {
        return prepareStatement(sql);
    }

    @Override
    public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException {
        return prepareStatement(sql);
    }

    @Override
    public Clob createClob() throws SQLException {
        throw new SQLFeatureNotSupportedException("createClob not supported");
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
        return !isClosed();
    }

    @Override
    public void setClientInfo(String name, String value) throws SQLClientInfoException {
        // Stub implementation
    }

    @Override
    public void setClientInfo(Properties properties) throws SQLClientInfoException {
        // Stub implementation
    }

    @Override
    public String getClientInfo(String name) throws SQLException {
        return null;
    }

    @Override
    public Properties getClientInfo() throws SQLException {
        return new Properties();
    }

    @Override
    public Array createArrayOf(String typeName, Object[] elements) throws SQLException {
        throw new SQLFeatureNotSupportedException("createArrayOf not supported");
    }

    @Override
    public Struct createStruct(String typeName, Object[] attributes) throws SQLException {
        throw new SQLFeatureNotSupportedException("createStruct not supported");
    }

    @Override
    public void setSchema(String schema) throws SQLException {
        checkClosed();
        this.schema = schema;
    }

    @Override
    public String getSchema() throws SQLException {
        checkClosed();
        return schema;
    }

    @Override
    public void abort(Executor executor) throws SQLException {
        close();
    }

    @Override
    public void setNetworkTimeout(Executor executor, int milliseconds) throws SQLException {
        // Stub implementation
    }

    @Override
    public int getNetworkTimeout() throws SQLException {
        return 0;
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
        return iface.isAssignableFrom(getClass());
    }

    private void checkClosed() throws SQLException {
        if (isClosed()) {
            throw new SQLException("Connection is closed");
        }
    }
} 