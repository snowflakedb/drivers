package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;

/** A {@link RowReader} with a fixed column layout and zero rows. */
@RequiredArgsConstructor
class EmptyRowReader implements RowReader {

  private static final String NO_ROW_MESSAGE = "No row found.";

  private final String[] columnNames;

  private boolean closed;
  private boolean afterLast;

  // --- RowCursor ---

  @Override
  public boolean next() {
    afterLast = true;
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
  public boolean isBeforeFirst() {
    return !afterLast;
  }

  @Override
  public boolean isAfterLast() {
    return afterLast;
  }

  @Override
  public boolean isFirst() {
    return false;
  }

  @Override
  public int getCurrentRow() {
    return -1;
  }

  // --- ColumnAccessor ---

  @Override
  public boolean wasNull() {
    return false;
  }

  @Override
  public int getColumnCount() {
    return columnNames.length;
  }

  @Override
  public String getColumnName(int columnIndex) throws SQLException {
    if (columnIndex < 1 || columnIndex > columnNames.length) {
      throw new SQLException("Column index out of range: " + columnIndex);
    }
    return columnNames[columnIndex - 1];
  }

  @Override
  public String getString(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) throws SQLException {
    throw noRow();
  }

  @Override
  public DataConversionContext getConversionContext() {
    // No rows are ever materialized, so no conversion context is consulted; the session defaults
    // suffice for the metadata-only contract.
    return new DataConversionContext() {};
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    throw noRow();
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    throw noRow();
  }

  private static SQLException noRow() {
    return new SQLException(NO_ROW_MESSAGE);
  }
}
