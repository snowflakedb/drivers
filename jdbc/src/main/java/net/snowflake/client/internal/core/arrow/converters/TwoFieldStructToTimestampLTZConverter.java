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
import net.snowflake.client.internal.jdbc.SnowflakeDateWithTimezone;
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.complex.StructVector;

/**
 * Converter from a two-field struct ({@code epoch} seconds + {@code fraction} nanos) to {@code
 * TIMESTAMP_LTZ}. The stored value is an instant; the caller-supplied {@code TimeZone}/{@code
 * Calendar} is ignored, and only {@code sessionTimeZone} + {@code useSessionTimezone} (read from
 * {@link DataConversionContext}) affect rendering and the returned subtype. Ported verbatim from
 * snowflake-jdbc's {@code TwoFieldStructToTimestampLTZConverter}.
 *
 * <p>The new tree carries the session flags on {@link DataConversionContext} (set once per result
 * set) rather than on per-call converter setters, so each get-method reads {@link
 * DataConversionContext#getSessionTimeZone()} / {@link
 * DataConversionContext#isUseSessionTimezone()} directly.
 */
public class TwoFieldStructToTimestampLTZConverter extends AbstractArrowVectorConverter {
  private final StructVector structVector;
  private final BigIntVector epochs;
  private final IntVector fractions;
  private final int scale;

  public TwoFieldStructToTimestampLTZConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(SnowflakeType.TIMESTAMP_LTZ.name(), fieldVector, columnIndex, context);
    this.structVector = (StructVector) fieldVector;
    this.epochs = structVector.getChild(FIELD_NAME_EPOCH, BigIntVector.class);
    this.fractions = structVector.getChild(FIELD_NAME_FRACTION, IntVector.class);
    this.scale = scale;
  }

  @Override
  public boolean isNull(int index) {
    return structVector.isNull(index) || epochs.isNull(index) || fractions.isNull(index);
  }

  @Override
  public String toString(int index) throws SFException {
    if (context.getTimestampLTZFormatter() == null) {
      throw new SFException(ErrorCode.INTERNAL_ERROR, "missing timestamp LTZ formatter");
    }
    try {
      Timestamp ts = isNull(index) ? null : getTimestamp(index, true);
      return ts == null
          ? null
          : context.getTimestampLTZFormatter().format(ts, context.getSessionTimeZone(), scale);
    } catch (TimestampOperationNotAvailableException e) {
      return e.getSecsSinceEpoch().toPlainString();
    }
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toTimestamp(index, TimeZone.getDefault());
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    // LTZ ignores the caller tz/Calendar; the instant is correct from the epoch alone and only
    // sessionTimeZone + useSessionTimezone affect the returned subtype.
    return isNull(index) ? null : getTimestamp(index, false);
  }

  private Timestamp getTimestamp(int index, boolean fromToString) throws SFException {
    long epoch = epochs.getDataBuffer().getLong((long) index * BigIntVector.TYPE_WIDTH);
    int fraction = fractions.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    return getTimestamp(
        epoch,
        fraction,
        context.getSessionTimeZone(),
        context.isUseSessionTimezone(),
        fromToString);
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean dateFormat) throws SFException {
    if (isNull(index)) {
      return null;
    }
    Timestamp ts = getTimestamp(index, false);
    // ts can be null when the value overflows Java's millisecond Timestamp range.
    return ts == null
        ? null
        : new SnowflakeDateWithTimezone(
            ts.getTime(), context.getSessionTimeZone(), context.isUseSessionTimezone());
  }

  @Override
  public Time toTime(int index) throws SFException {
    Timestamp ts = toTimestamp(index, TimeZone.getDefault());
    return ts == null
        ? null
        : new SnowflakeTimeWithTimezone(
            ts, context.getSessionTimeZone(), context.isUseSessionTimezone());
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

  static Timestamp getTimestamp(
      long epoch,
      int fraction,
      TimeZone sessionTimeZone,
      boolean useSessionTimezone,
      boolean fromToString)
      throws SFException {
    if (ArrowResultUtil.isTimestampOverflow(epoch)) {
      if (fromToString) {
        throw new TimestampOperationNotAvailableException(epoch, fraction);
      } else {
        return null;
      }
    }
    Timestamp ts =
        ArrowResultUtil.createTimestamp(epoch, fraction, sessionTimeZone, useSessionTimezone);
    return ArrowDateUtil.adjustTimestamp(ts);
  }
}
