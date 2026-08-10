package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
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
  public boolean isLast() {
    // Parity with snowflake-jdbc: an empty result reports the before-first cursor as last, since
    // currentRow (-1) == rows.length - 1 (-1). Deliberately no currentRow >= 0 guard.
    return !isAfterLast() && currentRow == rows.length - 1;
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
  public String getColumnName(int columnIndex) {
    if (columnIndex < 1 || columnIndex > columnNames.length) {
      throw new IllegalArgumentException("Column index out of range: " + columnIndex);
    }
    return columnNames[columnIndex - 1];
  }

  @Override
  public Object getObject(int columnIndex) {
    return cell(columnIndex);
  }

  @Override
  public String getString(int columnIndex) {
    Object value = cell(columnIndex);
    return value == null ? null : value.toString();
  }

  @Override
  public boolean getBoolean(int columnIndex) {
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
  public byte getByte(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid byte: " + value, e);
    }
    throw noConversion("BYTE", value);
  }

  @Override
  public short getShort(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid short: " + value, e);
    }
    throw noConversion("SHORT", value);
  }

  @Override
  public int getInt(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid int: " + value, e);
    }
    throw noConversion("INT", value);
  }

  @Override
  public long getLong(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid long: " + value, e);
    }
    throw noConversion("LONG", value);
  }

  @Override
  public float getFloat(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid float: " + value, e);
    }
    throw noConversion("FLOAT", value);
  }

  @Override
  public double getDouble(int columnIndex) {
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
      throw new IllegalArgumentException("Invalid double: " + value, e);
    }
    throw noConversion("DOUBLE", value);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) {
    Object value = cell(columnIndex);
    if (value == null) {
      return null;
    }
    return new BigDecimal(value.toString());
  }

  @Override
  public byte[] getBytes(int columnIndex) {
    Object value = cell(columnIndex);
    if (value == null) {
      throw new IllegalStateException("Cannot get bytes on null column");
    }
    if (value instanceof byte[]) {
      return (byte[]) value;
    }
    return value.toString().getBytes(StandardCharsets.UTF_8);
  }

  @Override
  public Date getDate(int columnIndex) {
    Object value = cell(columnIndex);
    if (value instanceof Date) {
      return (Date) value;
    }
    throw noConversion("DATE", value);
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) {
    return getDate(columnIndex);
  }

  @Override
  public DataConversionContext getConversionContext() {
    return null;
  }

  @Override
  public Time getTime(int columnIndex) {
    Object value = cell(columnIndex);
    if (value instanceof Time) {
      return (Time) value;
    }
    throw noConversion("TIME", value);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) {
    Object value = cell(columnIndex);
    if (value instanceof Timestamp) {
      return (Timestamp) value;
    }
    throw noConversion("TIMESTAMP", value);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, TimeZone tz) {
    return getTimestamp(columnIndex);
  }

  private Object cell(int columnIndex) {
    if (currentRow < 0 || currentRow >= rows.length) {
      throw new IllegalStateException(NO_CURRENT_ROW);
    }
    if (columnIndex < 1 || columnIndex > columnNames.length) {
      throw new IllegalArgumentException("Column index out of range: " + columnIndex);
    }
    Object value = rows[currentRow][columnIndex - 1];
    wasNull = value == null;
    return value;
  }

  private static IllegalArgumentException noConversion(String type, Object value) {
    return new IllegalArgumentException(
        "Cannot convert "
            + (value == null ? "null" : value.getClass().getSimpleName())
            + " to "
            + type
            + ".");
  }
}
