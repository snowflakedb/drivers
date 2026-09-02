package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.List;
import java.util.TimeZone;
import lombok.Getter;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.ValueVector;

abstract class AbstractArrowVectorConverter implements ArrowVectorConverter {
  /** Struct child holding seconds since epoch. */
  static final String FIELD_NAME_EPOCH = "epoch";

  /** Struct child holding the sub-second nanosecond fraction. */
  static final String FIELD_NAME_FRACTION = "fraction";

  /** Struct child holding the {@code TIMESTAMP_TZ} timezone index (biased by 1440). */
  static final String FIELD_NAME_TIMEZONE = "timezone";

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
  public boolean toBoolean(int rowIndex) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, "");
  }

  @Override
  public byte toByte(int rowIndex) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, "");
  }

  @Override
  public short toShort(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.SHORT_STR, "");
  }

  @Override
  public int toInt(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.INT_STR, "");
  }

  @Override
  public long toLong(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.LONG_STR, "");
  }

  @Override
  public double toDouble(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DOUBLE_STR, "");
  }

  @Override
  public float toFloat(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.FLOAT_STR, "");
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, "");
  }

  @Override
  public Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DATE_STR, "");
  }

  @Override
  public Time toTime(int index) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.TIME_STR, "");
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.TIMESTAMP_STR, "");
  }

  @Override
  public BigDecimal toBigDecimal(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BIG_DECIMAL_STR, "");
  }

  @Override
  public Period toPeriod(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "period", "");
  }

  @Override
  public Duration toDuration(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "duration", "");
  }

  @Override
  public List<?> toList(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "list", "");
  }

  @Override
  public boolean isNull(int index) {
    return valueVector.isNull(index);
  }

  protected boolean shouldTreatDecimalAsInt() {
    return context == null || context.isTreatDecimalAsInt() || context.isArrowTreatDecimalAsInt();
  }

  @Override
  public abstract Object toObject(int index);

  @Override
  public abstract String toString(int index);

  /**
   * Thrown when a Snowflake timestamp cannot be materialized as a {@link Timestamp} because its
   * seconds-since-epoch falls outside the millisecond range a {@code long} can hold. Snowflake can
   * use a full SB16 for a timestamp; certain operations (e.g. {@code getTimestamp}) are then
   * unavailable, while {@code getString} can still render the seconds. Ported verbatim from
   * snowflake-jdbc's {@code AbstractArrowVectorConverter.TimestampOperationNotAvailableException}.
   */
  public static class TimestampOperationNotAvailableException extends RuntimeException {
    @Getter private final BigDecimal secsSinceEpoch;

    TimestampOperationNotAvailableException(long secsSinceEpoch, int fraction) {
      super("seconds=" + secsSinceEpoch + " nanos=" + fraction);
      this.secsSinceEpoch =
          new BigDecimal(secsSinceEpoch).add(new BigDecimal(fraction).scaleByPowerOfTen(-9));
    }
  }
}
