package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.cursor.ArrowBatchManager;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import net.snowflake.client.internal.core.arrow.cursor.CursorState;
import net.snowflake.client.internal.core.arrow.cursor.SchemaState;

class ArrowRowReader implements RowReader {

  private final ArrowResources resources;
  private final CursorState cursor;
  private final SchemaState schema;
  private final ArrowBatchManager batchManager;
  private boolean closed = false;

  ArrowRowReader(ArrowResources resources, DataConversionContext conversionContext)
      throws SQLException {
    this.resources = resources;
    this.cursor = new CursorState();
    this.schema = new SchemaState(resources.getActiveRoot(), conversionContext);
    this.batchManager = new ArrowBatchManager(cursor, resources, schema);
  }

  @Override
  public boolean next() throws SQLException {
    if (closed) {
      return false;
    }
    boolean hasNext = batchManager.fetchNextRow();
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
    closed = true;
    resources.closeAll();
    resources.reset();
    schema.reset();
    cursor.reset();
  }

  @Override
  public boolean isClosed() {
    return closed;
  }

  @Override
  public boolean isBeforeFirst() {
    return cursor.getCurrentRow() < 0 && !cursor.isAfterLast();
  }

  @Override
  public boolean isAfterLast() {
    return cursor.isAfterLast();
  }

  @Override
  public boolean isFirst() {
    return cursor.getCurrentRow() == 0 && !cursor.isAfterLast();
  }

  @Override
  public int getCurrentRow() {
    return cursor.getCurrentRow();
  }

  @Override
  public boolean wasNull() {
    return cursor.wasNull();
  }

  @Override
  public int getColumnCount() {
    return schema.getColumnCount();
  }

  @Override
  public String getColumnName(int columnIndex) throws SQLException {
    checkColumnIndex(columnIndex);
    return schema.getColumnNames()[columnIndex - 1];
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
  public BigDecimal getBigDecimal(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toBigDecimal);
  }

  @Override
  public byte[] getBytes(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toBytes);
  }

  @Override
  public Date getDate(int columnIndex) throws SQLException {
    return getDate(columnIndex, null);
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) throws SQLException {
    boolean useDateFormat = schema.getConversionContext().isFormatDateWithTimezone();
    return convertColumn(columnIndex, (converter, idx) -> converter.toDate(idx, tz, useDateFormat));
  }

  @Override
  public DataConversionContext getConversionContext() {
    return schema.getConversionContext();
  }

  @Override
  public Time getTime(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toTime);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, (converter, idx) -> converter.toTimestamp(idx, null));
  }

  @Override
  public Object getObject(int columnIndex) throws SQLException {
    return convertColumn(columnIndex, ArrowVectorConverter::toObject);
  }

  private void checkState() throws SQLException {
    if (closed) {
      throw new SQLException("RowReader is closed");
    }
    if (cursor.isAfterLast()) {
      throw new SQLException("After last row");
    }
    if (cursor.getCurrentRowInBatch() < 0) {
      throw new SQLException("Before first row");
    }
  }

  private void checkColumnIndex(int columnIndex) throws SQLException {
    if (columnIndex < 1 || columnIndex > schema.getColumnCount()) {
      throw new SnowflakeSQLException(
          ErrorCode.COLUMN_DOES_NOT_EXIST, "Column index out of range: " + columnIndex);
    }
  }

  private interface ConverterFunction<T> {
    T convert(ArrowVectorConverter converter, int rowIndex) throws SFException;
  }

  private <T> T convertColumn(int columnIndex, ConverterFunction<T> fn) throws SQLException {
    checkState();
    checkColumnIndex(columnIndex);
    ArrowVectorConverter converter = schema.getConverter(columnIndex, resources.getActiveRoot());
    try {
      int rowIndex = cursor.getCurrentRowInBatch();
      T value = fn.convert(converter, rowIndex);
      cursor.setWasNull(converter.isNull(rowIndex));
      return value;
    } catch (SFException e) {
      throw new SnowflakeSQLException(e.getErrorCode(), e.getMessage());
    }
  }
}
