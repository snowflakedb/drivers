package net.snowflake.client.internal.core.arrow.converters;

import java.nio.ByteBuffer;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;

/**
 * Converter from a compact {@code Int64} (scaled epoch) to {@code TIMESTAMP_NTZ}. The stored value
 * is a UTC wall-clock; the only timezone effect is the honor-client-TZ re-anchoring (unless
 * rendering via {@code toString}). Ported verbatim from snowflake-jdbc's {@code
 * BigIntToTimestampNTZConverter}.
 */
public class BigIntToTimestampNTZConverter extends AbstractArrowVectorConverter {
  private static final TimeZone NTZ = TimeZone.getTimeZone("UTC");

  private final Column bigColumn;
  private final int scale;
  private final ByteBuffer byteBuf = ByteBuffer.allocate(Long.BYTES);

  public BigIntToTimestampNTZConverter(
      Column fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(SnowflakeType.TIMESTAMP_NTZ.name(), fieldVector, columnIndex, context);
    this.bigColumn = fieldVector;
    this.scale = scale;
  }

  @Override
  public String toString(int index) {
    if (context.getTimestampNTZFormatter() == null) {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INTERNAL_ERROR, "missing timestamp NTZ formatter");
    }
    Timestamp ts = isNull(index) ? null : getTimestamp(index, TimeZone.getDefault(), true);
    return ts == null
        ? null
        : context.getTimestampNTZFormatter().format(ts, TimeZone.getTimeZone("UTC"), scale);
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    byteBuf.putLong(0, bigColumn.getLong(index));
    return byteBuf.array();
  }

  @Override
  public Object toObject(int index) {
    return toTimestamp(index, TimeZone.getDefault());
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) {
    return isNull(index) ? null : getTimestamp(index, tz, false);
  }

  private Timestamp getTimestamp(int index, TimeZone tz, boolean fromToString) {
    if (tz == null) {
      tz = TimeZone.getDefault();
    }
    long val = bigColumn.getLong(index);
    Timestamp ts = ArrowResultUtil.toJavaTimestamp(val, scale);

    // Note: honorClientTZForTimestampNTZ is not enabled for the toString method.
    if (!fromToString && context.isHonorClientTZForTimestampNTZ()) {
      ts = ArrowResultUtil.moveToTimeZone(ts, NTZ, tz);
    }
    return ArrowDateUtil.adjustTimestamp(ts);
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean dateFormat) {
    return isNull(index)
        ? null
        : new Date(getTimestamp(index, TimeZone.getDefault(), false).getTime());
  }

  @Override
  public Time toTime(int index) {
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : new SnowflakeTimeWithTimezone(
            ts.getTime(), ts.getNanos(), context.isUseSessionTimezone());
  }

  @Override
  public boolean toBoolean(int index) {
    if (isNull(index)) {
      return false;
    }
    Timestamp val = toTimestamp(index, TimeZone.getDefault());
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }
}
