package net.snowflake.client.internal.api.implementation.statement;

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
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.codegen.JdbcBoundary;

/**
 * The {@link Statement#getGeneratedKeys()} result, method-for-method equal to snowflake-jdbc 4.x
 * {@code SnowflakeResultSetV1.EmptyResultSet}: no rows, getters return type defaults ({@code
 * getString} is {@code ""}), navigation and update calls are no-ops, and metadata, statement, and
 * {@code unwrap} return null. Every call except {@code next}, {@code close}, and {@code isClosed}
 * throws once closed.
 */
@JdbcBoundary
public class EmptyGeneratedKeysResultSet implements ResultSet {

  private volatile boolean closed;

  private void checkClosed() {
    if (closed) {
      throw new SFSQLException(ErrorCode.RESULTSET_ALREADY_CLOSED, "ResultSet is closed");
    }
  }

  @Override
  public boolean next() {
    return false;
  }

  @Override
  public void close() {
    closed = true;
  }

  @Override
  public boolean isClosed() {
    return closed;
  }

  @Override
  public boolean wasNull() {
    checkClosed();
    return false;
  }

  @Override
  public String getString(int columnIndex) {
    checkClosed();
    return "";
  }

  @Override
  public boolean getBoolean(int columnIndex) {
    checkClosed();
    return false;
  }

  @Override
  public byte getByte(int columnIndex) {
    checkClosed();
    return 0;
  }

  @Override
  public short getShort(int columnIndex) {
    checkClosed();
    return 0;
  }

  @Override
  public int getInt(int columnIndex) {
    checkClosed();
    return 0;
  }

  @Override
  public long getLong(int columnIndex) {
    checkClosed();
    return 0L;
  }

  @Override
  public float getFloat(int columnIndex) {
    checkClosed();
    return 0f;
  }

  @Override
  public double getDouble(int columnIndex) {
    checkClosed();
    return 0d;
  }

  @Deprecated
  @Override
  public BigDecimal getBigDecimal(int columnIndex, int scale) {
    checkClosed();
    return null;
  }

  @Override
  public byte[] getBytes(int columnIndex) {
    checkClosed();
    return new byte[0];
  }

  @Override
  public Date getDate(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Time getTime(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public InputStream getAsciiStream(int columnIndex) {
    checkClosed();
    return null;
  }

  @Deprecated
  @Override
  public InputStream getUnicodeStream(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public InputStream getBinaryStream(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public String getString(String columnLabel) {
    checkClosed();
    return "";
  }

  @Override
  public boolean getBoolean(String columnLabel) {
    checkClosed();
    return false;
  }

  @Override
  public byte getByte(String columnLabel) {
    checkClosed();
    return 0;
  }

  @Override
  public short getShort(String columnLabel) {
    checkClosed();
    return 0;
  }

  @Override
  public int getInt(String columnLabel) {
    checkClosed();
    return 0;
  }

  @Override
  public long getLong(String columnLabel) {
    checkClosed();
    return 0L;
  }

  @Override
  public float getFloat(String columnLabel) {
    checkClosed();
    return 0f;
  }

  @Override
  public double getDouble(String columnLabel) {
    checkClosed();
    return 0d;
  }

  @Deprecated
  @Override
  public BigDecimal getBigDecimal(String columnLabel, int scale) {
    checkClosed();
    return null;
  }

  @Override
  public byte[] getBytes(String columnLabel) {
    checkClosed();
    return new byte[0];
  }

  @Override
  public Date getDate(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Time getTime(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Timestamp getTimestamp(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public InputStream getAsciiStream(String columnLabel) {
    checkClosed();
    return null;
  }

  @Deprecated
  @Override
  public InputStream getUnicodeStream(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public SQLWarning getWarnings() {
    checkClosed();
    return null;
  }

  @Override
  public void clearWarnings() {
    checkClosed();
  }

  @Override
  public String getCursorName() {
    checkClosed();
    return null;
  }

  @Override
  public ResultSetMetaData getMetaData() {
    checkClosed();
    return null;
  }

  @Override
  public Object getObject(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Object getObject(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public int findColumn(String columnLabel) {
    checkClosed();
    return 0;
  }

  @Override
  public Reader getCharacterStream(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Reader getCharacterStream(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public boolean isBeforeFirst() {
    checkClosed();
    return false;
  }

  @Override
  public boolean isAfterLast() {
    checkClosed();
    return false;
  }

  @Override
  public boolean isFirst() {
    checkClosed();
    return false;
  }

  @Override
  public boolean isLast() {
    checkClosed();
    return false;
  }

  @Override
  public void beforeFirst() {
    checkClosed();
  }

  @Override
  public void afterLast() {
    checkClosed();
  }

  @Override
  public boolean first() {
    checkClosed();
    return false;
  }

  @Override
  public boolean last() {
    checkClosed();
    return false;
  }

  @Override
  public int getRow() {
    checkClosed();
    return 0;
  }

  @Override
  public boolean absolute(int row) {
    checkClosed();
    return false;
  }

  @Override
  public boolean relative(int rows) {
    checkClosed();
    return false;
  }

  @Override
  public boolean previous() {
    checkClosed();
    return false;
  }

  @Override
  public void setFetchDirection(int direction) {
    checkClosed();
  }

  @Override
  public int getFetchDirection() {
    checkClosed();
    return 0;
  }

  @Override
  public void setFetchSize(int rows) {
    checkClosed();
  }

  @Override
  public int getFetchSize() {
    checkClosed();
    return 0;
  }

  @Override
  public int getType() {
    checkClosed();
    return 0;
  }

  @Override
  public int getConcurrency() {
    checkClosed();
    return 0;
  }

  @Override
  public boolean rowUpdated() {
    checkClosed();
    return false;
  }

  @Override
  public boolean rowInserted() {
    checkClosed();
    return false;
  }

  @Override
  public boolean rowDeleted() {
    checkClosed();
    return false;
  }

  @Override
  public void updateNull(int columnIndex) {
    checkClosed();
  }

  @Override
  public void updateBoolean(int columnIndex, boolean x) {
    checkClosed();
  }

  @Override
  public void updateByte(int columnIndex, byte x) {
    checkClosed();
  }

  @Override
  public void updateShort(int columnIndex, short x) {
    checkClosed();
  }

  @Override
  public void updateInt(int columnIndex, int x) {
    checkClosed();
  }

  @Override
  public void updateLong(int columnIndex, long x) {
    checkClosed();
  }

  @Override
  public void updateFloat(int columnIndex, float x) {
    checkClosed();
  }

  @Override
  public void updateDouble(int columnIndex, double x) {
    checkClosed();
  }

  @Override
  public void updateBigDecimal(int columnIndex, BigDecimal x) {
    checkClosed();
  }

  @Override
  public void updateString(int columnIndex, String x) {
    checkClosed();
  }

  @Override
  public void updateBytes(int columnIndex, byte[] x) {
    checkClosed();
  }

  @Override
  public void updateDate(int columnIndex, Date x) {
    checkClosed();
  }

  @Override
  public void updateTime(int columnIndex, Time x) {
    checkClosed();
  }

  @Override
  public void updateTimestamp(int columnIndex, Timestamp x) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, int length) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, int length) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, int length) {
    checkClosed();
  }

  @Override
  public void updateObject(int columnIndex, Object x, int scaleOrLength) {
    checkClosed();
  }

  @Override
  public void updateObject(int columnIndex, Object x) {
    checkClosed();
  }

  @Override
  public void updateNull(String columnLabel) {
    checkClosed();
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) {
    checkClosed();
  }

  @Override
  public void updateByte(String columnLabel, byte x) {
    checkClosed();
  }

  @Override
  public void updateShort(String columnLabel, short x) {
    checkClosed();
  }

  @Override
  public void updateInt(String columnLabel, int x) {
    checkClosed();
  }

  @Override
  public void updateLong(String columnLabel, long x) {
    checkClosed();
  }

  @Override
  public void updateFloat(String columnLabel, float x) {
    checkClosed();
  }

  @Override
  public void updateDouble(String columnLabel, double x) {
    checkClosed();
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) {
    checkClosed();
  }

  @Override
  public void updateString(String columnLabel, String x) {
    checkClosed();
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) {
    checkClosed();
  }

  @Override
  public void updateDate(String columnLabel, Date x) {
    checkClosed();
  }

  @Override
  public void updateTime(String columnLabel, Time x) {
    checkClosed();
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length) {
    checkClosed();
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) {
    checkClosed();
  }

  @Override
  public void updateObject(String columnLabel, Object x) {
    checkClosed();
  }

  @Override
  public void insertRow() {
    checkClosed();
  }

  @Override
  public void updateRow() {
    checkClosed();
  }

  @Override
  public void deleteRow() {
    checkClosed();
  }

  @Override
  public void refreshRow() {
    checkClosed();
  }

  @Override
  public void cancelRowUpdates() {
    checkClosed();
  }

  @Override
  public void moveToInsertRow() {
    checkClosed();
  }

  @Override
  public void moveToCurrentRow() {
    checkClosed();
  }

  @Override
  public Statement getStatement() {
    checkClosed();
    return null;
  }

  @Override
  public Object getObject(int columnIndex, Map<String, Class<?>> map) {
    checkClosed();
    return null;
  }

  @Override
  public Ref getRef(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Blob getBlob(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Clob getClob(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Array getArray(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Object getObject(String columnLabel, Map<String, Class<?>> map) {
    checkClosed();
    return null;
  }

  @Override
  public Ref getRef(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Blob getBlob(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Clob getClob(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Array getArray(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Date getDate(int columnIndex, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public Date getDate(String columnLabel, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public Time getTime(int columnIndex, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public Time getTime(String columnLabel, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public Timestamp getTimestamp(String columnLabel, Calendar cal) {
    checkClosed();
    return null;
  }

  @Override
  public URL getURL(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public URL getURL(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public void updateRef(int columnIndex, Ref x) {
    checkClosed();
  }

  @Override
  public void updateRef(String columnLabel, Ref x) {
    checkClosed();
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) {
    checkClosed();
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) {
    checkClosed();
  }

  @Override
  public void updateClob(int columnIndex, Clob x) {
    checkClosed();
  }

  @Override
  public void updateClob(String columnLabel, Clob x) {
    checkClosed();
  }

  @Override
  public void updateArray(int columnIndex, Array x) {
    checkClosed();
  }

  @Override
  public void updateArray(String columnLabel, Array x) {
    checkClosed();
  }

  @Override
  public RowId getRowId(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public RowId getRowId(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) {
    checkClosed();
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) {
    checkClosed();
  }

  @Override
  public int getHoldability() {
    checkClosed();
    return 0;
  }

  @Override
  public void updateNString(int columnIndex, String nString) {
    checkClosed();
  }

  @Override
  public void updateNString(String columnLabel, String nString) {
    checkClosed();
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) {
    checkClosed();
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) {
    checkClosed();
  }

  @Override
  public NClob getNClob(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public NClob getNClob(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) {
    checkClosed();
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) {
    checkClosed();
  }

  @Override
  public String getNString(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public String getNString(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) {
    checkClosed();
    return null;
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) {
    checkClosed();
    return null;
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) {
    checkClosed();
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x, long length) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x, long length) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x, long length) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, long length) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length) {
    checkClosed();
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length) {
    checkClosed();
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) {
    checkClosed();
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) {
    checkClosed();
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(int columnIndex, InputStream x) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(int columnIndex, InputStream x) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(int columnIndex, Reader x) {
    checkClosed();
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x) {
    checkClosed();
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) {
    checkClosed();
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) {
    checkClosed();
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) {
    checkClosed();
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) {
    checkClosed();
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) {
    checkClosed();
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) {
    checkClosed();
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) {
    checkClosed();
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) {
    checkClosed();
  }

  @Override
  public <T> T getObject(int columnIndex, Class<T> type) {
    checkClosed();
    return null;
  }

  @Override
  public <T> T getObject(String columnLabel, Class<T> type) {
    checkClosed();
    return null;
  }

  @Override
  public <T> T unwrap(Class<T> iface) {
    checkClosed();
    return null;
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) {
    checkClosed();
    return false;
  }
}
