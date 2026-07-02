package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;

/** A {@link RowReader} backed by pre-built rows supplied by the caller. */
@RequiredArgsConstructor
class InMemoryRowReader implements RowReader {

  private static final String NO_CURRENT_ROW = "No current row.";

  private final String[] columnNames;
  private final Object[][] rows;

  private boolean closed;
  private int currentRow = -1;
  private boolean wasNull = false;

  // --- RowCursor ---

  @Override
  public boolean next() {
    if (currentRow < rows.length) {
      currentRow++;
    }
    return currentRow < rows.length;
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
    return currentRow == -1;
  }

  @Override
  public boolean isAfterLast() {
    return currentRow >= rows.length;
  }

  @Override
  public boolean isFirst() {
    return currentRow == 0 && rows.length > 0;
  }

  @Override
  public int getCurrentRow() {
    return currentRow;
  }

  // --- ColumnAccessor ---

  @Override
  public boolean wasNull() {
    return wasNull;
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
  public Object getObject(int columnIndex) throws SQLException {
    return cell(columnIndex);
  }

  @Override
  public String getString(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    return value == null ? null : value.toString();
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return false;
    }
    if (value instanceof Boolean) {
      return (Boolean) value;
    }
    if (value instanceof String) {
      return "1".equals(value);
    }
    if (value instanceof Number) {
      return ((Number) value).intValue() > 0;
    }
    throw noConversion("BOOLEAN", value);
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Byte.parseByte((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).byteValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid byte: " + value, e);
    }
    throw noConversion("BYTE", value);
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Short.parseShort((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).shortValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid short: " + value, e);
    }
    throw noConversion("SHORT", value);
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Integer.parseInt((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).intValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid int: " + value, e);
    }
    throw noConversion("INT", value);
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Long.parseLong((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).longValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid long: " + value, e);
    }
    throw noConversion("LONG", value);
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Float.parseFloat((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).floatValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid float: " + value, e);
    }
    throw noConversion("FLOAT", value);
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return 0;
    }
    try {
      if (value instanceof String) {
        return Double.parseDouble((String) value);
      }
      if (value instanceof Number) {
        return ((Number) value).doubleValue();
      }
    } catch (NumberFormatException e) {
      throw new SQLException("Invalid double: " + value, e);
    }
    throw noConversion("DOUBLE", value);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      return null;
    }
    return new BigDecimal(value.toString());
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value == null) {
      throw new SQLException("Cannot get bytes on null column");
    }
    if (value instanceof byte[]) {
      return (byte[]) value;
    }
    return value.toString().getBytes(StandardCharsets.UTF_8);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value instanceof Date) {
      return (Date) value;
    }
    throw noConversion("DATE", value);
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) throws SQLException {
    return getDate(columnIndex);
  }

  @Override
  public DataConversionContext getConversionContext() {
    return null;
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value instanceof Time) {
      return (Time) value;
    }
    throw noConversion("TIME", value);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    Object value = cell(columnIndex);
    if (value instanceof Timestamp) {
      return (Timestamp) value;
    }
    throw noConversion("TIMESTAMP", value);
  }

  private Object cell(int columnIndex) throws SQLException {
    if (currentRow < 0 || currentRow >= rows.length) {
      throw new SQLException(NO_CURRENT_ROW);
    }
    if (columnIndex < 1 || columnIndex > columnNames.length) {
      throw new SQLException("Column index out of range: " + columnIndex);
    }
    Object value = rows[currentRow][columnIndex - 1];
    wasNull = value == null;
    return value;
  }

  private static SQLException noConversion(String type, Object value) {
    return new SQLException(
        "Cannot convert "
            + (value == null ? "null" : value.getClass().getSimpleName())
            + " to "
            + type
            + ".");
  }
}
