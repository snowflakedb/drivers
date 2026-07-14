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

/**
 * A {@link RowReader} decorator that projects rows from a wrapped reader into a new column layout
 * with possible filtering applied
 *
 * <p>The {@link RowConverter} is called after each successful {@code next()} on the delegate. <br>
 * It reads values from the delegate's current row and returns:
 *
 * <ul>
 *   <li>an {@code Object[]} representing the output columns,
 *   <li>or {@code null} which mean that this row is skipped (filtering).
 * </ul>
 *
 * <p>Column access on this reader operates on the projected {@code Object[]} - it does not touch
 * the delegate.
 */
@RequiredArgsConstructor
class ConvertingRowReader implements RowReader {

  private final RowReader delegate;
  private final String[] columnNames;
  private final RowConverter converter;

  private Object[] currentRow;
  private boolean wasNull;
  private boolean closed;
  private int currentRowIndex = -1;
  private boolean afterLast;

  // --- RowCursor ---

  @Override
  public boolean next() throws SQLException {
    while (delegate.next()) {
      currentRow = converter.convert(delegate);
      if (currentRow != null) {
        currentRowIndex++;
        return true;
      }
    }
    currentRow = null;
    afterLast = true;
    return false;
  }

  @Override
  public void close() throws SQLException {
    closed = true;
    delegate.close();
  }

  @Override
  public boolean isClosed() {
    return closed;
  }

  @Override
  public boolean isBeforeFirst() {
    return currentRowIndex < 0 && !afterLast;
  }

  @Override
  public boolean isAfterLast() {
    return afterLast;
  }

  @Override
  public boolean isFirst() {
    return currentRowIndex == 0 && !afterLast;
  }

  @Override
  public boolean isLast() throws SQLException {
    // The converter may drop rows, so the delegate's row count does not map to the projected
    // count and isLast() cannot be answered without buffering ahead.
    // TODO: add a one-row look-ahead buffer so isLast() returns a correct boolean here (legacy
    //  parity for metadata result sets) instead of throwing.
    throw new SQLException("isLast not supported for projected result sets");
  }

  @Override
  public int getCurrentRow() {
    return currentRowIndex;
  }

  // --- ColumnAccessor (reads from projected Object[]) ---

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
  public String getString(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    return obj == null ? null : obj.toString();
  }

  @Override
  public boolean getBoolean(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return false;
    }
    if (obj instanceof Boolean) {
      return (Boolean) obj;
    }
    if (obj instanceof Number) {
      return ((Number) obj).intValue() != 0;
    }
    String s = obj.toString();
    return "1".equals(s) || Boolean.parseBoolean(s);
  }

  @Override
  public byte getByte(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).byteValue();
    }
    return Byte.parseByte(obj.toString());
  }

  @Override
  public short getShort(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).shortValue();
    }
    return Short.parseShort(obj.toString());
  }

  @Override
  public int getInt(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).intValue();
    }
    return Integer.parseInt(obj.toString());
  }

  @Override
  public long getLong(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).longValue();
    }
    return Long.parseLong(obj.toString());
  }

  @Override
  public float getFloat(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).floatValue();
    }
    return Float.parseFloat(obj.toString());
  }

  @Override
  public double getDouble(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return 0;
    }
    if (obj instanceof Number) {
      return ((Number) obj).doubleValue();
    }
    return Double.parseDouble(obj.toString());
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return null;
    }
    if (obj instanceof BigDecimal) {
      return (BigDecimal) obj;
    }
    return new BigDecimal(obj.toString());
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    String str = getString(columnIndex);
    return str == null ? null : str.getBytes(StandardCharsets.UTF_8);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return null;
    }
    if (obj instanceof Date) {
      return (Date) obj;
    }
    return Date.valueOf(obj.toString());
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) throws SQLException {
    // Projected rows hold already-materialized java.sql.Date values; the timezone shift was applied
    // (or not) by the delegate when the value was produced, so tz is not re-applied here.
    return getDate(columnIndex);
  }

  @Override
  public DataConversionContext getConversionContext() {
    return delegate.getConversionContext();
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return null;
    }
    if (obj instanceof Time) {
      return (Time) obj;
    }
    return Time.valueOf(obj.toString());
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    Object obj = getObjectInternal(columnIndex);
    if (obj == null) {
      return null;
    }
    if (obj instanceof Timestamp) {
      return (Timestamp) obj;
    }
    return Timestamp.valueOf(obj.toString());
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, TimeZone tz) throws SQLException {
    // Projected rows hold already-materialized java.sql.Timestamp values; the timezone was applied
    // (or not) by the delegate when the value was produced, so tz is not re-applied here.
    return getTimestamp(columnIndex);
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    return getObjectInternal(columnIndex);
  }

  private Object getObjectInternal(int columnIndex) throws SQLException {
    if (currentRow == null) {
      throw new SQLException("No row found.");
    }
    if (columnIndex < 1 || columnIndex > currentRow.length) {
      throw new SQLException("Invalid column index: " + columnIndex);
    }
    Object value = currentRow[columnIndex - 1];
    wasNull = (value == null);
    return value;
  }
}
