package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;

/** Reads typed column values from the current row. Column indices are 1-based (JDBC convention). */
public interface ColumnAccessor {

  boolean wasNull();

  String getString(int columnIndex) throws SQLException;

  boolean getBoolean(int columnIndex) throws SQLException;

  byte getByte(int columnIndex) throws SQLException;

  short getShort(int columnIndex) throws SQLException;

  int getInt(int columnIndex) throws SQLException;

  long getLong(int columnIndex) throws SQLException;

  float getFloat(int columnIndex) throws SQLException;

  double getDouble(int columnIndex) throws SQLException;

  BigDecimal getBigDecimal(int columnIndex) throws SQLException;

  byte[] getBytes(int columnIndex) throws SQLException;

  Date getDate(int columnIndex) throws SQLException;

  /**
   * Materializes a DATE column applying the caller-supplied {@code tz} together with the runtime
   * {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} flag, mirroring snowflake-jdbc's {@code
   * SFArrowResultSet.getDate(int, TimeZone)}.
   */
  Date getDate(int columnIndex, TimeZone tz) throws SQLException;

  /** Conversion context backing this reader; drives timezone/format-sensitive materialization. */
  DataConversionContext getConversionContext();

  Time getTime(int columnIndex) throws SQLException;

  Timestamp getTimestamp(int columnIndex) throws SQLException;

  Object getObject(int columnIndex) throws SQLException;

  int getColumnCount() throws SQLException;

  String getColumnName(int columnIndex) throws SQLException;

  default int findColumn(String columnLabel) throws SQLException {
    int count = getColumnCount();
    for (int i = 1; i <= count; i++) {
      if (getColumnName(i).equalsIgnoreCase(columnLabel)) {
        return i;
      }
    }
    throw new SQLException("Column not found: " + columnLabel);
  }

  default String getString(String columnLabel) throws SQLException {
    return getString(findColumn(columnLabel));
  }
}
