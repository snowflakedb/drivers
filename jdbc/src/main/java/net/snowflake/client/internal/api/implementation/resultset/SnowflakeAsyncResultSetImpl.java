package net.snowflake.client.internal.api.implementation.resultset;

import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.Clob;
import java.sql.Date;
import java.sql.NClob;
import java.sql.Ref;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.RowId;
import java.sql.SQLException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.resultset.metadata.DecoratedSnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;

/**
 * Async ResultSet that wraps a query ID from an async query submission.
 *
 * <p>On first data access ({@link #next()}, {@link #getMetaData()}), polls the query status until
 * completion, then fetches real results from the Core driver and delegates all subsequent calls to
 * the materialized {@link ResultSet}.
 *
 * <p>All {@link ResultSet} and {@link SnowflakeResultSet} interface methods delegate to the
 * underlying result set after materialization. This class never makes its own decisions about what
 * is or isn't supported — the delegate handles that.
 */
public class SnowflakeAsyncResultSetImpl implements InternalAsyncResultSet {

  private final String queryID;
  private final InternalSnowflakeConnection snowflakeConnection;
  private final SnowflakeStatementImpl statement;
  private final boolean ownsStatement;
  private final QueryResultWaiter waiter;

  private volatile InternalResultSet delegate;
  private volatile boolean closed = false;
  private volatile QueryStatus lastQueriedStatus = QueryStatus.empty();

  SnowflakeAsyncResultSetImpl(
      String queryID,
      InternalSnowflakeConnection snowflakeConnection,
      SnowflakeStatementImpl statement,
      boolean ownsStatement) {
    this.queryID = queryID;
    this.snowflakeConnection = snowflakeConnection;
    this.statement = statement;
    this.ownsStatement = ownsStatement;
    this.waiter = new QueryResultWaiter(this::getStatus, queryID);
  }

  // =========================================================================
  // Async-specific (SnowflakeAsyncResultSet)
  // =========================================================================

  @Override
  public QueryStatus getStatus() throws SQLException {
    if (!lastQueriedStatus.isStillRunning()) {
      return lastQueriedStatus;
    }
    lastQueriedStatus = snowflakeConnection.getQueryStatus(queryID);
    return lastQueriedStatus;
  }

  @Override
  public String getQueryID() throws SQLException {
    return queryID;
  }

  // =========================================================================
  // Lifecycle
  // =========================================================================

  @Override
  public void close() throws SQLException {
    synchronized (this) {
      if (closed) {
        return;
      }
      closed = true;
      if (delegate != null) {
        delegate.close();
      }
    }
    statement.removeClosedResultSet(this);
    if (ownsStatement) {
      statement.close();
    }
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed;
  }

  @Override
  public Statement getStatement() throws SQLException {
    checkClosed();
    return statement;
  }

  // =========================================================================
  // Wrapper
  // =========================================================================

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    if (iface.isInstance(this)) {
      return iface.cast(this);
    }
    materialize();
    return delegate.unwrap(iface);
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    if (iface.isInstance(this)) {
      return true;
    }
    // Avoid materializing for introspection; the delegate is always SnowflakeResultSetImpl.
    return iface.isAssignableFrom(SnowflakeResultSetImpl.class);
  }

  // =========================================================================
  // ResultSet — all methods delegate
  // =========================================================================

  @Override
  public boolean next() throws SQLException {
    materialize();
    return delegate.next();
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    materialize();
    SnowflakeResultSetMetaDataImpl metaData =
        delegate.getMetaData().unwrap(SnowflakeResultSetMetaDataImpl.class);
    // Non-mutating async view: original async query ID and ASYNC query type
    // (which suppresses catalog/schema/table names).
    return new DecoratedSnowflakeResultSetMetaDataImpl(
        SnowflakeResultSetMetaDataImpl.toAsync(metaData, queryID),
        snowflakeConnection.getTelemetry());
  }

  @Override
  public boolean wasNull() throws SQLException {
    materialize();
    return delegate.wasNull();
  }

  @Override
  public String getString(int columnIndex) throws SQLException {
    materialize();
    return delegate.getString(columnIndex);
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    materialize();
    return delegate.getBoolean(columnIndex);
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    materialize();
    return delegate.getByte(columnIndex);
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    materialize();
    return delegate.getShort(columnIndex);
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    materialize();
    return delegate.getInt(columnIndex);
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    materialize();
    return delegate.getLong(columnIndex);
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    materialize();
    return delegate.getFloat(columnIndex);
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    materialize();
    return delegate.getDouble(columnIndex);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex, int scale) throws SQLException {
    materialize();
    return delegate.getBigDecimal(columnIndex, scale);
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    materialize();
    return delegate.getBytes(columnIndex);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    materialize();
    return delegate.getDate(columnIndex);
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    materialize();
    return delegate.getTime(columnIndex);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    materialize();
    return delegate.getTimestamp(columnIndex);
  }

  @Override
  public InputStream getAsciiStream(int columnIndex) throws SQLException {
    materialize();
    return delegate.getAsciiStream(columnIndex);
  }

  @Override
  public InputStream getUnicodeStream(int columnIndex) throws SQLException {
    materialize();
    return delegate.getUnicodeStream(columnIndex);
  }

  @Override
  public InputStream getBinaryStream(int columnIndex) throws SQLException {
    materialize();
    return delegate.getBinaryStream(columnIndex);
  }

  @Override
  public String getString(String columnLabel) throws SQLException {
    materialize();
    return delegate.getString(columnLabel);
  }

  @Override
  public boolean getBoolean(String columnLabel) throws SQLException {
    materialize();
    return delegate.getBoolean(columnLabel);
  }

  @Override
  public byte getByte(String columnLabel) throws SQLException {
    materialize();
    return delegate.getByte(columnLabel);
  }

  @Override
  public short getShort(String columnLabel) throws SQLException {
    materialize();
    return delegate.getShort(columnLabel);
  }

  @Override
  public int getInt(String columnLabel) throws SQLException {
    materialize();
    return delegate.getInt(columnLabel);
  }

  @Override
  public long getLong(String columnLabel) throws SQLException {
    materialize();
    return delegate.getLong(columnLabel);
  }

  @Override
  public float getFloat(String columnLabel) throws SQLException {
    materialize();
    return delegate.getFloat(columnLabel);
  }

  @Override
  public double getDouble(String columnLabel) throws SQLException {
    materialize();
    return delegate.getDouble(columnLabel);
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel, int scale) throws SQLException {
    materialize();
    return delegate.getBigDecimal(columnLabel, scale);
  }

  @Override
  public byte[] getBytes(String columnLabel) throws SQLException {
    materialize();
    return delegate.getBytes(columnLabel);
  }

  @Override
  public Date getDate(String columnLabel) throws SQLException {
    materialize();
    return delegate.getDate(columnLabel);
  }

  @Override
  public Time getTime(String columnLabel) throws SQLException {
    materialize();
    return delegate.getTime(columnLabel);
  }

  @Override
  public Timestamp getTimestamp(String columnLabel) throws SQLException {
    materialize();
    return delegate.getTimestamp(columnLabel);
  }

  @Override
  public InputStream getAsciiStream(String columnLabel) throws SQLException {
    materialize();
    return delegate.getAsciiStream(columnLabel);
  }

  @Override
  public InputStream getUnicodeStream(String columnLabel) throws SQLException {
    materialize();
    return delegate.getUnicodeStream(columnLabel);
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) throws SQLException {
    materialize();
    return delegate.getBinaryStream(columnLabel);
  }

  @Override
  public SQLWarning getWarnings() throws SQLException {
    materialize();
    return delegate.getWarnings();
  }

  @Override
  public void clearWarnings() throws SQLException {
    materialize();
    delegate.clearWarnings();
  }

  @Override
  public String getCursorName() throws SQLException {
    materialize();
    return delegate.getCursorName();
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    materialize();
    return delegate.getObject(columnIndex);
  }

  @Override
  public Object getObject(String columnLabel) throws SQLException {
    materialize();
    return delegate.getObject(columnLabel);
  }

  @Override
  public int findColumn(String columnLabel) throws SQLException {
    materialize();
    return delegate.findColumn(columnLabel);
  }

  @Override
  public Reader getCharacterStream(int columnIndex) throws SQLException {
    materialize();
    return delegate.getCharacterStream(columnIndex);
  }

  @Override
  public Reader getCharacterStream(String columnLabel) throws SQLException {
    materialize();
    return delegate.getCharacterStream(columnLabel);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    materialize();
    return delegate.getBigDecimal(columnIndex);
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel) throws SQLException {
    materialize();
    return delegate.getBigDecimal(columnLabel);
  }

  @Override
  public boolean isBeforeFirst() throws SQLException {
    checkClosed();
    if (delegate == null) {
      return true;
    }
    return delegate.isBeforeFirst();
  }

  @Override
  public boolean isAfterLast() throws SQLException {
    checkClosed();
    if (delegate == null) {
      return false;
    }
    return delegate.isAfterLast();
  }

  @Override
  public boolean isFirst() throws SQLException {
    checkClosed();
    if (delegate == null) {
      return false;
    }
    return delegate.isFirst();
  }

  @Override
  public boolean isLast() throws SQLException {
    materialize();
    return delegate.isLast();
  }

  @Override
  public void beforeFirst() throws SQLException {
    materialize();
    delegate.beforeFirst();
  }

  @Override
  public void afterLast() throws SQLException {
    materialize();
    delegate.afterLast();
  }

  @Override
  public boolean first() throws SQLException {
    materialize();
    return delegate.first();
  }

  @Override
  public boolean last() throws SQLException {
    materialize();
    return delegate.last();
  }

  @Override
  public int getRow() throws SQLException {
    checkClosed();
    if (delegate == null) {
      return 0;
    }
    return delegate.getRow();
  }

  @Override
  public boolean absolute(int row) throws SQLException {
    materialize();
    return delegate.absolute(row);
  }

  @Override
  public boolean relative(int rows) throws SQLException {
    materialize();
    return delegate.relative(rows);
  }

  @Override
  public boolean previous() throws SQLException {
    materialize();
    return delegate.previous();
  }

  @Override
  public void setFetchDirection(int direction) throws SQLException {
    materialize();
    delegate.setFetchDirection(direction);
  }

  @Override
  public int getFetchDirection() throws SQLException {
    materialize();
    return delegate.getFetchDirection();
  }

  @Override
  public void setFetchSize(int rows) throws SQLException {
    materialize();
    delegate.setFetchSize(rows);
  }

  @Override
  public int getFetchSize() throws SQLException {
    materialize();
    return delegate.getFetchSize();
  }

  @Override
  public int getType() throws SQLException {
    materialize();
    return delegate.getType();
  }

  @Override
  public int getConcurrency() throws SQLException {
    materialize();
    return delegate.getConcurrency();
  }

  @Override
  public boolean rowUpdated() throws SQLException {
    materialize();
    return delegate.rowUpdated();
  }

  @Override
  public boolean rowInserted() throws SQLException {
    materialize();
    return delegate.rowInserted();
  }

  @Override
  public boolean rowDeleted() throws SQLException {
    materialize();
    return delegate.rowDeleted();
  }

  @Override
  public void updateNull(int columnIndex) throws SQLException {
    materialize();
    delegate.updateNull(columnIndex);
  }

  @Override
  public void updateBoolean(int columnIndex, boolean x) throws SQLException {
    materialize();
    delegate.updateBoolean(columnIndex, x);
  }

  @Override
  public void updateByte(int columnIndex, byte x) throws SQLException {
    materialize();
    delegate.updateByte(columnIndex, x);
  }

  @Override
  public void updateShort(int columnIndex, short x) throws SQLException {
    materialize();
    delegate.updateShort(columnIndex, x);
  }

  @Override
  public void updateInt(int columnIndex, int x) throws SQLException {
    materialize();
    delegate.updateInt(columnIndex, x);
  }

  @Override
  public void updateLong(int columnIndex, long x) throws SQLException {
    materialize();
    delegate.updateLong(columnIndex, x);
  }

  @Override
  public void updateFloat(int columnIndex, float x) throws SQLException {
    materialize();
    delegate.updateFloat(columnIndex, x);
  }

  @Override
  public void updateDouble(int columnIndex, double x) throws SQLException {
    materialize();
    delegate.updateDouble(columnIndex, x);
  }

  @Override
  public void updateBigDecimal(int columnIndex, BigDecimal x) throws SQLException {
    materialize();
    delegate.updateBigDecimal(columnIndex, x);
  }

  @Override
  public void updateString(int columnIndex, String x) throws SQLException {
    materialize();
    delegate.updateString(columnIndex, x);
  }

  @Override
  public void updateBytes(int columnIndex, byte[] x) throws SQLException {
    materialize();
    delegate.updateBytes(columnIndex, x);
  }

  @Override
  public void updateDate(int columnIndex, Date x) throws SQLException {
    materialize();
    delegate.updateDate(columnIndex, x);
  }

  @Override
  public void updateTime(int columnIndex, Time x) throws SQLException {
    materialize();
    delegate.updateTime(columnIndex, x);
  }

  @Override
  public void updateTimestamp(int columnIndex, Timestamp x) throws SQLException {
    materialize();
    delegate.updateTimestamp(columnIndex, x);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, int length) throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnIndex, x, length);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, int length) throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnIndex, x, length);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, int length) throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateObject(int columnIndex, Object x, int scaleOrLength) throws SQLException {
    materialize();
    delegate.updateObject(columnIndex, x, scaleOrLength);
  }

  @Override
  public void updateObject(int columnIndex, Object x) throws SQLException {
    materialize();
    delegate.updateObject(columnIndex, x);
  }

  @Override
  public void updateNull(String columnLabel) throws SQLException {
    materialize();
    delegate.updateNull(columnLabel);
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) throws SQLException {
    materialize();
    delegate.updateBoolean(columnLabel, x);
  }

  @Override
  public void updateByte(String columnLabel, byte x) throws SQLException {
    materialize();
    delegate.updateByte(columnLabel, x);
  }

  @Override
  public void updateShort(String columnLabel, short x) throws SQLException {
    materialize();
    delegate.updateShort(columnLabel, x);
  }

  @Override
  public void updateInt(String columnLabel, int x) throws SQLException {
    materialize();
    delegate.updateInt(columnLabel, x);
  }

  @Override
  public void updateLong(String columnLabel, long x) throws SQLException {
    materialize();
    delegate.updateLong(columnLabel, x);
  }

  @Override
  public void updateFloat(String columnLabel, float x) throws SQLException {
    materialize();
    delegate.updateFloat(columnLabel, x);
  }

  @Override
  public void updateDouble(String columnLabel, double x) throws SQLException {
    materialize();
    delegate.updateDouble(columnLabel, x);
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) throws SQLException {
    materialize();
    delegate.updateBigDecimal(columnLabel, x);
  }

  @Override
  public void updateString(String columnLabel, String x) throws SQLException {
    materialize();
    delegate.updateString(columnLabel, x);
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) throws SQLException {
    materialize();
    delegate.updateBytes(columnLabel, x);
  }

  @Override
  public void updateDate(String columnLabel, Date x) throws SQLException {
    materialize();
    delegate.updateDate(columnLabel, x);
  }

  @Override
  public void updateTime(String columnLabel, Time x) throws SQLException {
    materialize();
    delegate.updateTime(columnLabel, x);
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) throws SQLException {
    materialize();
    delegate.updateTimestamp(columnLabel, x);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnLabel, x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length)
      throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnLabel, x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length)
      throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) throws SQLException {
    materialize();
    delegate.updateObject(columnLabel, x, scaleOrLength);
  }

  @Override
  public void updateObject(String columnLabel, Object x) throws SQLException {
    materialize();
    delegate.updateObject(columnLabel, x);
  }

  @Override
  public void insertRow() throws SQLException {
    materialize();
    delegate.insertRow();
  }

  @Override
  public void updateRow() throws SQLException {
    materialize();
    delegate.updateRow();
  }

  @Override
  public void deleteRow() throws SQLException {
    materialize();
    delegate.deleteRow();
  }

  @Override
  public void refreshRow() throws SQLException {
    materialize();
    delegate.refreshRow();
  }

  @Override
  public void cancelRowUpdates() throws SQLException {
    materialize();
    delegate.cancelRowUpdates();
  }

  @Override
  public void moveToInsertRow() throws SQLException {
    materialize();
    delegate.moveToInsertRow();
  }

  @Override
  public void moveToCurrentRow() throws SQLException {
    materialize();
    delegate.moveToCurrentRow();
  }

  @Override
  public Object getObject(int columnIndex, Map<String, Class<?>> map) throws SQLException {
    materialize();
    return delegate.getObject(columnIndex, map);
  }

  @Override
  public Ref getRef(int columnIndex) throws SQLException {
    materialize();
    return delegate.getRef(columnIndex);
  }

  @Override
  public Blob getBlob(int columnIndex) throws SQLException {
    materialize();
    return delegate.getBlob(columnIndex);
  }

  @Override
  public Clob getClob(int columnIndex) throws SQLException {
    materialize();
    return delegate.getClob(columnIndex);
  }

  @Override
  public Array getArray(int columnIndex) throws SQLException {
    materialize();
    return delegate.getArray(columnIndex);
  }

  @Override
  public Object getObject(String columnLabel, Map<String, Class<?>> map) throws SQLException {
    materialize();
    return delegate.getObject(columnLabel, map);
  }

  @Override
  public Ref getRef(String columnLabel) throws SQLException {
    materialize();
    return delegate.getRef(columnLabel);
  }

  @Override
  public Blob getBlob(String columnLabel) throws SQLException {
    materialize();
    return delegate.getBlob(columnLabel);
  }

  @Override
  public Clob getClob(String columnLabel) throws SQLException {
    materialize();
    return delegate.getClob(columnLabel);
  }

  @Override
  public Array getArray(String columnLabel) throws SQLException {
    materialize();
    return delegate.getArray(columnLabel);
  }

  @Override
  public Date getDate(int columnIndex, Calendar cal) throws SQLException {
    materialize();
    return delegate.getDate(columnIndex, cal);
  }

  @Override
  public Date getDate(String columnLabel, Calendar cal) throws SQLException {
    materialize();
    return delegate.getDate(columnLabel, cal);
  }

  @Override
  public Time getTime(int columnIndex, Calendar cal) throws SQLException {
    materialize();
    return delegate.getTime(columnIndex, cal);
  }

  @Override
  public Time getTime(String columnLabel, Calendar cal) throws SQLException {
    materialize();
    return delegate.getTime(columnLabel, cal);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, Calendar cal) throws SQLException {
    materialize();
    return delegate.getTimestamp(columnIndex, cal);
  }

  @Override
  public Timestamp getTimestamp(String columnLabel, Calendar cal) throws SQLException {
    materialize();
    return delegate.getTimestamp(columnLabel, cal);
  }

  @Override
  public URL getURL(int columnIndex) throws SQLException {
    materialize();
    return delegate.getURL(columnIndex);
  }

  @Override
  public URL getURL(String columnLabel) throws SQLException {
    materialize();
    return delegate.getURL(columnLabel);
  }

  @Override
  public void updateRef(int columnIndex, Ref x) throws SQLException {
    materialize();
    delegate.updateRef(columnIndex, x);
  }

  @Override
  public void updateRef(String columnLabel, Ref x) throws SQLException {
    materialize();
    delegate.updateRef(columnLabel, x);
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) throws SQLException {
    materialize();
    delegate.updateBlob(columnIndex, x);
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) throws SQLException {
    materialize();
    delegate.updateBlob(columnLabel, x);
  }

  @Override
  public void updateClob(int columnIndex, Clob x) throws SQLException {
    materialize();
    delegate.updateClob(columnIndex, x);
  }

  @Override
  public void updateClob(String columnLabel, Clob x) throws SQLException {
    materialize();
    delegate.updateClob(columnLabel, x);
  }

  @Override
  public void updateArray(int columnIndex, Array x) throws SQLException {
    materialize();
    delegate.updateArray(columnIndex, x);
  }

  @Override
  public void updateArray(String columnLabel, Array x) throws SQLException {
    materialize();
    delegate.updateArray(columnLabel, x);
  }

  @Override
  public RowId getRowId(int columnIndex) throws SQLException {
    materialize();
    return delegate.getRowId(columnIndex);
  }

  @Override
  public RowId getRowId(String columnLabel) throws SQLException {
    materialize();
    return delegate.getRowId(columnLabel);
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) throws SQLException {
    materialize();
    delegate.updateRowId(columnIndex, x);
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) throws SQLException {
    materialize();
    delegate.updateRowId(columnLabel, x);
  }

  @Override
  public int getHoldability() throws SQLException {
    materialize();
    return delegate.getHoldability();
  }

  @Override
  public void updateNString(int columnIndex, String nString) throws SQLException {
    materialize();
    delegate.updateNString(columnIndex, nString);
  }

  @Override
  public void updateNString(String columnLabel, String nString) throws SQLException {
    materialize();
    delegate.updateNString(columnLabel, nString);
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) throws SQLException {
    materialize();
    delegate.updateNClob(columnIndex, nClob);
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) throws SQLException {
    materialize();
    delegate.updateNClob(columnLabel, nClob);
  }

  @Override
  public NClob getNClob(int columnIndex) throws SQLException {
    materialize();
    return delegate.getNClob(columnIndex);
  }

  @Override
  public NClob getNClob(String columnLabel) throws SQLException {
    materialize();
    return delegate.getNClob(columnLabel);
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) throws SQLException {
    materialize();
    return delegate.getSQLXML(columnIndex);
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) throws SQLException {
    materialize();
    return delegate.getSQLXML(columnLabel);
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) throws SQLException {
    materialize();
    delegate.updateSQLXML(columnIndex, xmlObject);
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) throws SQLException {
    materialize();
    delegate.updateSQLXML(columnLabel, xmlObject);
  }

  @Override
  public String getNString(int columnIndex) throws SQLException {
    materialize();
    return delegate.getNString(columnIndex);
  }

  @Override
  public String getNString(String columnLabel) throws SQLException {
    materialize();
    return delegate.getNString(columnLabel);
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) throws SQLException {
    materialize();
    return delegate.getNCharacterStream(columnIndex);
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) throws SQLException {
    materialize();
    return delegate.getNCharacterStream(columnLabel);
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) throws SQLException {
    materialize();
    delegate.updateNCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    materialize();
    delegate.updateNCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, long length) throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnIndex, x, length);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, long length) throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnIndex, x, length);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, long length) throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, long length)
      throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnLabel, x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length)
      throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnLabel, x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length)
      throws SQLException {
    materialize();
    delegate.updateBlob(columnIndex, inputStream, length);
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length)
      throws SQLException {
    materialize();
    delegate.updateBlob(columnLabel, inputStream, length);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) throws SQLException {
    materialize();
    delegate.updateClob(columnIndex, reader, length);
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) throws SQLException {
    materialize();
    delegate.updateClob(columnLabel, reader, length);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) throws SQLException {
    materialize();
    delegate.updateNClob(columnIndex, reader, length);
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) throws SQLException {
    materialize();
    delegate.updateNClob(columnLabel, reader, length);
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) throws SQLException {
    materialize();
    delegate.updateNCharacterStream(columnIndex, x);
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) throws SQLException {
    materialize();
    delegate.updateNCharacterStream(columnLabel, reader);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x) throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnIndex, x);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x) throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnIndex, x);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x) throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnIndex, x);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x) throws SQLException {
    materialize();
    delegate.updateAsciiStream(columnLabel, x);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) throws SQLException {
    materialize();
    delegate.updateBinaryStream(columnLabel, x);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) throws SQLException {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) throws SQLException {
    materialize();
    delegate.updateBlob(columnIndex, inputStream);
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) throws SQLException {
    materialize();
    delegate.updateBlob(columnLabel, inputStream);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) throws SQLException {
    materialize();
    delegate.updateClob(columnIndex, reader);
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) throws SQLException {
    materialize();
    delegate.updateClob(columnLabel, reader);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) throws SQLException {
    materialize();
    delegate.updateNClob(columnIndex, reader);
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) throws SQLException {
    materialize();
    delegate.updateNClob(columnLabel, reader);
  }

  @Override
  public <T> T getObject(int columnIndex, Class<T> type) throws SQLException {
    materialize();
    return delegate.getObject(columnIndex, type);
  }

  @Override
  public <T> T getObject(String columnLabel, Class<T> type) throws SQLException {
    materialize();
    return delegate.getObject(columnLabel, type);
  }

  // =========================================================================
  // SnowflakeResultSet
  // =========================================================================

  @Override
  public List<SnowflakeResultSetSerializable> getResultSetSerializables(long maxSizeInBytes)
      throws SQLException {
    materialize();
    return delegate.getResultSetSerializables(maxSizeInBytes);
  }

  @Override
  public <T> T[] getArray(int columnIndex, Class<T> type) throws SQLException {
    materialize();
    return delegate.getArray(columnIndex, type);
  }

  @Override
  public <T> List<T> getList(int columnIndex, Class<T> type) throws SQLException {
    materialize();
    return delegate.getList(columnIndex, type);
  }

  @Override
  public <T> Map<String, T> getMap(int columnIndex, Class<T> type) throws SQLException {
    materialize();
    return delegate.getMap(columnIndex, type);
  }

  // =========================================================================
  // Internal
  // =========================================================================

  private void materialize() throws SQLException {
    checkClosed();
    if (delegate == null) {
      if (!lastQueriedStatus.isSuccess()) {
        lastQueriedStatus = waiter.waitForCompletion();
      }
      synchronized (this) {
        checkClosed();
        if (delegate == null) {
          delegate = snowflakeConnection.createResultSetFromSfqid(queryID, statement);
        }
      }
    }
  }

  private void checkClosed() throws SQLException {
    if (closed) {
      throw new SQLException("ResultSet is closed");
    }
  }
}
