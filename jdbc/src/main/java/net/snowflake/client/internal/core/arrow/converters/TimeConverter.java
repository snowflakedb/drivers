package net.snowflake.client.internal.core.arrow.converters;

import java.nio.ByteBuffer;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.LocalTime;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.BaseIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;

public class TimeConverter extends AbstractArrowVectorConverter {
  private static final long MILLIS_PER_NANO = 1_000_000L;

  private final BaseIntVector intVector;
  private final int scale;
  private final SnowflakeDateTimeFormat timeFormatter;

  public TimeConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(SnowflakeType.TIME.name(), fieldVector, columnIndex, context);
    this.intVector = (BaseIntVector) fieldVector;
    this.scale = scale;
    this.timeFormatter = context.getTimeFormatter();
  }

  private LocalTime getLocalTime(int index) {
    return LocalTime.ofNanoOfDay(
        intVector.getValueAsLong(index) * ArrowResultUtil.powerOfTen(9 - scale));
  }

  @Override
  public Time toTime(int index) {
    if (isNull(index)) {
      return null;
    }
    LocalTime localTime = getLocalTime(index);
    if (context.isUseSessionTimezone()) {
      return getTimeInSessionTimezone(localTime);
    }
    // Default (JDBC_USE_SESSION_TIMEZONE=false): a UTC-anchored Time built from
    // millis-since-midnight, matching snowflake-jdbc's new Time(SFTime.getFractionalSeconds(3)).
    return new Time(localTime.toNanoOfDay() / MILLIS_PER_NANO);
  }

  /**
   * Mirrors snowflake-jdbc's {@code SnowflakeUtil.getTimeInSessionTimezone}: anchor the wall-clock
   * fields in the JVM default timezone (via {@code Time.valueOf(LocalTime)}, which drops sub-second
   * precision) and re-attach the milliseconds within the second.
   */
  private static Time getTimeInSessionTimezone(LocalTime localTime) {
    Time ts = Time.valueOf(localTime);
    ts.setTime(ts.getTime() + localTime.getNano() / MILLIS_PER_NANO);
    return ts;
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) {
    if (isNull(index)) {
      return null;
    }
    // UTC-anchored SnowflakeTimestampWithTimezone (not a plain Timestamp) keeps toString() stable
    // across JVM timezones. Precision mirrors snowflake-jdbc: full nanos under session timezone,
    // else millisecond-truncated (the default path routes through millisecond-resolution SFTime).
    LocalTime localTime = getLocalTime(index);
    long millis = localTime.toNanoOfDay() / MILLIS_PER_NANO;
    int nanos =
        context.isUseSessionTimezone()
            ? localTime.getNano()
            : (int) (millis % 1000L * MILLIS_PER_NANO);
    return new SnowflakeTimestampWithTimezone(millis, nanos, TimeZone.getTimeZone("UTC"));
  }

  @Override
  public String toString(int index) {
    if (isNull(index)) {
      return null;
    }
    return timeFormatter.format(getLocalTime(index), scale);
  }

  @Override
  public Object toObject(int index) {
    return toTime(index);
  }

  @Override
  public boolean toBoolean(int index) {
    if (isNull(index)) {
      return false;
    }
    Time val = toTime(index);
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    // Only INT-backed TIME exposes raw bytes; BIGINT-backed TIME has no toBytes and falls through
    // to the unsupported-conversion error.
    if (intVector instanceof IntVector) {
      ByteBuffer byteBuf = ByteBuffer.allocate(IntVector.TYPE_WIDTH);
      byteBuf.putInt(
          0, ((IntVector) intVector).getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH));
      return byteBuf.array();
    }
    return super.toBytes(index);
  }
}
