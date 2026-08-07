package net.snowflake.client.internal.api.implementation.resultset;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.TimeZone;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;

/** Reads typed column values from the current row. Column indices are 1-based (JDBC convention). */
public interface ColumnAccessor {

  boolean wasNull();

  String getString(int columnIndex);

  boolean getBoolean(int columnIndex);

  byte getByte(int columnIndex);

  short getShort(int columnIndex);

  int getInt(int columnIndex);

  long getLong(int columnIndex);

  float getFloat(int columnIndex);

  double getDouble(int columnIndex);

  BigDecimal getBigDecimal(int columnIndex);

  byte[] getBytes(int columnIndex);

  Date getDate(int columnIndex);

  /**
   * Materializes a DATE column applying the caller-supplied {@code tz} together with the runtime
   * {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} flag, mirroring snowflake-jdbc's {@code
   * SFArrowResultSet.getDate(int, TimeZone)}.
   */
  Date getDate(int columnIndex, TimeZone tz);

  /** Conversion context backing this reader; drives timezone/format-sensitive materialization. */
  DataConversionContext getConversionContext();

  Time getTime(int columnIndex);

  Timestamp getTimestamp(int columnIndex);

  Timestamp getTimestamp(int columnIndex, TimeZone tz);

  Object getObject(int columnIndex);

  /**
   * Materializes an {@code INTERVAL YEAR TO MONTH} column as a {@link Period}. Backs {@code
   * ResultSet.getObject(col, Period.class)}. Readers that cannot produce intervals inherit the
   * default, which reports the operation as unsupported.
   */
  default Period getPeriod(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getPeriod is not supported for this result set");
  }

  /**
   * Materializes an {@code INTERVAL DAY TO SECOND} column as a {@link Duration}. Backs {@code
   * ResultSet.getObject(col, Duration.class)} and, unlike plain {@code getObject}, yields a {@code
   * Duration} for the SB16 (Decimal128) physical layout as well.
   */
  default Duration getDuration(int columnIndex) {
    throw new SFSQLFeatureNotSupportedException("getDuration is not supported for this result set");
  }

  int getColumnCount();

  String getColumnName(int columnIndex);

  default int findColumn(String columnLabel) {
    int count = getColumnCount();
    for (int i = 1; i <= count; i++) {
      if (getColumnName(i).equalsIgnoreCase(columnLabel)) {
        return i;
      }
    }
    throw new IllegalArgumentException("Column not found: " + columnLabel);
  }

  default String getString(String columnLabel) {
    return getString(findColumn(columnLabel));
  }
}
