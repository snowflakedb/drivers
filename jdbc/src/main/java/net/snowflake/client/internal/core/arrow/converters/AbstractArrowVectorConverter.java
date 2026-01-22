package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.TimeZone;
import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.ValueVector;

abstract class AbstractArrowVectorConverter implements ArrowVectorConverter {
  protected String logicalTypeStr;
  private final ValueVector valueVector;
  protected final DataConversionContext context;
  protected final int columnIndex;

  AbstractArrowVectorConverter(
      String logicalTypeStr,
      ValueVector valueVector,
      int vectorIndex,
      DataConversionContext context) {
    this.logicalTypeStr = logicalTypeStr;
    this.valueVector = valueVector;
    this.columnIndex = vectorIndex + 1;
    this.context = context;
  }

  @Override
  public boolean toBoolean(int rowIndex) throws SFException {
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, "");
  }

  @Override
  public byte toByte(int rowIndex) throws SFException {
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, "");
  }

  @Override
  public short toShort(int rowIndex) throws SFException {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.SHORT_STR, "");
  }

  @Override
  public int toInt(int rowIndex) throws SFException {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.INT_STR);
  }

  @Override
  public long toLong(int rowIndex) throws SFException {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.LONG_STR, "");
  }

  @Override
  public double toDouble(int rowIndex) throws SFException {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DOUBLE_STR, "");
  }

  @Override
  public float toFloat(int rowIndex) throws SFException {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.FLOAT_STR, "");
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, "");
  }

  @Override
  public Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException {
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DATE_STR, "");
  }

  @Override
  public Time toTime(int index) throws SFException {
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.TIME_STR, "");
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.TIMESTAMP_STR, "");
  }

  @Override
  public BigDecimal toBigDecimal(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BIG_DECIMAL_STR, "");
  }

  @Override
  public Period toPeriod(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "period", "");
  }

  @Override
  public Duration toDuration(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "duration", "");
  }

  @Override
  public boolean isNull(int index) {
    return valueVector.isNull(index);
  }

  @Override
  public abstract Object toObject(int index) throws SFException;

  @Override
  public abstract String toString(int index) throws SFException;
}
