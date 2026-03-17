package net.snowflake.client.internal.api.implementation.connection;

import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.CallableStatement;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.Date;
import java.sql.NClob;
import java.sql.ParameterMetaData;
import java.sql.PreparedStatement;
import java.sql.Ref;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.RowId;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.Map;

class SnowflakeCallableStatementWrapper implements CallableStatement {
  private final PreparedStatement delegate;

  SnowflakeCallableStatementWrapper(PreparedStatement delegate) {
    this.delegate = delegate;
  }

  @Override
  public ResultSet executeQuery() throws SQLException {
    return delegate.executeQuery();
  }

  @Override
  public int executeUpdate() throws SQLException {
    return delegate.executeUpdate();
  }

  @Override
  public void setNull(int idx, int type) throws SQLException {
    delegate.setNull(idx, type);
  }

  @Override
  public void setBoolean(int idx, boolean x) throws SQLException {
    delegate.setBoolean(idx, x);
  }

  @Override
  public void setByte(int idx, byte x) throws SQLException {
    delegate.setByte(idx, x);
  }

  @Override
  public void setShort(int idx, short x) throws SQLException {
    delegate.setShort(idx, x);
  }

  @Override
  public void setInt(int idx, int x) throws SQLException {
    delegate.setInt(idx, x);
  }

  @Override
  public void setLong(int idx, long x) throws SQLException {
    delegate.setLong(idx, x);
  }

  @Override
  public void setFloat(int idx, float x) throws SQLException {
    delegate.setFloat(idx, x);
  }

  @Override
  public void setDouble(int idx, double x) throws SQLException {
    delegate.setDouble(idx, x);
  }

  @Override
  public void setBigDecimal(int idx, BigDecimal x) throws SQLException {
    delegate.setBigDecimal(idx, x);
  }

  @Override
  public void setString(int idx, String x) throws SQLException {
    delegate.setString(idx, x);
  }

  @Override
  public void setBytes(int idx, byte[] x) throws SQLException {
    delegate.setBytes(idx, x);
  }

  @Override
  public void setDate(int idx, Date x) throws SQLException {
    delegate.setDate(idx, x);
  }

  @Override
  public void setTime(int idx, Time x) throws SQLException {
    delegate.setTime(idx, x);
  }

  @Override
  public void setTimestamp(int idx, Timestamp x) throws SQLException {
    delegate.setTimestamp(idx, x);
  }

  @Override
  public void setAsciiStream(int idx, InputStream x, int len) throws SQLException {
    delegate.setAsciiStream(idx, x, len);
  }

  @SuppressWarnings("deprecation")
  @Override
  public void setUnicodeStream(int idx, InputStream x, int len) throws SQLException {
    delegate.setUnicodeStream(idx, x, len);
  }

  @Override
  public void setBinaryStream(int idx, InputStream x, int len) throws SQLException {
    delegate.setBinaryStream(idx, x, len);
  }

  @Override
  public void clearParameters() throws SQLException {
    delegate.clearParameters();
  }

  @Override
  public void setObject(int idx, Object x, int targetSqlType) throws SQLException {
    delegate.setObject(idx, x, targetSqlType);
  }

  @Override
  public void setObject(int idx, Object x) throws SQLException {
    delegate.setObject(idx, x);
  }

  @Override
  public boolean execute() throws SQLException {
    return delegate.execute();
  }

  @Override
  public void addBatch() throws SQLException {
    delegate.addBatch();
  }

  @Override
  public void setCharacterStream(int idx, Reader reader, int len) throws SQLException {
    delegate.setCharacterStream(idx, reader, len);
  }

  @Override
  public void setRef(int idx, Ref x) throws SQLException {
    delegate.setRef(idx, x);
  }

  @Override
  public void setBlob(int idx, Blob x) throws SQLException {
    delegate.setBlob(idx, x);
  }

  @Override
  public void setClob(int idx, Clob x) throws SQLException {
    delegate.setClob(idx, x);
  }

  @Override
  public void setArray(int idx, Array x) throws SQLException {
    delegate.setArray(idx, x);
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    return delegate.getMetaData();
  }

  @Override
  public void setDate(int idx, Date x, Calendar cal) throws SQLException {
    delegate.setDate(idx, x, cal);
  }

  @Override
  public void setTime(int idx, Time x, Calendar cal) throws SQLException {
    delegate.setTime(idx, x, cal);
  }

  @Override
  public void setTimestamp(int idx, Timestamp x, Calendar cal) throws SQLException {
    delegate.setTimestamp(idx, x, cal);
  }

  @Override
  public void setNull(int idx, int type, String typeName) throws SQLException {
    delegate.setNull(idx, type, typeName);
  }

  @Override
  public void setURL(int idx, URL x) throws SQLException {
    delegate.setURL(idx, x);
  }

  @Override
  public ParameterMetaData getParameterMetaData() throws SQLException {
    return delegate.getParameterMetaData();
  }

  @Override
  public void setRowId(int idx, RowId x) throws SQLException {
    delegate.setRowId(idx, x);
  }

  @Override
  public void setNString(int idx, String value) throws SQLException {
    delegate.setNString(idx, value);
  }

  @Override
  public void setNCharacterStream(int idx, Reader value, long len) throws SQLException {
    delegate.setNCharacterStream(idx, value, len);
  }

  @Override
  public void setNClob(int idx, NClob value) throws SQLException {
    delegate.setNClob(idx, value);
  }

  @Override
  public void setClob(int idx, Reader reader, long len) throws SQLException {
    delegate.setClob(idx, reader, len);
  }

  @Override
  public void setBlob(int idx, InputStream is, long len) throws SQLException {
    delegate.setBlob(idx, is, len);
  }

  @Override
  public void setNClob(int idx, Reader reader, long len) throws SQLException {
    delegate.setNClob(idx, reader, len);
  }

  @Override
  public void setSQLXML(int idx, SQLXML xmlObject) throws SQLException {
    delegate.setSQLXML(idx, xmlObject);
  }

  @Override
  public void setObject(int idx, Object x, int targetSqlType, int scaleOrLength)
      throws SQLException {
    delegate.setObject(idx, x, targetSqlType, scaleOrLength);
  }

  @Override
  public void setAsciiStream(int idx, InputStream x, long len) throws SQLException {
    delegate.setAsciiStream(idx, x, len);
  }

  @Override
  public void setBinaryStream(int idx, InputStream x, long len) throws SQLException {
    delegate.setBinaryStream(idx, x, len);
  }

  @Override
  public void setCharacterStream(int idx, Reader reader, long len) throws SQLException {
    delegate.setCharacterStream(idx, reader, len);
  }

  @Override
  public void setAsciiStream(int idx, InputStream x) throws SQLException {
    delegate.setAsciiStream(idx, x);
  }

  @Override
  public void setBinaryStream(int idx, InputStream x) throws SQLException {
    delegate.setBinaryStream(idx, x);
  }

  @Override
  public void setCharacterStream(int idx, Reader reader) throws SQLException {
    delegate.setCharacterStream(idx, reader);
  }

  @Override
  public void setNCharacterStream(int idx, Reader value) throws SQLException {
    delegate.setNCharacterStream(idx, value);
  }

  @Override
  public void setClob(int idx, Reader reader) throws SQLException {
    delegate.setClob(idx, reader);
  }

  @Override
  public void setBlob(int idx, InputStream is) throws SQLException {
    delegate.setBlob(idx, is);
  }

  @Override
  public void setNClob(int idx, Reader reader) throws SQLException {
    delegate.setNClob(idx, reader);
  }

  // Statement methods
  @Override
  public ResultSet executeQuery(String sql) throws SQLException {
    return delegate.executeQuery(sql);
  }

  @Override
  public int executeUpdate(String sql) throws SQLException {
    return delegate.executeUpdate(sql);
  }

  @Override
  public void close() throws SQLException {
    delegate.close();
  }

  @Override
  public int getMaxFieldSize() throws SQLException {
    return delegate.getMaxFieldSize();
  }

  @Override
  public void setMaxFieldSize(int max) throws SQLException {
    delegate.setMaxFieldSize(max);
  }

  @Override
  public int getMaxRows() throws SQLException {
    return delegate.getMaxRows();
  }

  @Override
  public void setMaxRows(int max) throws SQLException {
    delegate.setMaxRows(max);
  }

  @Override
  public void setEscapeProcessing(boolean enable) throws SQLException {
    delegate.setEscapeProcessing(enable);
  }

  @Override
  public int getQueryTimeout() throws SQLException {
    return delegate.getQueryTimeout();
  }

  @Override
  public void setQueryTimeout(int seconds) throws SQLException {
    delegate.setQueryTimeout(seconds);
  }

  @Override
  public void cancel() throws SQLException {
    delegate.cancel();
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    return delegate.getWarnings();
  }

  @Override
  public void clearWarnings() throws SQLException {
    delegate.clearWarnings();
  }

  @Override
  public void setCursorName(String name) throws SQLException {
    delegate.setCursorName(name);
  }

  @Override
  public boolean execute(String sql) throws SQLException {
    return delegate.execute(sql);
  }

  @Override
  public ResultSet getResultSet() throws SQLException {
    return delegate.getResultSet();
  }

  @Override
  public int getUpdateCount() throws SQLException {
    return delegate.getUpdateCount();
  }

  @Override
  public boolean getMoreResults() throws SQLException {
    return delegate.getMoreResults();
  }

  @Override
  public void setFetchDirection(int direction) throws SQLException {
    delegate.setFetchDirection(direction);
  }

  @Override
  public int getFetchDirection() throws SQLException {
    return delegate.getFetchDirection();
  }

  @Override
  public void setFetchSize(int rows) throws SQLException {
    delegate.setFetchSize(rows);
  }

  @Override
  public int getFetchSize() throws SQLException {
    return delegate.getFetchSize();
  }

  @Override
  public int getResultSetConcurrency() throws SQLException {
    return delegate.getResultSetConcurrency();
  }

  @Override
  public int getResultSetType() throws SQLException {
    return delegate.getResultSetType();
  }

  @Override
  public void addBatch(String sql) throws SQLException {
    delegate.addBatch(sql);
  }

  @Override
  public void clearBatch() throws SQLException {
    delegate.clearBatch();
  }

  @Override
  public int[] executeBatch() throws SQLException {
    return delegate.executeBatch();
  }

  @Override
  public Connection getConnection() throws SQLException {
    return delegate.getConnection();
  }

  @Override
  public boolean getMoreResults(int current) throws SQLException {
    return delegate.getMoreResults(current);
  }

  @Override
  public ResultSet getGeneratedKeys() throws SQLException {
    return delegate.getGeneratedKeys();
  }

  @Override
  public int executeUpdate(String sql, int autoGeneratedKeys) throws SQLException {
    return delegate.executeUpdate(sql, autoGeneratedKeys);
  }

  @Override
  public int executeUpdate(String sql, int[] columnIndexes) throws SQLException {
    return delegate.executeUpdate(sql, columnIndexes);
  }

  @Override
  public int executeUpdate(String sql, String[] columnNames) throws SQLException {
    return delegate.executeUpdate(sql, columnNames);
  }

  @Override
  public boolean execute(String sql, int autoGeneratedKeys) throws SQLException {
    return delegate.execute(sql, autoGeneratedKeys);
  }

  @Override
  public boolean execute(String sql, int[] columnIndexes) throws SQLException {
    return delegate.execute(sql, columnIndexes);
  }

  @Override
  public boolean execute(String sql, String[] columnNames) throws SQLException {
    return delegate.execute(sql, columnNames);
  }

  @Override
  public int getResultSetHoldability() throws SQLException {
    return delegate.getResultSetHoldability();
  }

  @Override
  public boolean isClosed() throws SQLException {
    return delegate.isClosed();
  }

  @Override
  public void setPoolable(boolean poolable) throws SQLException {
    delegate.setPoolable(poolable);
  }

  @Override
  public boolean isPoolable() throws SQLException {
    return delegate.isPoolable();
  }

  @Override
  public void closeOnCompletion() throws SQLException {
    delegate.closeOnCompletion();
  }

  @Override
  public boolean isCloseOnCompletion() throws SQLException {
    return delegate.isCloseOnCompletion();
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    return delegate.unwrap(iface);
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    return delegate.isWrapperFor(iface);
  }

  // CallableStatement OUT parameter methods - all throw not supported
  private SQLException notSupported() {
    return new SQLFeatureNotSupportedException("OUT parameters not supported");
  }

  @Override
  public void registerOutParameter(int idx, int sqlType) throws SQLException {
    throw notSupported();
  }

  @Override
  public void registerOutParameter(int idx, int sqlType, int scale) throws SQLException {
    throw notSupported();
  }

  @Override
  public boolean wasNull() throws SQLException {
    throw notSupported();
  }

  @Override
  public String getString(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public boolean getBoolean(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public byte getByte(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public short getShort(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public int getInt(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public long getLong(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public float getFloat(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public double getDouble(int idx) throws SQLException {
    throw notSupported();
  }

  @SuppressWarnings("deprecation")
  @Override
  public BigDecimal getBigDecimal(int idx, int scale) throws SQLException {
    throw notSupported();
  }

  @Override
  public byte[] getBytes(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Date getDate(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Time getTime(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Timestamp getTimestamp(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Object getObject(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public BigDecimal getBigDecimal(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Object getObject(int idx, Map<String, Class<?>> map) throws SQLException {
    throw notSupported();
  }

  @Override
  public Ref getRef(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Blob getBlob(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Clob getClob(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Array getArray(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Date getDate(int idx, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public Time getTime(int idx, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public Timestamp getTimestamp(int idx, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public void registerOutParameter(int idx, int sqlType, String typeName) throws SQLException {
    throw notSupported();
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType) throws SQLException {
    throw notSupported();
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, int scale)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void registerOutParameter(String parameterName, int sqlType, String typeName)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public URL getURL(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setURL(String parameterName, URL val) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNull(String parameterName, int sqlType) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBoolean(String parameterName, boolean x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setByte(String parameterName, byte x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setShort(String parameterName, short x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setInt(String parameterName, int x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setLong(String parameterName, long x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setFloat(String parameterName, float x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setDouble(String parameterName, double x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBigDecimal(String parameterName, BigDecimal x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setString(String parameterName, String x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBytes(String parameterName, byte[] x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setDate(String parameterName, Date x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setTime(String parameterName, Time x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, int length) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, int length) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType, int scale)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setObject(String parameterName, Object x, int targetSqlType) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setObject(String parameterName, Object x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, int length)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setDate(String parameterName, Date x, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setTime(String parameterName, Time x, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setTimestamp(String parameterName, Timestamp x, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNull(String parameterName, int sqlType, String typeName) throws SQLException {
    throw notSupported();
  }

  @Override
  public String getString(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public boolean getBoolean(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public byte getByte(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public short getShort(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public int getInt(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public long getLong(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public float getFloat(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public double getDouble(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public byte[] getBytes(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Date getDate(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Time getTime(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Timestamp getTimestamp(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Object getObject(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public BigDecimal getBigDecimal(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Object getObject(String parameterName, Map<String, Class<?>> map) throws SQLException {
    throw notSupported();
  }

  @Override
  public Ref getRef(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Blob getBlob(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Clob getClob(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Array getArray(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Date getDate(String parameterName, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public Time getTime(String parameterName, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public Timestamp getTimestamp(String parameterName, Calendar cal) throws SQLException {
    throw notSupported();
  }

  @Override
  public URL getURL(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public RowId getRowId(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public RowId getRowId(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setRowId(String parameterName, RowId x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNString(String parameterName, String value) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value, long length)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNClob(String parameterName, NClob value) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setClob(String parameterName, Reader reader, long length) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream, long length)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNClob(String parameterName, Reader reader, long length) throws SQLException {
    throw notSupported();
  }

  @Override
  public NClob getNClob(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public NClob getNClob(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setSQLXML(String parameterName, SQLXML xmlObject) throws SQLException {
    throw notSupported();
  }

  @Override
  public SQLXML getSQLXML(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public SQLXML getSQLXML(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public String getNString(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public String getNString(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Reader getNCharacterStream(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Reader getNCharacterStream(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public Reader getCharacterStream(int idx) throws SQLException {
    throw notSupported();
  }

  @Override
  public Reader getCharacterStream(String parameterName) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBlob(String parameterName, Blob x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setClob(String parameterName, Clob x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x, long length) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x, long length)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader, long length)
      throws SQLException {
    throw notSupported();
  }

  @Override
  public void setAsciiStream(String parameterName, InputStream x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBinaryStream(String parameterName, InputStream x) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setCharacterStream(String parameterName, Reader reader) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNCharacterStream(String parameterName, Reader value) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setClob(String parameterName, Reader reader) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setBlob(String parameterName, InputStream inputStream) throws SQLException {
    throw notSupported();
  }

  @Override
  public void setNClob(String parameterName, Reader reader) throws SQLException {
    throw notSupported();
  }

  @Override
  public <T> T getObject(int idx, Class<T> type) throws SQLException {
    throw notSupported();
  }

  @Override
  public <T> T getObject(String parameterName, Class<T> type) throws SQLException {
    throw notSupported();
  }
}
