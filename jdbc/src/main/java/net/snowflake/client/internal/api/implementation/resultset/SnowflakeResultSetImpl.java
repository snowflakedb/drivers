package net.snowflake.client.internal.api.implementation.resultset;

import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.math.RoundingMode;
import java.net.URL;
import java.sql.Array;
import java.sql.Blob;
import java.sql.Clob;
import java.sql.Date;
import java.sql.NClob;
import java.sql.Ref;
import java.sql.ResultSetMetaData;
import java.sql.RowId;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.List;
import java.util.Map;
import java.util.TimeZone;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.NotImplementedException;

@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
public class SnowflakeResultSetImpl implements InternalResultSet, DelegatingWrapper {

  private final SnowflakeStatementImpl statement;
  private final String queryId;
  private final RowReader rowReader;
  private final SnowflakeResultSetMetaDataImpl resultSetMetaData;
  private final boolean ownsStatement;

  private boolean closed = false;
  private int fetchSize = 0;
  private int fetchDirection = FETCH_FORWARD;

  @Override
  public boolean next() throws SQLException {
    if (closed) {
      return false;
    }
    return rowReader.next();
  }

  @Override
  public void close() throws SQLException {
    if (closed) {
      return;
    }
    try {
      rowReader.close();
    } finally {
      closed = true;
      statement.removeClosedResultSet(this);
    }

    if (ownsStatement) {
      try {
        if (!statement.isClosed()) {
          statement.close();
        }
      } catch (SQLException ignored) {
        // closing the owning statement is best-effort
      }
    }
  }

  @Override
  public boolean wasNull() throws SQLException {
    checkClosed();
    return rowReader.wasNull();
  }

  @Override
  public String getString(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getString(columnIndex);
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getBoolean(columnIndex);
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getByte(columnIndex);
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getShort(columnIndex);
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getInt(columnIndex);
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getLong(columnIndex);
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getFloat(columnIndex);
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getDouble(columnIndex);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex, int scale) throws SQLException {
    BigDecimal value = getBigDecimal(columnIndex);
    if (value == null) {
      return null;
    }
    return value.setScale(scale, RoundingMode.HALF_UP);
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getBytes(columnIndex);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    checkClosed();
    // Mirrors snowflake-jdbc's SnowflakeBaseResultSet.getDate(int): JDBC_GET_DATE_USE_NULL_TIMEZONE
    // (default true) selects a null timezone (raw epoch-day date); when false the JVM default
    // timezone is used, which the converter shifts only if JDBC_FORMAT_DATE_WITH_TIMEZONE is set.
    TimeZone tz =
        rowReader.getConversionContext().isGetDateUseNullTimezone() ? null : TimeZone.getDefault();
    return getDate(columnIndex, tz);
  }

  /**
   * Shared DATE materialization mirroring snowflake-jdbc's {@code SFArrowResultSet.getDate(int,
   * TimeZone)}: the caller timezone and the runtime {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} flag are
   * threaded into the converter, which applies the session-vs-caller timezone shift only when both
   * are present and the flag is set.
   */
  private Date getDate(int columnIndex, TimeZone tz) throws SQLException {
    checkClosed();
    return rowReader.getDate(columnIndex, tz);
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getTime(columnIndex);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getTimestamp(columnIndex);
  }

  @Override
  public InputStream getAsciiStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getAsciiStream not supported");
  }

  @Override
  public InputStream getUnicodeStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getUnicodeStream not supported");
  }

  @Override
  public InputStream getBinaryStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBinaryStream not supported");
  }

  // String-based column access
  @Override
  public String getString(String columnLabel) throws SQLException {
    return getString(findColumn(columnLabel));
  }

  @Override
  public boolean getBoolean(String columnLabel) throws SQLException {
    return getBoolean(findColumn(columnLabel));
  }

  @Override
  public byte getByte(String columnLabel) throws SQLException {
    return getByte(findColumn(columnLabel));
  }

  @Override
  public short getShort(String columnLabel) throws SQLException {
    return getShort(findColumn(columnLabel));
  }

  @Override
  public int getInt(String columnLabel) throws SQLException {
    return getInt(findColumn(columnLabel));
  }

  @Override
  public long getLong(String columnLabel) throws SQLException {
    return getLong(findColumn(columnLabel));
  }

  @Override
  public float getFloat(String columnLabel) throws SQLException {
    return getFloat(findColumn(columnLabel));
  }

  @Override
  public double getDouble(String columnLabel) throws SQLException {
    return getDouble(findColumn(columnLabel));
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel, int scale) throws SQLException {
    return getBigDecimal(findColumn(columnLabel), scale);
  }

  @Override
  public byte[] getBytes(String columnLabel) throws SQLException {
    return getBytes(findColumn(columnLabel));
  }

  @Override
  public Date getDate(String columnLabel) throws SQLException {
    return getDate(findColumn(columnLabel));
  }

  @Override
  public Time getTime(String columnLabel) throws SQLException {
    return getTime(findColumn(columnLabel));
  }

  @Override
  public Timestamp getTimestamp(String columnLabel) throws SQLException {
    return getTimestamp(findColumn(columnLabel));
  }

  @Override
  public InputStream getAsciiStream(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getAsciiStream not supported");
  }

  @Override
  public InputStream getUnicodeStream(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getUnicodeStream not supported");
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getAsciiStream not supported");
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
  public String getCursorName() throws SQLException {
    throw new SQLFeatureNotSupportedException("getCursorName not supported");
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    checkClosed();
    return resultSetMetaData;
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getObject(columnIndex);
  }

  @Override
  public Object getObject(String columnLabel) throws SQLException {
    return getObject(findColumn(columnLabel));
  }

  @Override
  public int findColumn(String columnLabel) throws SQLException {
    // TODO(SNOW-3740748): in SnowflakeResultSetMetaDataImpl::getColumnIndex session parameter
    //  "isResultColumnCaseInsensitive" is respect during the search, should we respect it here?

    checkClosed();
    for (int i = 0; i < resultSetMetaData.getColumnCount(); i++) {
      if (resultSetMetaData.getColumnNames().get(i).equalsIgnoreCase(columnLabel)) {
        return i + 1; // JDBC columns are 1-based
      }
    }
    throw new SQLException("Column not found: " + columnLabel);
  }

  @Override
  public Reader getCharacterStream(int columnIndex) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Reader getCharacterStream(String columnLabel) throws SQLException {
    return getCharacterStream(findColumn(columnLabel));
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    checkClosed();
    return rowReader.getBigDecimal(columnIndex);
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel) throws SQLException {
    return getBigDecimal(findColumn(columnLabel));
  }

  @Override
  public boolean isBeforeFirst() throws SQLException {
    checkClosed();
    return rowReader.isBeforeFirst();
  }

  @Override
  public boolean isAfterLast() throws SQLException {
    checkClosed();
    return rowReader.isAfterLast();
  }

  @Override
  public boolean isFirst() throws SQLException {
    checkClosed();
    return rowReader.isFirst();
  }

  @Override
  public boolean isLast() throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public void beforeFirst() throws SQLException {
    throw new SQLFeatureNotSupportedException("beforeFirst not supported");
  }

  @Override
  public void afterLast() throws SQLException {
    throw new SQLFeatureNotSupportedException("afterLast not supported");
  }

  @Override
  public boolean first() throws SQLException {
    throw new SQLFeatureNotSupportedException("first not supported (forward-only)");
  }

  @Override
  public boolean last() throws SQLException {
    throw new SQLFeatureNotSupportedException("last not supported (forward-only)");
  }

  @Override
  public int getRow() throws SQLException {
    checkClosed();
    int currentRow = rowReader.getCurrentRow();
    if (currentRow < 0 || rowReader.isAfterLast()) {
      return 0;
    }
    return currentRow + 1; // JDBC rows are 1-based
  }

  @Override
  public boolean absolute(int row) throws SQLException {
    throw new SQLFeatureNotSupportedException("absolute not supported (forward-only)");
  }

  @Override
  public boolean relative(int rows) throws SQLException {
    throw new SQLFeatureNotSupportedException("relative not supported (forward-only)");
  }

  @Override
  public boolean previous() throws SQLException {
    throw new SQLFeatureNotSupportedException("previous not supported (forward-only)");
  }

  @Override
  public void setFetchDirection(int direction) throws SQLException {
    checkClosed();
    if (direction != FETCH_FORWARD) {
      throw new SQLFeatureNotSupportedException("Only FETCH_FORWARD supported");
    }
    this.fetchDirection = direction;
  }

  @Override
  public int getFetchDirection() throws SQLException {
    checkClosed();
    return fetchDirection;
  }

  @Override
  public void setFetchSize(int rows) throws SQLException {
    checkClosed();
    if (rows < 0) {
      throw new SQLException("Fetch size must be >= 0");
    }
    this.fetchSize = rows;
  }

  @Override
  public int getFetchSize() throws SQLException {
    checkClosed();
    return fetchSize;
  }

  @Override
  public int getType() throws SQLException {
    return TYPE_FORWARD_ONLY;
  }

  @Override
  public int getConcurrency() throws SQLException {
    return CONCUR_READ_ONLY;
  }

  // Update methods (not supported)
  @Override
  public boolean rowUpdated() throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public boolean rowInserted() throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public boolean rowDeleted() throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNull(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBoolean(int columnIndex, boolean x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateByte(int columnIndex, byte x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateShort(int columnIndex, short x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateInt(int columnIndex, int x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateLong(int columnIndex, long x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateFloat(int columnIndex, float x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDouble(int columnIndex, double x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBigDecimal(int columnIndex, BigDecimal x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateString(int columnIndex, String x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBytes(int columnIndex, byte[] x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDate(int columnIndex, Date x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTime(int columnIndex, Time x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTimestamp(int columnIndex, Timestamp x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(int columnIndex, Object x, int scaleOrLength) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(int columnIndex, Object x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  // String-based update methods
  @Override
  public void updateNull(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateByte(String columnLabel, byte x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateShort(String columnLabel, short x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateInt(String columnLabel, int x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateLong(String columnLabel, long x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateFloat(String columnLabel, float x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDouble(String columnLabel, double x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateString(String columnLabel, String x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateDate(String columnLabel, Date x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTime(String columnLabel, Time x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateObject(String columnLabel, Object x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void insertRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("insertRow not supported");
  }

  @Override
  public void updateRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("updateRow not supported");
  }

  @Override
  public void deleteRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("deleteRow not supported");
  }

  @Override
  public void refreshRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("refreshRow not supported");
  }

  @Override
  public void cancelRowUpdates() throws SQLException {
    checkClosed();
  }

  @Override
  public void moveToInsertRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("moveToInsertRow not supported");
  }

  @Override
  public void moveToCurrentRow() throws SQLException {
    throw new SQLFeatureNotSupportedException("moveToCurrentRow not supported");
  }

  @Override
  public Statement getStatement() throws SQLException {
    checkClosed();
    return statement;
  }

  @Override
  public Object getObject(int columnIndex, Map<String, Class<?>> map) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getClob not supported");
  }

  @Override
  public Array getArray(int columnIndex) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Object getObject(String columnLabel, Map<String, Class<?>> map) throws SQLException {
    throw new SQLFeatureNotSupportedException("getObject not supported");
  }

  @Override
  public Ref getRef(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRef not supported");
  }

  @Override
  public Blob getBlob(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getBlob not supported");
  }

  @Override
  public Clob getClob(String columnLabel) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public Array getArray(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public Date getDate(int columnIndex, Calendar cal) throws SQLException {
    return getDate(columnIndex, cal == null ? null : cal.getTimeZone());
  }

  @Override
  public Date getDate(String columnLabel, Calendar cal) throws SQLException {
    return getDate(findColumn(columnLabel), cal);
  }

  @Override
  public Time getTime(int columnIndex, Calendar cal) throws SQLException {
    return getTime(columnIndex);
  }

  @Override
  public Time getTime(String columnLabel, Calendar cal) throws SQLException {
    return getTime(findColumn(columnLabel), cal);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, Calendar cal) throws SQLException {
    checkClosed();
    // Mirrors snowflake-jdbc's SnowflakeBaseResultSet.getTimestamp(int, Calendar): pass the
    // Calendar's timezone to the converter. Only TIMESTAMP_NTZ consumes it (honor-client-TZ
    // re-anchoring); LTZ/TZ ignore it. Note getTime(int, Calendar) intentionally drops the Calendar
    // to match legacy.
    return rowReader.getTimestamp(columnIndex, cal == null ? null : cal.getTimeZone());
  }

  @Override
  public Timestamp getTimestamp(String columnLabel, Calendar cal) throws SQLException {
    return getTimestamp(findColumn(columnLabel), cal);
  }

  @Override
  public URL getURL(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public URL getURL(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getURL not supported");
  }

  @Override
  public void updateRef(int columnIndex, Ref x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRef(String columnLabel, Ref x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Clob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Clob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateArray(int columnIndex, Array x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateArray(String columnLabel, Array x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public RowId getRowId(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public RowId getRowId(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public int getHoldability() throws SQLException {
    return CLOSE_CURSORS_AT_COMMIT;
  }

  @Override
  public boolean isClosed() throws SQLException {
    return closed;
  }

  @Override
  public void updateNString(int columnIndex, String nString) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNString(String columnLabel, String nString) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public NClob getNClob(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public NClob getNClob(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public String getNString(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public String getNString(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNString not supported");
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public <T> T getObject(int columnIndex, Class<T> type) throws SQLException {
    if (type == String.class) {
      return type.cast(getString(columnIndex));
    } else if (type == Integer.class) {
      return type.cast(getInt(columnIndex));
    } else if (type == Long.class) {
      return type.cast(getLong(columnIndex));
    } else if (type == Double.class) {
      return type.cast(getDouble(columnIndex));
    } else if (type == Boolean.class) {
      return type.cast(getBoolean(columnIndex));
    } else if (type == BigDecimal.class) {
      return type.cast(getBigDecimal(columnIndex));
    } else if (type == Date.class) {
      return type.cast(getDate(columnIndex));
    } else if (type == Time.class) {
      return type.cast(getTime(columnIndex));
    } else if (type == Timestamp.class) {
      return type.cast(getTimestamp(columnIndex));
    }
    throw new SQLFeatureNotSupportedException("Type not supported: " + type.getName());
  }

  @Override
  public <T> T getObject(String columnLabel, Class<T> type) throws SQLException {
    return getObject(findColumn(columnLabel), type);
  }

  /**
   * Transfers ownership of the {@link RowReader} to the caller. This result set is marked closed
   * and unregistered from the statement, but the reader itself is NOT closed - the caller assumes
   * responsibility for its lifecycle.
   */
  RowReader detachRowReader() {
    closed = true;
    statement.removeClosedResultSet(this);
    return rowReader;
  }

  private void checkClosed() throws SQLException {
    if (closed) {
      throw new SQLException("ResultSet is closed");
    }
  }

  @Override
  public String getQueryID() throws SQLException {
    return queryId;
  }

  @Override
  public List<SnowflakeResultSetSerializable> getResultSetSerializables(long maxSizeInBytes)
      throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public <T> T[] getArray(int columnIndex, Class<T> type) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public <T> List<T> getList(int columnIndex, Class<T> type) throws SQLException {
    throw new NotImplementedException();
  }

  @Override
  public <T> Map<String, T> getMap(int columnIndex, Class<T> type) throws SQLException {
    throw new NotImplementedException();
  }
}
