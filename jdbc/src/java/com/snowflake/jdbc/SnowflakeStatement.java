package com.snowflake.jdbc;

import com.snowflake.jdbc.thrift_gen.ExecuteResult;
import com.snowflake.jdbc.thrift_gen.StatementHandle;
import org.apache.thrift.TException;

import java.io.IOException;
import java.sql.*;

/**
 * Snowflake JDBC Statement implementation
 * 
 * This is a stub implementation that provides the basic JDBC Statement interface
 * and delegates to native Rust implementation via JNI.
 */
public class SnowflakeStatement implements Statement {
    
    public final SnowflakeConnection connection;
    protected boolean closed = false;
    protected int maxRows = 0;
    protected int queryTimeout = 0;
    protected int fetchSize = 0;
    protected StatementHandle statementHandle;
    
    public SnowflakeStatement(SnowflakeConnection connection) {
        this.connection = connection;
        try {
            this.statementHandle = this.connection.driverApi.statementNew(this.connection.connectionHandle);
        } catch (TException e) {
            throw new RuntimeException(e);
        }
    }
    
    @Override
    public ResultSet executeQuery(String sql) throws SQLException {
        checkClosed();
        try {
            this.connection.driverApi.statementSetSqlQuery(this.statementHandle, sql);
        } catch (TException e) {
            throw new RuntimeException(e);
        }

        try {
            ExecuteResult result = this.connection.driverApi.statementExecuteQuery(this.statementHandle);
            return new SnowflakeResultSet(this, result);
        } catch (TException e) {
            throw new RuntimeException(e);
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }
    
    @Override
    public int executeUpdate(String sql) throws SQLException {
        checkClosed();
        return 0; // Stub: return 0 rows affected
    }
    
    @Override
    public void close() throws SQLException {
        closed = true;
    }
    
    @Override
    public int getMaxFieldSize() throws SQLException {
        checkClosed();
        return 0; // No limit in stub implementation
    }
    
    @Override
    public void setMaxFieldSize(int max) throws SQLException {
        checkClosed();
        // Stub implementation - ignore
    }
    
    @Override
    public int getMaxRows() throws SQLException {
        checkClosed();
        return maxRows;
    }
    
    @Override
    public void setMaxRows(int max) throws SQLException {
        checkClosed();
        this.maxRows = max;
    }
    
    @Override
    public void setEscapeProcessing(boolean enable) throws SQLException {
        checkClosed();
        // Stub implementation - ignore
    }
    
    @Override
    public int getQueryTimeout() throws SQLException {
        checkClosed();
        return queryTimeout;
    }
    
    @Override
    public void setQueryTimeout(int seconds) throws SQLException {
        checkClosed();
        this.queryTimeout = seconds;
    }
    
    @Override
    public void cancel() throws SQLException {
        checkClosed();
        // Stub implementation - no cancellation logic
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
    
    @Override
    public void setCursorName(String name) throws SQLException {
        throw new SQLFeatureNotSupportedException("setCursorName not supported");
    }
    
    @Override
    public boolean execute(String sql) throws SQLException {
        checkClosed();
        // Stub implementation - assume all statements return result sets
        executeQuery(sql);
        return true;
    }
    
    @Override
    public ResultSet getResultSet() throws SQLException {
        checkClosed();
        return null; // No current result set in stub implementation
    }
    
    @Override
    public int getUpdateCount() throws SQLException {
        checkClosed();
        return -1; // No update count in stub implementation
    }
    
    @Override
    public boolean getMoreResults() throws SQLException {
        checkClosed();
        return false; // No additional result sets in stub implementation
    }
    
    @Override
    public void setFetchDirection(int direction) throws SQLException {
        checkClosed();
        if (direction != ResultSet.FETCH_FORWARD) {
            throw new SQLFeatureNotSupportedException("Only FETCH_FORWARD supported");
        }
    }
    
    @Override
    public int getFetchDirection() throws SQLException {
        checkClosed();
        return ResultSet.FETCH_FORWARD;
    }
    
    @Override
    public void setFetchSize(int rows) throws SQLException {
        checkClosed();
        this.fetchSize = rows;
    }
    
    @Override
    public int getFetchSize() throws SQLException {
        checkClosed();
        return fetchSize;
    }
    
    @Override
    public int getResultSetConcurrency() throws SQLException {
        checkClosed();
        return ResultSet.CONCUR_READ_ONLY;
    }
    
    @Override
    public int getResultSetType() throws SQLException {
        checkClosed();
        return ResultSet.TYPE_FORWARD_ONLY;
    }
    
    @Override
    public void addBatch(String sql) throws SQLException {
        throw new SQLFeatureNotSupportedException("addBatch not supported");
    }
    
    @Override
    public void clearBatch() throws SQLException {
        throw new SQLFeatureNotSupportedException("clearBatch not supported");
    }
    
    @Override
    public int[] executeBatch() throws SQLException {
        throw new SQLFeatureNotSupportedException("executeBatch not supported");
    }
    
    @Override
    public Connection getConnection() throws SQLException {
        checkClosed();
        return connection;
    }
    
    @Override
    public boolean getMoreResults(int current) throws SQLException {
        checkClosed();
        return false;
    }
    
    @Override
    public ResultSet getGeneratedKeys() throws SQLException {
        throw new SQLFeatureNotSupportedException("getGeneratedKeys not supported");
    }
    
    @Override
    public int executeUpdate(String sql, int autoGeneratedKeys) throws SQLException {
        return executeUpdate(sql);
    }
    
    @Override
    public int executeUpdate(String sql, int[] columnIndexes) throws SQLException {
        return executeUpdate(sql);
    }
    
    @Override
    public int executeUpdate(String sql, String[] columnNames) throws SQLException {
        return executeUpdate(sql);
    }
    
    @Override
    public boolean execute(String sql, int autoGeneratedKeys) throws SQLException {
        return execute(sql);
    }
    
    @Override
    public boolean execute(String sql, int[] columnIndexes) throws SQLException {
        return execute(sql);
    }
    
    @Override
    public boolean execute(String sql, String[] columnNames) throws SQLException {
        return execute(sql);
    }
    
    @Override
    public int getResultSetHoldability() throws SQLException {
        checkClosed();
        return ResultSet.CLOSE_CURSORS_AT_COMMIT;
    }
    
    @Override
    public boolean isClosed() throws SQLException {
        return closed;
    }
    
    @Override
    public void setPoolable(boolean poolable) throws SQLException {
        checkClosed();
        // Stub implementation - ignore
    }
    
    @Override
    public boolean isPoolable() throws SQLException {
        checkClosed();
        return false;
    }
    
    @Override
    public void closeOnCompletion() throws SQLException {
        checkClosed();
        // Stub implementation - ignore
    }
    
    @Override
    public boolean isCloseOnCompletion() throws SQLException {
        checkClosed();
        return false;
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
    
    protected void checkClosed() throws SQLException {
        if (closed) {
            throw new SQLException("Statement is closed");
        }
        if (connection.isClosed()) {
            throw new SQLException("Connection is closed");
        }
    }
} 