package net.snowflake.client.internal.core.arrow.converters;

import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.complex.StructVector;

/**
 * Converter from a two-field struct ({@code epoch} seconds + {@code timezone} index) to {@code
 * TIMESTAMP_TZ}. This is the scale-0 layout: there is no {@code fraction} child, so the instant is
 * built from whole seconds via {@link ArrowResultUtil#toJavaTimestamp(long, int)} and is always a
 * plain {@link Timestamp} — the stored offset only affects {@code getString} rendering. Ported
 * verbatim from snowflake-jdbc's {@code TwoFieldStructToTimestampTZConverter}.
 *
 * <p>Unlike the three-field form, this layout never re-anchors to the session/caller timezone: the
 * caller-supplied {@code TimeZone}/{@code Calendar} is ignored and the returned {@code Timestamp}
 * carries no zone wrapper.
 *
 * <p>The legacy converter decodes the timezone index only when {@code resultVersion > 0} (falling
 * back to UTC otherwise). The new core has no {@code resultVersion} concept and always emits a
 * valid timezone index for {@code TIMESTAMP_TZ}, so that branch is dropped here.
 */
public class TwoFieldStructToTimestampTZConverter extends AbstractArrowVectorConverter {
  private final StructVector structVector;
  private final BigIntVector epochs;
  private final IntVector timeZoneIndices;
  private final int scale;

  public TwoFieldStructToTimestampTZConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    // Legacy passes TIMESTAMP_LTZ.name() here; kept verbatim so INVALID_VALUE_CONVERT error
    // messages render identically to snowflake-jdbc.
    super(SnowflakeType.TIMESTAMP_LTZ.name(), fieldVector, columnIndex, context);
    this.structVector = (StructVector) fieldVector;
    this.epochs = structVector.getChild(FIELD_NAME_EPOCH, BigIntVector.class);
    this.timeZoneIndices = structVector.getChild(FIELD_NAME_TIMEZONE, IntVector.class);
    this.scale = scale;
  }

  @Override
  public boolean isNull(int index) {
    return structVector.isNull(index) || epochs.isNull(index) || timeZoneIndices.isNull(index);
  }

  @Override
  public String toString(int index) throws SFException {
    if (context.getTimestampTZFormatter() == null) {
      throw new SFException(ErrorCode.INTERNAL_ERROR, "missing timestamp TZ formatter");
    }
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : context.getTimestampTZFormatter().format(ts, getStoredZone(index), scale);
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toTimestamp(index, TimeZone.getDefault());
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    // TZ carries its own stored offset; the caller tz/Calendar is ignored.
    return isNull(index) ? null : getTimestamp(index);
  }

  private Timestamp getTimestamp(int index) throws SFException {
    long epoch = epochs.getDataBuffer().getLong((long) index * BigIntVector.TYPE_WIDTH);
    return getTimestamp(epoch, scale);
  }

  /** The fixed-offset zone stored with this value, used for {@code getString} rendering only. */
  private TimeZone getStoredZone(int index) {
    int timeZoneIndex = timeZoneIndices.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    return ArrowResultUtil.convertTimezoneIndexToTimeZone(timeZoneIndex);
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean dateFormat) throws SFException {
    if (isNull(index)) {
      return null;
    }
    Timestamp ts = getTimestamp(index);
    // ts can be null when the value overflows Java's millisecond Timestamp range.
    return ts == null ? null : new Date(ts.getTime());
  }

  @Override
  public Time toTime(int index) throws SFException {
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : new SnowflakeTimeWithTimezone(
            ts.getTime(), ts.getNanos(), context.isUseSessionTimezone());
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    if (isNull(index)) {
      return false;
    }
    Timestamp val = toTimestamp(index, TimeZone.getDefault());
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "byteArray", toString(index));
  }

  static Timestamp getTimestamp(long epoch, int scale) throws SFException {
    Timestamp ts = ArrowResultUtil.toJavaTimestamp(epoch, scale);
    return ArrowDateUtil.adjustTimestamp(ts);
  }
}
