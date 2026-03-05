package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.DateDayVector;
import org.apache.arrow.vector.ValueVector;

public class DateConverter extends AbstractArrowVectorConverter {
  private final DateDayVector dateVector;
  private final DateTimeFormatter formatter;

  public DateConverter(ValueVector fieldVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.DATE.name(), fieldVector, columnIndex, context);
    this.dateVector = (DateDayVector) fieldVector;
    this.formatter = DateTimeFormatter.ofPattern(context.getDateOutputFormat());
  }

  private int getEpochDays(int index) {
    return dateVector.get(index);
  }

  private LocalDate getLocalDate(int index) {
    return LocalDate.ofEpochDay(getEpochDays(index));
  }

  @Override
  public Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return Date.valueOf(getLocalDate(index));
  }

  @Override
  public String toString(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return getLocalDate(index).format(formatter);
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toDate(index, null, false);
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return Timestamp.valueOf(getLocalDate(index).atStartOfDay());
  }

  @Override
  public int toInt(int index) {
    if (isNull(index)) {
      return 0;
    }
    return getEpochDays(index);
  }

  @Override
  public short toShort(int index) throws SFException {
    if (isNull(index)) {
      return 0;
    }
    int val = getEpochDays(index);
    if (val < Short.MIN_VALUE || val > Short.MAX_VALUE) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.SHORT_STR, val);
    }
    return (short) val;
  }

  @Override
  public long toLong(int index) {
    return toInt(index);
  }

  @Override
  public float toFloat(int index) {
    return toInt(index);
  }

  @Override
  public double toDouble(int index) {
    return toInt(index);
  }

  @Override
  public BigDecimal toBigDecimal(int index) {
    if (isNull(index)) {
      return null;
    }
    return BigDecimal.valueOf(getEpochDays(index));
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    if (isNull(index)) {
      return false;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.BOOLEAN_STR,
        getLocalDate(index).toString());
  }
}
