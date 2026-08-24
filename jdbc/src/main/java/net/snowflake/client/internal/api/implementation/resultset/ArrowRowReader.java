package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.List;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
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
  private final long totalRowCount;
  private boolean closed = false;

  // Construction runs before any decorator boundary, so it declares the checked SQLException from
  // the Arrow schema read directly rather than sneaky-throwing it like the accessors below.
  ArrowRowReader(
      ArrowResources resources, DataConversionContext conversionContext, long totalRowCount) {
    this.resources = resources;
    this.cursor = new CursorState();
    this.schema = new SchemaState(resources.getActiveRoot(), conversionContext);
    this.batchManager = new ArrowBatchManager(cursor, resources, schema);
    this.totalRowCount = totalRowCount;
  }

  @Override
  public boolean next() {
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
  public void close() {
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
  public boolean isLast() {
    // Parity with snowflake-jdbc: an empty result (totalRowCount == 0) reports the before-first
    // cursor as last, since currentRow (-1) + 1 == 0. Deliberately no currentRow >= 0 guard.
    return !cursor.isAfterLast() && cursor.getCurrentRow() + 1 == totalRowCount;
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
  public String getColumnName(int columnIndex) {
    checkColumnIndex(columnIndex);
    return schema.getColumnNames()[columnIndex - 1];
  }

  @Override
  public String getString(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toString);
  }

  @Override
  public boolean getBoolean(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toBoolean);
  }

  @Override
  public byte getByte(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toByte);
  }

  @Override
  public short getShort(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toShort);
  }

  @Override
  public int getInt(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toInt);
  }

  @Override
  public long getLong(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toLong);
  }

  @Override
  public float getFloat(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toFloat);
  }

  @Override
  public double getDouble(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toDouble);
  }

  @Override
  public BigDecimal getBigDecimal(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toBigDecimal);
  }

  @Override
  public byte[] getBytes(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toBytes);
  }

  @Override
  public Date getDate(int columnIndex) {
    return getDate(columnIndex, null);
  }

  @Override
  public Date getDate(int columnIndex, TimeZone tz) {
    boolean useDateFormat = schema.getConversionContext().isFormatDateWithTimezone();
    return convertColumn(columnIndex, (converter, idx) -> converter.toDate(idx, tz, useDateFormat));
  }

  @Override
  public DataConversionContext getConversionContext() {
    return schema.getConversionContext();
  }

  @Override
  public Time getTime(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toTime);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex) {
    return getTimestamp(columnIndex, null);
  }

  @Override
  public Timestamp getTimestamp(int columnIndex, TimeZone tz) {
    return convertColumn(columnIndex, (converter, idx) -> converter.toTimestamp(idx, tz));
  }

  @Override
  public Object getObject(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toObject);
  }

  @Override
  public Period getPeriod(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toPeriod);
  }

  @Override
  public Duration getDuration(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toDuration);
  }

  @Override
  public List<?> getList(int columnIndex) {
    return convertColumn(columnIndex, ArrowVectorConverter::toList);
  }

  private void checkState() {
    if (closed) {
      throw new IllegalStateException("RowReader is closed");
    }
    if (cursor.isAfterLast()) {
      throw new IllegalStateException("After last row");
    }
    if (cursor.getCurrentRowInBatch() < 0) {
      throw new IllegalStateException("Before first row");
    }
  }

  private void checkColumnIndex(int columnIndex) {
    if (columnIndex < 1 || columnIndex > schema.getColumnCount()) {
      throw SFSQLException.fromErrorCode(ErrorCode.COLUMN_DOES_NOT_EXIST, columnIndex);
    }
  }

  private interface ConverterFunction<T> {
    T convert(ArrowVectorConverter converter, int rowIndex);
  }

  private <T> T convertColumn(int columnIndex, ConverterFunction<T> fn) {
    checkState();
    checkColumnIndex(columnIndex);
    ArrowVectorConverter converter = schema.getConverter(columnIndex, resources.getActiveRoot());
    int rowIndex = cursor.getCurrentRowInBatch();
    T value = fn.convert(converter, rowIndex);
    cursor.setWasNull(converter.isNull(rowIndex));
    return value;
  }
}
