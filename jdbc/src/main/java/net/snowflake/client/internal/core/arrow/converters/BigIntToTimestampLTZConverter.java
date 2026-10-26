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
import net.snowflake.client.internal.util.SnowflakeUtil;

/**
 * Converter from a compact {@code Int64} (scaled epoch) to {@code TIMESTAMP_LTZ}. The stored value
 * is an instant; the caller-supplied {@code TimeZone}/{@code Calendar} is ignored, and only {@code
 * sessionTimeZone} + {@code useSessionTimezone} (read from {@link DataConversionContext}) affect
 * rendering and the returned subtype. Ported verbatim from snowflake-jdbc's {@code
 * BigIntToTimestampLTZConverter}.
 */
public class BigIntToTimestampLTZConverter extends AbstractArrowVectorConverter {
  private final Column bigColumn;
  private final int scale;
  private final ByteBuffer byteBuf = ByteBuffer.allocate(Long.BYTES);

  public BigIntToTimestampLTZConverter(
      Column fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(SnowflakeType.TIMESTAMP_LTZ.name(), fieldVector, columnIndex, context);
    this.bigColumn = fieldVector;
    this.scale = scale;
  }

  @Override
  public String toString(int index) {
    if (context.getTimestampLTZFormatter() == null) {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INTERNAL_ERROR, "missing timestamp LTZ formatter");
    }
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : context.getTimestampLTZFormatter().format(ts, context.getSessionTimeZone(), scale);
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
    // LTZ ignores the caller tz/Calendar; the instant is correct from the epoch alone and only
    // sessionTimeZone + useSessionTimezone affect the returned subtype.
    return isNull(index) ? null : getTimestamp(index);
  }

  private Timestamp getTimestamp(int index) {
    long val = bigColumn.getLong(index);
    return ArrowDateUtil.adjustTimestamp(
        ArrowResultUtil.toJavaTimestamp(
            val, scale, context.getSessionTimeZone(), context.isUseSessionTimezone()));
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean useDateFormat) {
    return isNull(index) ? null : new Date(getTimestamp(index).getTime());
  }

  @Override
  public Time toTime(int index) {
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null ? null : new Time(ts.getTime());
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
