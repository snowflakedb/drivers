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
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.resultset.metadata.DecoratedSnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.codegen.NoTelemetry;

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
@JdbcBoundary
public class SnowflakeAsyncResultSetImpl implements InternalAsyncResultSet {

  private final String queryID;
  private final InternalSnowflakeConnection snowflakeConnection;
  private final SnowflakeStatementImpl statement;
  private final boolean ownsStatement;
  private final QueryResultWaiter waiter;

  // Raw impl (not the generated decorator): async is itself decorated by
  // DecoratedSnowflakeAsyncResultSetImpl, so holding a decorated child here would double-wrap.
  private volatile SnowflakeResultSetImpl delegate;
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
  public QueryStatus getStatus() {
    if (!lastQueriedStatus.isStillRunning()) {
      return lastQueriedStatus;
    }
    lastQueriedStatus = snowflakeConnection.getQueryStatus(queryID);
    return lastQueriedStatus;
  }

  @Override
  public String getQueryID() {
    return queryID;
  }

  // =========================================================================
  // Lifecycle
  // =========================================================================

  @Override
  public void close() {
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
  public boolean isClosed() {
    return closed;
  }

  @Override
  public Statement getStatement() {
    checkClosed();
    return statement == null
        ? null
        : Decorators.statement(statement, Decorators.telemetryOf(statement));
  }

  // =========================================================================
  // Wrapper
  // =========================================================================

  @Override
  public <T> T unwrap(Class<T> iface) {
    if (iface.isInstance(this)) {
      return iface.cast(this);
    }
    materialize();
    return delegate.unwrap(iface);
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) {
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
  @NoTelemetry
  public boolean next() {
    materialize();
    return delegate.next();
  }

  @Override
  public ResultSetMetaData getMetaData() {
    materialize();
    SnowflakeResultSetMetaDataImpl metaData = delegate.getMetaDataImpl();
    // Non-mutating async view: original async query ID and ASYNC query type
    // (which suppresses catalog/schema/table names).
    return new DecoratedSnowflakeResultSetMetaDataImpl(
        SnowflakeResultSetMetaDataImpl.toAsync(metaData, queryID),
        snowflakeConnection.getTelemetry());
  }

  @Override
  @NoTelemetry
  public boolean wasNull() {
    materialize();
    return delegate.wasNull();
  }

  @Override
  @NoTelemetry
  public String getString(int columnIndex) {
    materialize();
    return delegate.getString(columnIndex);
  }

  @Override
  @NoTelemetry
  public boolean getBoolean(int columnIndex) {
    materialize();
    return delegate.getBoolean(columnIndex);
  }

  @Override
  @NoTelemetry
  public byte getByte(int columnIndex) {
    materialize();
    return delegate.getByte(columnIndex);
  }

  @Override
  @NoTelemetry
  public short getShort(int columnIndex) {
    materialize();
    return delegate.getShort(columnIndex);
  }

  @Override
  @NoTelemetry
  public int getInt(int columnIndex) {
    materialize();
    return delegate.getInt(columnIndex);
  }

  @Override
  @NoTelemetry
  public long getLong(int columnIndex) {
    materialize();
    return delegate.getLong(columnIndex);
  }

  @Override
  @NoTelemetry
  public float getFloat(int columnIndex) {
    materialize();
    return delegate.getFloat(columnIndex);
  }

  @Override
  @NoTelemetry
  public double getDouble(int columnIndex) {
    materialize();
    return delegate.getDouble(columnIndex);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(int columnIndex, int scale) {
    materialize();
    return delegate.getBigDecimal(columnIndex, scale);
  }

  @Override
  @NoTelemetry
  public byte[] getBytes(int columnIndex) {
    materialize();
    return delegate.getBytes(columnIndex);
  }

  @Override
  @NoTelemetry
  public Date getDate(int columnIndex) {
    materialize();
    return delegate.getDate(columnIndex);
  }

  @Override
  @NoTelemetry
  public Time getTime(int columnIndex) {
    materialize();
    return delegate.getTime(columnIndex);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(int columnIndex) {
    materialize();
    return delegate.getTimestamp(columnIndex);
  }

  @Override
  public InputStream getAsciiStream(int columnIndex) {
    materialize();
    return delegate.getAsciiStream(columnIndex);
  }

  @Override
  public InputStream getUnicodeStream(int columnIndex) {
    materialize();
    return delegate.getUnicodeStream(columnIndex);
  }

  @Override
  public InputStream getBinaryStream(int columnIndex) {
    materialize();
    return delegate.getBinaryStream(columnIndex);
  }

  @Override
  @NoTelemetry
  public String getString(String columnLabel) {
    materialize();
    return delegate.getString(columnLabel);
  }

  @Override
  @NoTelemetry
  public boolean getBoolean(String columnLabel) {
    materialize();
    return delegate.getBoolean(columnLabel);
  }

  @Override
  @NoTelemetry
  public byte getByte(String columnLabel) {
    materialize();
    return delegate.getByte(columnLabel);
  }

  @Override
  @NoTelemetry
  public short getShort(String columnLabel) {
    materialize();
    return delegate.getShort(columnLabel);
  }

  @Override
  @NoTelemetry
  public int getInt(String columnLabel) {
    materialize();
    return delegate.getInt(columnLabel);
  }

  @Override
  @NoTelemetry
  public long getLong(String columnLabel) {
    materialize();
    return delegate.getLong(columnLabel);
  }

  @Override
  @NoTelemetry
  public float getFloat(String columnLabel) {
    materialize();
    return delegate.getFloat(columnLabel);
  }

  @Override
  @NoTelemetry
  public double getDouble(String columnLabel) {
    materialize();
    return delegate.getDouble(columnLabel);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(String columnLabel, int scale) {
    materialize();
    return delegate.getBigDecimal(columnLabel, scale);
  }

  @Override
  @NoTelemetry
  public byte[] getBytes(String columnLabel) {
    materialize();
    return delegate.getBytes(columnLabel);
  }

  @Override
  @NoTelemetry
  public Date getDate(String columnLabel) {
    materialize();
    return delegate.getDate(columnLabel);
  }

  @Override
  @NoTelemetry
  public Time getTime(String columnLabel) {
    materialize();
    return delegate.getTime(columnLabel);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(String columnLabel) {
    materialize();
    return delegate.getTimestamp(columnLabel);
  }

  @Override
  public InputStream getAsciiStream(String columnLabel) {
    materialize();
    return delegate.getAsciiStream(columnLabel);
  }

  @Override
  public InputStream getUnicodeStream(String columnLabel) {
    materialize();
    return delegate.getUnicodeStream(columnLabel);
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) {
    materialize();
    return delegate.getBinaryStream(columnLabel);
  }

  @Override
  public SQLWarning getWarnings() {
    materialize();
    return delegate.getWarnings();
  }

  @Override
  public void clearWarnings() {
    materialize();
    delegate.clearWarnings();
  }

  @Override
  public String getCursorName() {
    materialize();
    return delegate.getCursorName();
  }

  @Override
  @NoTelemetry
  public Object getObject(int columnIndex) {
    materialize();
    return delegate.getObject(columnIndex);
  }

  @Override
  @NoTelemetry
  public Object getObject(String columnLabel) {
    materialize();
    return delegate.getObject(columnLabel);
  }

  @Override
  @NoTelemetry
  public int findColumn(String columnLabel) {
    materialize();
    return delegate.findColumn(columnLabel);
  }

  @Override
  @NoTelemetry
  public Reader getCharacterStream(int columnIndex) {
    materialize();
    return delegate.getCharacterStream(columnIndex);
  }

  @Override
  @NoTelemetry
  public Reader getCharacterStream(String columnLabel) {
    materialize();
    return delegate.getCharacterStream(columnLabel);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(int columnIndex) {
    materialize();
    return delegate.getBigDecimal(columnIndex);
  }

  @Override
  @NoTelemetry
  public BigDecimal getBigDecimal(String columnLabel) {
    materialize();
    return delegate.getBigDecimal(columnLabel);
  }

  @Override
  @NoTelemetry
  public boolean isBeforeFirst() {
    checkClosed();
    if (delegate == null) {
      return true;
    }
    return delegate.isBeforeFirst();
  }

  @Override
  @NoTelemetry
  public boolean isAfterLast() {
    checkClosed();
    if (delegate == null) {
      return false;
    }
    return delegate.isAfterLast();
  }

  @Override
  @NoTelemetry
  public boolean isFirst() {
    checkClosed();
    if (delegate == null) {
      return false;
    }
    return delegate.isFirst();
  }

  @Override
  @NoTelemetry
  public boolean isLast() {
    materialize();
    return delegate.isLast();
  }

  @Override
  public void beforeFirst() {
    materialize();
    delegate.beforeFirst();
  }

  @Override
  public void afterLast() {
    materialize();
    delegate.afterLast();
  }

  @Override
  public boolean first() {
    materialize();
    return delegate.first();
  }

  @Override
  public boolean last() {
    materialize();
    return delegate.last();
  }

  @Override
  @NoTelemetry
  public int getRow() {
    checkClosed();
    if (delegate == null) {
      return 0;
    }
    return delegate.getRow();
  }

  @Override
  public boolean absolute(int row) {
    materialize();
    return delegate.absolute(row);
  }

  @Override
  public boolean relative(int rows) {
    materialize();
    return delegate.relative(rows);
  }

  @Override
  public boolean previous() {
    materialize();
    return delegate.previous();
  }

  @Override
  public void setFetchDirection(int direction) {
    materialize();
    delegate.setFetchDirection(direction);
  }

  @Override
  public int getFetchDirection() {
    materialize();
    return delegate.getFetchDirection();
  }

  @Override
  public void setFetchSize(int rows) {
    materialize();
    delegate.setFetchSize(rows);
  }

  @Override
  public int getFetchSize() {
    materialize();
    return delegate.getFetchSize();
  }

  @Override
  public int getType() {
    materialize();
    return delegate.getType();
  }

  @Override
  public int getConcurrency() {
    materialize();
    return delegate.getConcurrency();
  }

  @Override
  public boolean rowUpdated() {
    materialize();
    return delegate.rowUpdated();
  }

  @Override
  public boolean rowInserted() {
    materialize();
    return delegate.rowInserted();
  }

  @Override
  public boolean rowDeleted() {
    materialize();
    return delegate.rowDeleted();
  }

  @Override
  public void updateNull(int columnIndex) {
    materialize();
    delegate.updateNull(columnIndex);
  }

  @Override
  public void updateBoolean(int columnIndex, boolean x) {
    materialize();
    delegate.updateBoolean(columnIndex, x);
  }

  @Override
  public void updateByte(int columnIndex, byte x) {
    materialize();
    delegate.updateByte(columnIndex, x);
  }

  @Override
  public void updateShort(int columnIndex, short x) {
    materialize();
    delegate.updateShort(columnIndex, x);
  }

  @Override
  public void updateInt(int columnIndex, int x) {
    materialize();
    delegate.updateInt(columnIndex, x);
  }

  @Override
  public void updateLong(int columnIndex, long x) {
    materialize();
    delegate.updateLong(columnIndex, x);
  }

  @Override
  public void updateFloat(int columnIndex, float x) {
    materialize();
    delegate.updateFloat(columnIndex, x);
  }

  @Override
  public void updateDouble(int columnIndex, double x) {
    materialize();
    delegate.updateDouble(columnIndex, x);
  }

  @Override
  public void updateBigDecimal(int columnIndex, BigDecimal x) {
    materialize();
    delegate.updateBigDecimal(columnIndex, x);
  }

  @Override
  public void updateString(int columnIndex, String x) {
    materialize();
    delegate.updateString(columnIndex, x);
  }

  @Override
  public void updateBytes(int columnIndex, byte[] x) {
    materialize();
    delegate.updateBytes(columnIndex, x);
  }

  @Override
  public void updateDate(int columnIndex, Date x) {
    materialize();
    delegate.updateDate(columnIndex, x);
  }

  @Override
  public void updateTime(int columnIndex, Time x) {
    materialize();
    delegate.updateTime(columnIndex, x);
  }

  @Override
  public void updateTimestamp(int columnIndex, Timestamp x) {
    materialize();
    delegate.updateTimestamp(columnIndex, x);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, int length) {
    materialize();
    delegate.updateAsciiStream(columnIndex, x, length);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, int length) {
    materialize();
    delegate.updateBinaryStream(columnIndex, x, length);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, int length) {
    materialize();
    delegate.updateCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateObject(int columnIndex, Object x, int scaleOrLength) {
    materialize();
    delegate.updateObject(columnIndex, x, scaleOrLength);
  }

  @Override
  public void updateObject(int columnIndex, Object x) {
    materialize();
    delegate.updateObject(columnIndex, x);
  }

  @Override
  public void updateNull(String columnLabel) {
    materialize();
    delegate.updateNull(columnLabel);
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) {
    materialize();
    delegate.updateBoolean(columnLabel, x);
  }

  @Override
  public void updateByte(String columnLabel, byte x) {
    materialize();
    delegate.updateByte(columnLabel, x);
  }

  @Override
  public void updateShort(String columnLabel, short x) {
    materialize();
    delegate.updateShort(columnLabel, x);
  }

  @Override
  public void updateInt(String columnLabel, int x) {
    materialize();
    delegate.updateInt(columnLabel, x);
  }

  @Override
  public void updateLong(String columnLabel, long x) {
    materialize();
    delegate.updateLong(columnLabel, x);
  }

  @Override
  public void updateFloat(String columnLabel, float x) {
    materialize();
    delegate.updateFloat(columnLabel, x);
  }

  @Override
  public void updateDouble(String columnLabel, double x) {
    materialize();
    delegate.updateDouble(columnLabel, x);
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) {
    materialize();
    delegate.updateBigDecimal(columnLabel, x);
  }

  @Override
  public void updateString(String columnLabel, String x) {
    materialize();
    delegate.updateString(columnLabel, x);
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) {
    materialize();
    delegate.updateBytes(columnLabel, x);
  }

  @Override
  public void updateDate(String columnLabel, Date x) {
    materialize();
    delegate.updateDate(columnLabel, x);
  }

  @Override
  public void updateTime(String columnLabel, Time x) {
    materialize();
    delegate.updateTime(columnLabel, x);
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) {
    materialize();
    delegate.updateTimestamp(columnLabel, x);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) {
    materialize();
    delegate.updateAsciiStream(columnLabel, x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length) {
    materialize();
    delegate.updateBinaryStream(columnLabel, x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length) {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) {
    materialize();
    delegate.updateObject(columnLabel, x, scaleOrLength);
  }

  @Override
  public void updateObject(String columnLabel, Object x) {
    materialize();
    delegate.updateObject(columnLabel, x);
  }

  @Override
  public void insertRow() {
    materialize();
    delegate.insertRow();
  }

  @Override
  public void updateRow() {
    materialize();
    delegate.updateRow();
  }

  @Override
  public void deleteRow() {
    materialize();
    delegate.deleteRow();
  }

  @Override
  public void refreshRow() {
    materialize();
    delegate.refreshRow();
  }

  @Override
  public void cancelRowUpdates() {
    materialize();
    delegate.cancelRowUpdates();
  }

  @Override
  public void moveToInsertRow() {
    materialize();
    delegate.moveToInsertRow();
  }

  @Override
  public void moveToCurrentRow() {
    materialize();
    delegate.moveToCurrentRow();
  }

  @Override
  @NoTelemetry
  public Object getObject(int columnIndex, Map<String, Class<?>> map) {
    materialize();
    return delegate.getObject(columnIndex, map);
  }

  @Override
  public Ref getRef(int columnIndex) {
    materialize();
    return delegate.getRef(columnIndex);
  }

  @Override
  public Blob getBlob(int columnIndex) {
    materialize();
    return delegate.getBlob(columnIndex);
  }

  @Override
  public Clob getClob(int columnIndex) {
    materialize();
    return delegate.getClob(columnIndex);
  }

  @Override
  public Array getArray(int columnIndex) {
    materialize();
    return delegate.getArray(columnIndex);
  }

  @Override
  @NoTelemetry
  public Object getObject(String columnLabel, Map<String, Class<?>> map) {
    materialize();
    return delegate.getObject(columnLabel, map);
  }

  @Override
  public Ref getRef(String columnLabel) {
    materialize();
    return delegate.getRef(columnLabel);
  }

  @Override
  public Blob getBlob(String columnLabel) {
    materialize();
    return delegate.getBlob(columnLabel);
  }

  @Override
  public Clob getClob(String columnLabel) {
    materialize();
    return delegate.getClob(columnLabel);
  }

  @Override
  public Array getArray(String columnLabel) {
    materialize();
    return delegate.getArray(columnLabel);
  }

  @Override
  @NoTelemetry
  public Date getDate(int columnIndex, Calendar cal) {
    materialize();
    return delegate.getDate(columnIndex, cal);
  }

  @Override
  @NoTelemetry
  public Date getDate(String columnLabel, Calendar cal) {
    materialize();
    return delegate.getDate(columnLabel, cal);
  }

  @Override
  @NoTelemetry
  public Time getTime(int columnIndex, Calendar cal) {
    materialize();
    return delegate.getTime(columnIndex, cal);
  }

  @Override
  @NoTelemetry
  public Time getTime(String columnLabel, Calendar cal) {
    materialize();
    return delegate.getTime(columnLabel, cal);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(int columnIndex, Calendar cal) {
    materialize();
    return delegate.getTimestamp(columnIndex, cal);
  }

  @Override
  @NoTelemetry
  public Timestamp getTimestamp(String columnLabel, Calendar cal) {
    materialize();
    return delegate.getTimestamp(columnLabel, cal);
  }

  @Override
  public URL getURL(int columnIndex) {
    materialize();
    return delegate.getURL(columnIndex);
  }

  @Override
  public URL getURL(String columnLabel) {
    materialize();
    return delegate.getURL(columnLabel);
  }

  @Override
  public void updateRef(int columnIndex, Ref x) {
    materialize();
    delegate.updateRef(columnIndex, x);
  }

  @Override
  public void updateRef(String columnLabel, Ref x) {
    materialize();
    delegate.updateRef(columnLabel, x);
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) {
    materialize();
    delegate.updateBlob(columnIndex, x);
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) {
    materialize();
    delegate.updateBlob(columnLabel, x);
  }

  @Override
  public void updateClob(int columnIndex, Clob x) {
    materialize();
    delegate.updateClob(columnIndex, x);
  }

  @Override
  public void updateClob(String columnLabel, Clob x) {
    materialize();
    delegate.updateClob(columnLabel, x);
  }

  @Override
  public void updateArray(int columnIndex, Array x) {
    materialize();
    delegate.updateArray(columnIndex, x);
  }

  @Override
  public void updateArray(String columnLabel, Array x) {
    materialize();
    delegate.updateArray(columnLabel, x);
  }

  @Override
  public RowId getRowId(int columnIndex) {
    materialize();
    return delegate.getRowId(columnIndex);
  }

  @Override
  public RowId getRowId(String columnLabel) {
    materialize();
    return delegate.getRowId(columnLabel);
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) {
    materialize();
    delegate.updateRowId(columnIndex, x);
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) {
    materialize();
    delegate.updateRowId(columnLabel, x);
  }

  @Override
  public int getHoldability() {
    materialize();
    return delegate.getHoldability();
  }

  @Override
  public void updateNString(int columnIndex, String nString) {
    materialize();
    delegate.updateNString(columnIndex, nString);
  }

  @Override
  public void updateNString(String columnLabel, String nString) {
    materialize();
    delegate.updateNString(columnLabel, nString);
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) {
    materialize();
    delegate.updateNClob(columnIndex, nClob);
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) {
    materialize();
    delegate.updateNClob(columnLabel, nClob);
  }

  @Override
  public NClob getNClob(int columnIndex) {
    materialize();
    return delegate.getNClob(columnIndex);
  }

  @Override
  public NClob getNClob(String columnLabel) {
    materialize();
    return delegate.getNClob(columnLabel);
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) {
    materialize();
    return delegate.getSQLXML(columnIndex);
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) {
    materialize();
    return delegate.getSQLXML(columnLabel);
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) {
    materialize();
    delegate.updateSQLXML(columnIndex, xmlObject);
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) {
    materialize();
    delegate.updateSQLXML(columnLabel, xmlObject);
  }

  @Override
  public String getNString(int columnIndex) {
    materialize();
    return delegate.getNString(columnIndex);
  }

  @Override
  public String getNString(String columnLabel) {
    materialize();
    return delegate.getNString(columnLabel);
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) {
    materialize();
    return delegate.getNCharacterStream(columnIndex);
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) {
    materialize();
    return delegate.getNCharacterStream(columnLabel);
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) {
    materialize();
    delegate.updateNCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length) {
    materialize();
    delegate.updateNCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, long length) {
    materialize();
    delegate.updateAsciiStream(columnIndex, x, length);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, long length) {
    materialize();
    delegate.updateBinaryStream(columnIndex, x, length);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, long length) {
    materialize();
    delegate.updateCharacterStream(columnIndex, x, length);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, long length) {
    materialize();
    delegate.updateAsciiStream(columnLabel, x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length) {
    materialize();
    delegate.updateBinaryStream(columnLabel, x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length) {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader, length);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length) {
    materialize();
    delegate.updateBlob(columnIndex, inputStream, length);
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length) {
    materialize();
    delegate.updateBlob(columnLabel, inputStream, length);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) {
    materialize();
    delegate.updateClob(columnIndex, reader, length);
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) {
    materialize();
    delegate.updateClob(columnLabel, reader, length);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) {
    materialize();
    delegate.updateNClob(columnIndex, reader, length);
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) {
    materialize();
    delegate.updateNClob(columnLabel, reader, length);
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) {
    materialize();
    delegate.updateNCharacterStream(columnIndex, x);
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) {
    materialize();
    delegate.updateNCharacterStream(columnLabel, reader);
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x) {
    materialize();
    delegate.updateAsciiStream(columnIndex, x);
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x) {
    materialize();
    delegate.updateBinaryStream(columnIndex, x);
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x) {
    materialize();
    delegate.updateCharacterStream(columnIndex, x);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x) {
    materialize();
    delegate.updateAsciiStream(columnLabel, x);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) {
    materialize();
    delegate.updateBinaryStream(columnLabel, x);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) {
    materialize();
    delegate.updateCharacterStream(columnLabel, reader);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) {
    materialize();
    delegate.updateBlob(columnIndex, inputStream);
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) {
    materialize();
    delegate.updateBlob(columnLabel, inputStream);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) {
    materialize();
    delegate.updateClob(columnIndex, reader);
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) {
    materialize();
    delegate.updateClob(columnLabel, reader);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) {
    materialize();
    delegate.updateNClob(columnIndex, reader);
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) {
    materialize();
    delegate.updateNClob(columnLabel, reader);
  }

  @Override
  @NoTelemetry
  public <T> T getObject(int columnIndex, Class<T> type) {
    materialize();
    return delegate.getObject(columnIndex, type);
  }

  @Override
  @NoTelemetry
  public <T> T getObject(String columnLabel, Class<T> type) {
    materialize();
    return delegate.getObject(columnLabel, type);
  }

  // =========================================================================
  // SnowflakeResultSet
  // =========================================================================

  @Override
  public List<SnowflakeResultSetSerializable> getResultSetSerializables(long maxSizeInBytes) {
    materialize();
    return delegate.getResultSetSerializables(maxSizeInBytes);
  }

  @Override
  public <T> T[] getArray(int columnIndex, Class<T> type) {
    materialize();
    return delegate.getArray(columnIndex, type);
  }

  @Override
  public <T> List<T> getList(int columnIndex, Class<T> type) {
    materialize();
    return delegate.getList(columnIndex, type);
  }

  @Override
  public <T> Map<String, T> getMap(int columnIndex, Class<T> type) {
    materialize();
    return delegate.getMap(columnIndex, type);
  }

  // =========================================================================
  // Internal
  // =========================================================================

  private void materialize() {
    checkClosed();
    if (delegate == null) {
      if (!lastQueriedStatus.isSuccess()) {
        lastQueriedStatus = waiter.waitForCompletion();
      }
      synchronized (this) {
        checkClosed();
        if (delegate == null) {
          delegate =
              (SnowflakeResultSetImpl)
                  snowflakeConnection.createResultSetFromSfqid(queryID, statement);
        }
      }
    }
  }

  private void checkClosed() {
    if (closed) {
      throw new SFSQLException("ResultSet is closed");
    }
  }
}
