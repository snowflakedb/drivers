package com.snowflake.jdbc;

import com.google.protobuf.ByteString;
import com.snowflake.unicore.protobuf_gen.DatabaseDriverV1.ExecuteResult;
import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.math.RoundingMode;
import java.net.URL;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
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
import java.sql.SQLFeatureNotSupportedException;
import java.sql.SQLWarning;
import java.sql.SQLXML;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import java.util.Calendar;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.cursor.ArrowBatchManager;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import net.snowflake.client.internal.core.arrow.cursor.CursorState;
import net.snowflake.client.internal.core.arrow.cursor.SchemaState;
import net.snowflake.client.jdbc.SFException;
import org.apache.arrow.c.ArrowArrayStream;
import org.apache.arrow.c.Data;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.VectorSchemaRoot;

/**
 * Snowflake JDBC ResultSet implementation
 *
 * <p>This is a stub implementation that provides the basic JDBC ResultSet interface.
 */
public class SnowflakeResultSet implements ResultSet {

  private final SnowflakeStatement statement;
  private final CursorState cursor = new CursorState();
  private final SchemaState schema;
  private final ArrowResources resources;
  private final ArrowBatchManager batchManager;
  private boolean closed = false;
  private int fetchSize = 0;
  private int fetchDirection = FETCH_FORWARD;

  public SnowflakeResultSet(SnowflakeStatement statement, ExecuteResult result)
      throws SQLException {
    this.statement = statement;
    ByteString streamPointerBytes = result.getStream().getValue();
    // TODO Check how will this behave on AIX (Big Endian)
    long pointer =
        ByteBuffer.wrap(streamPointerBytes.toByteArray()).order(ByteOrder.LITTLE_ENDIAN).getLong();
    ArrowArrayStream stream = ArrowArrayStream.wrap(pointer);
    RootAllocator allocator = new RootAllocator();
    ArrowResources resources =
        new ArrowResources(stream, allocator, Data.importArrayStream(allocator, stream));
    this.resources = resources;
    this.schema = new SchemaState(resources.getActiveRoot());
    this.batchManager = new ArrowBatchManager(cursor, resources, schema);
  }

  @Override
  public boolean next() throws SQLException {
    if (closed) {
      return false;
    }
    boolean hasNext = fetchNextRow();
    if (!hasNext) {
      cursor.setAfterLast();
      return false;
    }
    cursor.incrementRow();
    return true;
  }

  @Override
  public void close() throws SQLException {
    if (closed) {
      return;
    }
    try {
      resources.closeAll();
    } finally {
      closed = true;
      resetStateAfterClose();
    }
  }

  @Override
  public boolean wasNull() throws SQLException {
    checkClosed();
    return cursor.wasNull();
  }

  @Override
  public String getString(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toString);
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toBoolean);
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toByte);
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toShort);
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toInt);
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toLong);
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toFloat);
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toDouble);
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
    return convertColumn(columnIndex, ArrowVectorConverter::toBytes);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException(
        "getDate not supported"); // TODO: Will be handled later
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException(
        "getTime not supported"); // TODO: Will be handled later
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException(
        "getTimestamp not supported"); // TODO: Will be handled later
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
    return getAsciiStream(findColumn(columnLabel));
  }

  @Override
  public InputStream getUnicodeStream(String columnLabel) throws SQLException {
    return getUnicodeStream(findColumn(columnLabel));
  }

  @Override
  public InputStream getBinaryStream(String columnLabel) throws SQLException {
    return getBinaryStream(findColumn(columnLabel));
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
  public String getCursorName() throws SQLException {
    throw new SQLFeatureNotSupportedException("getCursorName not supported");
  }

  @Override
  public ResultSetMetaData getMetaData() throws SQLException {
    checkClosed();
    return new SnowflakeResultSetMetaData(schema.getColumnNames(), schema.getColumnTypes());
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toObject);
  }

  @Override
  public Object getObject(String columnLabel) throws SQLException {
    return getObject(findColumn(columnLabel));
  }

  @Override
  public int findColumn(String columnLabel) throws SQLException {
    checkClosed();
    String[] columnNames = schema.getColumnNames();
    for (int i = 0; i < columnNames.length; i++) {
      if (columnNames[i].equalsIgnoreCase(columnLabel)) {
        return i + 1; // JDBC columns are 1-based
      }
    }
    throw new SQLException("Column not found: " + columnLabel);
  }

  @Override
  public Reader getCharacterStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getCharacterStream not supported");
  }

  @Override
  public Reader getCharacterStream(String columnLabel) throws SQLException {
    return getCharacterStream(findColumn(columnLabel));
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toBigDecimal);
  }

  @Override
  public BigDecimal getBigDecimal(String columnLabel) throws SQLException {
    return getBigDecimal(findColumn(columnLabel));
  }

  @Override
  public boolean isBeforeFirst() throws SQLException {
    checkClosed();
    // After calling next() on an empty result: currentRow stays -1, but afterLast becomes true
    // Without the !isAfterLast() check, isBeforeFirst() would incorrectly return true for an
    // exhausted empty result set
    return cursor.getCurrentRow() < 0 && !cursor.isAfterLast();
  }

  @Override
  public boolean isAfterLast() throws SQLException {
    checkClosed();
    return cursor.isAfterLast();
  }

  @Override
  public boolean isFirst() throws SQLException {
    checkClosed();
    // Row 0 is the first row; also check afterLast to handle exhausted single-row result sets
    return cursor.getCurrentRow() == 0 && !cursor.isAfterLast();
  }

  @Override
  public boolean isLast() throws SQLException {
    throw new SQLFeatureNotSupportedException(
        "isLast not supported"); // TODO: See if we can handle it by backend metadata only
  }

  @Override
  public void beforeFirst() throws SQLException {
    throw new SQLFeatureNotSupportedException("beforeFirst not supported (forward-only)");
  }

  @Override
  public void afterLast() throws SQLException {
    throw new SQLFeatureNotSupportedException("afterLast not supported (forward-only)");
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
    if (cursor.getCurrentRow() < 0 || cursor.isAfterLast()) {
      return 0;
    }
    return cursor.getCurrentRow() + 1; // JDBC rows are 1-based
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
    return false;
  }

  @Override
  public boolean rowInserted() throws SQLException {
    return false;
  }

  @Override
  public boolean rowDeleted() throws SQLException {
    return false;
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
    updateNull(findColumn(columnLabel));
  }

  @Override
  public void updateBoolean(String columnLabel, boolean x) throws SQLException {
    updateBoolean(findColumn(columnLabel), x);
  }

  @Override
  public void updateByte(String columnLabel, byte x) throws SQLException {
    updateByte(findColumn(columnLabel), x);
  }

  @Override
  public void updateShort(String columnLabel, short x) throws SQLException {
    updateShort(findColumn(columnLabel), x);
  }

  @Override
  public void updateInt(String columnLabel, int x) throws SQLException {
    updateInt(findColumn(columnLabel), x);
  }

  @Override
  public void updateLong(String columnLabel, long x) throws SQLException {
    updateLong(findColumn(columnLabel), x);
  }

  @Override
  public void updateFloat(String columnLabel, float x) throws SQLException {
    updateFloat(findColumn(columnLabel), x);
  }

  @Override
  public void updateDouble(String columnLabel, double x) throws SQLException {
    updateDouble(findColumn(columnLabel), x);
  }

  @Override
  public void updateBigDecimal(String columnLabel, BigDecimal x) throws SQLException {
    updateBigDecimal(findColumn(columnLabel), x);
  }

  @Override
  public void updateString(String columnLabel, String x) throws SQLException {
    updateString(findColumn(columnLabel), x);
  }

  @Override
  public void updateBytes(String columnLabel, byte[] x) throws SQLException {
    updateBytes(findColumn(columnLabel), x);
  }

  @Override
  public void updateDate(String columnLabel, Date x) throws SQLException {
    updateDate(findColumn(columnLabel), x);
  }

  @Override
  public void updateTime(String columnLabel, Time x) throws SQLException {
    updateTime(findColumn(columnLabel), x);
  }

  @Override
  public void updateTimestamp(String columnLabel, Timestamp x) throws SQLException {
    updateTimestamp(findColumn(columnLabel), x);
  }

  @Override
  public void updateAsciiStream(String columnLabel, InputStream x, int length) throws SQLException {
    updateAsciiStream(findColumn(columnLabel), x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, int length)
      throws SQLException {
    updateBinaryStream(findColumn(columnLabel), x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, int length)
      throws SQLException {
    updateCharacterStream(findColumn(columnLabel), reader, length);
  }

  @Override
  public void updateObject(String columnLabel, Object x, int scaleOrLength) throws SQLException {
    updateObject(findColumn(columnLabel), x, scaleOrLength);
  }

  @Override
  public void updateObject(String columnLabel, Object x) throws SQLException {
    updateObject(findColumn(columnLabel), x);
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
    throw new SQLFeatureNotSupportedException("cancelRowUpdates not supported");
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
    return getObject(columnIndex);
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
    throw new SQLFeatureNotSupportedException("getArray not supported");
  }

  @Override
  public Object getObject(String columnLabel, Map<String, Class<?>> map) throws SQLException {
    return getObject(findColumn(columnLabel), map);
  }

  @Override
  public Ref getRef(String columnLabel) throws SQLException {
    return getRef(findColumn(columnLabel));
  }

  @Override
  public Blob getBlob(String columnLabel) throws SQLException {
    return getBlob(findColumn(columnLabel));
  }

  @Override
  public Clob getClob(String columnLabel) throws SQLException {
    return getClob(findColumn(columnLabel));
  }

  @Override
  public Array getArray(String columnLabel) throws SQLException {
    return getArray(findColumn(columnLabel));
  }

  @Override
  public Date getDate(int columnIndex, Calendar cal) throws SQLException {
    return getDate(columnIndex);
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
    return getTimestamp(columnIndex);
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
    return getURL(findColumn(columnLabel));
  }

  @Override
  public void updateRef(int columnIndex, Ref x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRef(String columnLabel, Ref x) throws SQLException {
    updateRef(findColumn(columnLabel), x);
  }

  @Override
  public void updateBlob(int columnIndex, Blob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, Blob x) throws SQLException {
    updateBlob(findColumn(columnLabel), x);
  }

  @Override
  public void updateClob(int columnIndex, Clob x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Clob x) throws SQLException {
    updateClob(findColumn(columnLabel), x);
  }

  @Override
  public void updateArray(int columnIndex, Array x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateArray(String columnLabel, Array x) throws SQLException {
    updateArray(findColumn(columnLabel), x);
  }

  @Override
  public RowId getRowId(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getRowId not supported");
  }

  @Override
  public RowId getRowId(String columnLabel) throws SQLException {
    return getRowId(findColumn(columnLabel));
  }

  @Override
  public void updateRowId(int columnIndex, RowId x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateRowId(String columnLabel, RowId x) throws SQLException {
    updateRowId(findColumn(columnLabel), x);
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
    updateNString(findColumn(columnLabel), nString);
  }

  @Override
  public void updateNClob(int columnIndex, NClob nClob) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, NClob nClob) throws SQLException {
    updateNClob(findColumn(columnLabel), nClob);
  }

  @Override
  public NClob getNClob(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNClob not supported");
  }

  @Override
  public NClob getNClob(String columnLabel) throws SQLException {
    return getNClob(findColumn(columnLabel));
  }

  @Override
  public SQLXML getSQLXML(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getSQLXML not supported");
  }

  @Override
  public SQLXML getSQLXML(String columnLabel) throws SQLException {
    return getSQLXML(findColumn(columnLabel));
  }

  @Override
  public void updateSQLXML(int columnIndex, SQLXML xmlObject) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateSQLXML(String columnLabel, SQLXML xmlObject) throws SQLException {
    updateSQLXML(findColumn(columnLabel), xmlObject);
  }

  @Override
  public String getNString(int columnIndex) throws SQLException {
    return getString(columnIndex);
  }

  @Override
  public String getNString(String columnLabel) throws SQLException {
    return getString(columnLabel);
  }

  @Override
  public Reader getNCharacterStream(int columnIndex) throws SQLException {
    throw new SQLFeatureNotSupportedException("getNCharacterStream not supported");
  }

  @Override
  public Reader getNCharacterStream(String columnLabel) throws SQLException {
    return getNCharacterStream(findColumn(columnLabel));
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    updateNCharacterStream(findColumn(columnLabel), reader, length);
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
    updateAsciiStream(findColumn(columnLabel), x, length);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x, long length)
      throws SQLException {
    updateBinaryStream(findColumn(columnLabel), x, length);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader, long length)
      throws SQLException {
    updateCharacterStream(findColumn(columnLabel), reader, length);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream, long length)
      throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream, long length)
      throws SQLException {
    updateBlob(findColumn(columnLabel), inputStream, length);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader, long length) throws SQLException {
    updateClob(findColumn(columnLabel), reader, length);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader, long length) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader, long length) throws SQLException {
    updateNClob(findColumn(columnLabel), reader, length);
  }

  @Override
  public void updateNCharacterStream(int columnIndex, Reader x) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNCharacterStream(String columnLabel, Reader reader) throws SQLException {
    updateNCharacterStream(findColumn(columnLabel), reader);
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
    updateAsciiStream(findColumn(columnLabel), x);
  }

  @Override
  public void updateBinaryStream(String columnLabel, InputStream x) throws SQLException {
    updateBinaryStream(findColumn(columnLabel), x);
  }

  @Override
  public void updateCharacterStream(String columnLabel, Reader reader) throws SQLException {
    updateCharacterStream(findColumn(columnLabel), reader);
  }

  @Override
  public void updateBlob(int columnIndex, InputStream inputStream) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateBlob(String columnLabel, InputStream inputStream) throws SQLException {
    updateBlob(findColumn(columnLabel), inputStream);
  }

  @Override
  public void updateClob(int columnIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateClob(String columnLabel, Reader reader) throws SQLException {
    updateClob(findColumn(columnLabel), reader);
  }

  @Override
  public void updateNClob(int columnIndex, Reader reader) throws SQLException {
    throw new SQLFeatureNotSupportedException("Updates not supported");
  }

  @Override
  public void updateNClob(String columnLabel, Reader reader) throws SQLException {
    updateNClob(findColumn(columnLabel), reader);
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
    if (closed) {
      throw new SQLException("ResultSet is closed");
    }
  }

  private void resetStateAfterClose() {
    resources.reset();
    schema.reset();
    cursor.reset();
  }

  private void checkRowPosition() throws SQLException {
    if (cursor.isAfterLast()) {
      throw new SQLException("After last row");
    }
    if (cursor.getCurrentRowInBatch() < 0) {
      throw new SQLException("Before first row");
    }
  }

  private void checkColumnIndex(int columnIndex) throws SQLException {
    int columnCount = schema.getColumnCount();
    if (columnIndex < 1 || columnIndex > columnCount) {
      throw new SQLException("Invalid column index: " + columnIndex);
    }
  }

  private interface ConverterFunction<T> {
    T convert(ArrowVectorConverter converter, int rowIndex) throws SFException;
  }

  private void validateColumnAccess(int columnIndex) throws SQLException {
    checkClosed();
    checkRowPosition();
    checkColumnIndex(columnIndex);
  }

  private <T> T convertColumn(int columnIndex, ConverterFunction<T> converterFunction)
      throws SQLException {
    validateColumnAccess(columnIndex);
    ArrowVectorConverter converter = getConverter(columnIndex);
    try {
      int rowIndex = cursor.getCurrentRowInBatch();
      T value = converterFunction.convert(converter, rowIndex);
      cursor.setWasNull(converter.isNull(rowIndex));
      return value;
    } catch (SFException e) {
      throw new SQLException(
          "Cannot convert column " + columnIndex + " using " + converterFunction, e);
    }
  }

  private ArrowVectorConverter getConverter(int columnIndex) throws SQLException {
    VectorSchemaRoot root = resources.getActiveRoot();
    return schema.getConverter(columnIndex, root);
  }

  private boolean fetchNextRow() throws SQLException {
    return batchManager.fetchNextRow();
  }

  /** Simple ResultSetMetaData implementation */
  private static class SnowflakeResultSetMetaData implements ResultSetMetaData {
    private final String[] columnNames;
    private final int[] columnTypes;

    public SnowflakeResultSetMetaData(String[] columnNames, int[] columnTypes) {
      this.columnNames = columnNames;
      this.columnTypes = columnTypes;
    }

    @Override
    public int getColumnCount() throws SQLException {
      return columnNames.length;
    }

    @Override
    public boolean isAutoIncrement(int column) throws SQLException {
      return false;
    }

    @Override
    public boolean isCaseSensitive(int column) throws SQLException {
      return false;
    }

    @Override
    public boolean isSearchable(int column) throws SQLException {
      return true;
    }

    @Override
    public boolean isCurrency(int column) throws SQLException {
      return false;
    }

    @Override
    public int isNullable(int column) throws SQLException {
      return columnNullable;
    }

    @Override
    public boolean isSigned(int column) throws SQLException {
      int type = getColumnType(column);
      return type == Types.INTEGER
          || type == Types.BIGINT
          || type == Types.SMALLINT
          || type == Types.TINYINT
          || type == Types.FLOAT
          || type == Types.DOUBLE
          || type == Types.DECIMAL
          || type == Types.NUMERIC;
    }

    @Override
    public int getColumnDisplaySize(int column) throws SQLException {
      return 255;
    }

    @Override
    public String getColumnLabel(int column) throws SQLException {
      checkColumnIndex(column);
      return columnNames[column - 1];
    }

    @Override
    public String getColumnName(int column) throws SQLException {
      return getColumnLabel(column);
    }

    @Override
    public String getSchemaName(int column) throws SQLException {
      return "PUBLIC";
    }

    @Override
    public int getPrecision(int column) throws SQLException {
      return 0;
    }

    @Override
    public int getScale(int column) throws SQLException {
      return 0;
    }

    @Override
    public String getTableName(int column) throws SQLException {
      return "STUB_TABLE";
    }

    @Override
    public String getCatalogName(int column) throws SQLException {
      return "STUB_DB";
    }

    @Override
    public int getColumnType(int column) throws SQLException {
      checkColumnIndex(column);
      return columnTypes[column - 1];
    }

    @Override
    public String getColumnTypeName(int column) throws SQLException {
      int type = getColumnType(column);
      switch (type) {
        case Types.INTEGER:
          return "INTEGER";
        case Types.VARCHAR:
          return "VARCHAR";
        case Types.DATE:
          return "DATE";
        case Types.TIMESTAMP:
          return "TIMESTAMP";
        default:
          return "VARCHAR";
      }
    }

    @Override
    public boolean isReadOnly(int column) throws SQLException {
      return true;
    }

    @Override
    public boolean isWritable(int column) throws SQLException {
      return false;
    }

    @Override
    public boolean isDefinitelyWritable(int column) throws SQLException {
      return false;
    }

    @Override
    public String getColumnClassName(int column) throws SQLException {
      int type = getColumnType(column);
      switch (type) {
        case Types.INTEGER:
          return "java.lang.Integer";
        case Types.VARCHAR:
          return "java.lang.String";
        case Types.DATE:
          return "java.sql.Date";
        case Types.TIMESTAMP:
          return "java.sql.Timestamp";
        default:
          return "java.lang.String";
      }
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

    private void checkColumnIndex(int column) throws SQLException {
      if (column < 1 || column > columnNames.length) {
        throw new SQLException("Invalid column index: " + column);
      }
    }
  }
}
