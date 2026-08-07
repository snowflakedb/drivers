package net.snowflake.client.internal.core.arrow.converters;

import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.jdbc.SnowflakeDateWithTimezone;
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.complex.StructVector;

/**
 * Converter from a three-field struct ({@code epoch} seconds + {@code fraction} nanos + {@code
 * timezone} index) to {@code TIMESTAMP_TZ}. The stored value is an instant plus its own fixed
 * offset; the caller-supplied {@code TimeZone}/{@code Calendar} is ignored. The stored offset (not
 * the session zone) is what renders in {@code getString} and what the {@link
 * SnowflakeDateWithTimezone}/{@link SnowflakeTimeWithTimezone}/{@code
 * SnowflakeTimestampWithTimezone} wrappers carry. Ported verbatim from snowflake-jdbc's {@code
 * ThreeFieldStructToTimestampTZConverter}.
 *
 * <p>The legacy converter decodes the timezone index only when {@code resultVersion > 0} (falling
 * back to UTC otherwise). The new core has no {@code resultVersion} concept and always emits a
 * valid timezone index for {@code TIMESTAMP_TZ}, so that branch is dropped here.
 */
public class ThreeFieldStructToTimestampTZConverter extends AbstractArrowVectorConverter {
  private final StructVector structVector;
  private final BigIntVector epochs;
  private final IntVector fractions;
  private final IntVector timeZoneIndices;
  private final int scale;

  public ThreeFieldStructToTimestampTZConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    // Legacy passes TIMESTAMP_LTZ.name() here; kept verbatim so INVALID_VALUE_CONVERT error
    // messages render identically to snowflake-jdbc.
    super(SnowflakeType.TIMESTAMP_LTZ.name(), fieldVector, columnIndex, context);
    this.structVector = (StructVector) fieldVector;
    this.epochs = structVector.getChild(FIELD_NAME_EPOCH, BigIntVector.class);
    this.fractions = structVector.getChild(FIELD_NAME_FRACTION, IntVector.class);
    this.timeZoneIndices = structVector.getChild(FIELD_NAME_TIMEZONE, IntVector.class);
    this.scale = scale;
  }

  @Override
  public boolean isNull(int index) {
    return structVector.isNull(index)
        || epochs.isNull(index)
        || fractions.isNull(index)
        || timeZoneIndices.isNull(index);
  }

  @Override
  public String toString(int index) {
    if (context.getTimestampTZFormatter() == null) {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INTERNAL_ERROR, "missing timestamp TZ formatter");
    }
    try {
      Timestamp ts = isNull(index) ? null : getTimestamp(index, true);
      return ts == null
          ? null
          : context.getTimestampTZFormatter().format(ts, getStoredZone(index), scale);
    } catch (TimestampOperationNotAvailableException e) {
      return e.getSecsSinceEpoch().toPlainString();
    }
  }

  @Override
  public Object toObject(int index) {
    return toTimestamp(index, TimeZone.getDefault());
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) {
    // TZ carries its own stored offset; the caller tz/Calendar is ignored.
    return isNull(index) ? null : getTimestamp(index, false);
  }

  private Timestamp getTimestamp(int index, boolean fromToString) {
    long epoch = epochs.getDataBuffer().getLong((long) index * BigIntVector.TYPE_WIDTH);
    int fraction = fractions.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    int timeZoneIndex = timeZoneIndices.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    return getTimestamp(
        epoch, fraction, timeZoneIndex, context.isUseSessionTimezone(), fromToString);
  }

  /** The fixed-offset zone stored with this value, used for rendering and the returned wrappers. */
  private TimeZone getStoredZone(int index) {
    int timeZoneIndex = timeZoneIndices.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    return ArrowResultUtil.convertTimezoneIndexToTimeZone(timeZoneIndex);
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean dateFormat) {
    if (isNull(index)) {
      return null;
    }
    Timestamp ts = getTimestamp(index, false);
    // ts can be null when the value overflows Java's millisecond Timestamp range.
    return ts == null
        ? null
        : new SnowflakeDateWithTimezone(
            ts.getTime(), getStoredZone(index), context.isUseSessionTimezone());
  }

  @Override
  public Time toTime(int index) {
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : new SnowflakeTimeWithTimezone(ts, getStoredZone(index), context.isUseSessionTimezone());
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

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "byteArray", toString(index));
  }

  static Timestamp getTimestamp(
      long epoch,
      int fraction,
      int timeZoneIndex,
      boolean useSessionTimezone,
      boolean fromToString) {
    if (ArrowResultUtil.isTimestampOverflow(epoch)) {
      if (fromToString) {
        throw new TimestampOperationNotAvailableException(epoch, fraction);
      } else {
        return null;
      }
    }
    TimeZone timeZone = ArrowResultUtil.convertTimezoneIndexToTimeZone(timeZoneIndex);
    Timestamp ts = ArrowResultUtil.createTimestamp(epoch, fraction, timeZone, useSessionTimezone);
    return ArrowDateUtil.adjustTimestamp(ts);
  }
}
