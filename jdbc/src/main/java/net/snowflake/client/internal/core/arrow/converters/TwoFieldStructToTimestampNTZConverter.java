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
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.complex.StructVector;

/**
 * Converter from a two-field struct ({@code epoch} seconds + {@code fraction} nanos) to {@code
 * TIMESTAMP_NTZ}. The stored value is a UTC wall-clock; {@code JDBC_USE_SESSION_TIMEZONE} / {@code
 * JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC} / {@code CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ} control
 * re-anchoring. Ported verbatim from snowflake-jdbc's {@code
 * TwoFieldStructToTimestampNTZConverter}.
 *
 * <p>The new tree carries the session flags on {@link DataConversionContext} (set once per result
 * set) rather than on per-call converter setters, so each get-method passes the legacy per-path
 * flag values explicitly: {@code treatNTZasUTC} is honored only on {@link #toObject} (legacy sets
 * it only on the getObject path), and {@link #toString} reads no session flags so it always renders
 * UTC.
 */
public class TwoFieldStructToTimestampNTZConverter extends AbstractArrowVectorConverter {
  private static final TimeZone NTZ = TimeZone.getTimeZone("UTC");

  private final StructVector structVector;
  private final BigIntVector epochs;
  private final IntVector fractions;
  private final int scale;

  public TwoFieldStructToTimestampNTZConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(SnowflakeType.TIMESTAMP_NTZ.name(), fieldVector, columnIndex, context);
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
  public String toString(int index) {
    if (context.getTimestampNTZFormatter() == null) {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INTERNAL_ERROR, "missing timestamp NTZ formatter");
    }
    try {
      // toString reads no session flags (always UTC): fromToString=true, treatNTZasUTC=false,
      // useSessionTimezone=false.
      Timestamp ts =
          isNull(index) ? null : getTimestamp(index, TimeZone.getDefault(), true, false, false);
      return ts == null
          ? null
          : context.getTimestampNTZFormatter().format(ts, TimeZone.getTimeZone("UTC"), scale);
    } catch (TimestampOperationNotAvailableException e) {
      return e.getSecsSinceEpoch().toPlainString();
    }
  }

  @Override
  public Object toObject(int index) {
    // getObject is the only path that honors JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC (legacy sets the flag
    // only on the getObject path).
    return isNull(index)
        ? null
        : getTimestamp(
            index,
            TimeZone.getDefault(),
            false,
            context.isTreatNTZAsUTC(),
            context.isUseSessionTimezone());
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) {
    if (tz == null) {
      tz = TimeZone.getDefault();
    }
    // getTimestamp path: treatNTZasUTC stays false (legacy never sets it here).
    return isNull(index)
        ? null
        : getTimestamp(index, tz, false, false, context.isUseSessionTimezone());
  }

  private Timestamp getTimestamp(
      int index,
      TimeZone tz,
      boolean fromToString,
      boolean treatNTZasUTC,
      boolean useSessionTimezone) {
    long epoch = epochs.getDataBuffer().getLong((long) index * BigIntVector.TYPE_WIDTH);
    int fraction = fractions.getDataBuffer().getInt((long) index * IntVector.TYPE_WIDTH);
    return getTimestamp(
        epoch,
        fraction,
        tz,
        context.getSessionTimeZone(),
        treatNTZasUTC,
        useSessionTimezone,
        context.isHonorClientTZForTimestampNTZ(),
        fromToString);
  }

  @Override
  public Date toDate(int index, TimeZone tz, boolean dateFormat) {
    return isNull(index)
        ? null
        : new Date(
            getTimestamp(index, TimeZone.getDefault(), false, false, context.isUseSessionTimezone())
                .getTime());
  }

  @Override
  public Time toTime(int index) {
    Timestamp ts = toTimestamp(index, null);
    if (context.isUseSessionTimezone()) {
      ts = toTimestamp(index, context.getSessionTimeZone());
    }
    return ts == null
        ? null
        : new SnowflakeTimeWithTimezone(
            ts, context.getSessionTimeZone(), context.isUseSessionTimezone());
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

  static Timestamp getTimestamp(
      long epoch,
      int fraction,
      TimeZone tz,
      TimeZone sessionTimeZone,
      boolean treatNTZasUTC,
      boolean useSessionTimezone,
      boolean honorClientTZForTimestampNTZ,
      boolean fromToString) {

    if (ArrowResultUtil.isTimestampOverflow(epoch)) {
      if (fromToString) {
        throw new TimestampOperationNotAvailableException(epoch, fraction);
      } else {
        return null;
      }
    }
    Timestamp ts;
    if (treatNTZasUTC || !useSessionTimezone) {
      ts = ArrowResultUtil.createTimestamp(epoch, fraction, TimeZone.getTimeZone("UTC"), true);
    } else {
      ts = ArrowResultUtil.createTimestamp(epoch, fraction, sessionTimeZone, false);
    }

    // Note: honorClientTZForTimestampNTZ is not enabled for toString. If
    // JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC=false, the default is to honor the client timezone for NTZ:
    // move the NTZ wall-clock to the client timezone. useSessionTimezone overrides treatNTZasUTC.
    // Verbatim legacy boolean (Java precedence groups the && pair before ||):
    //   (!fromToString && (honorClientTZForTimestampNTZ && !treatNTZasUTC)) || useSessionTimezone
    if (!fromToString && (honorClientTZForTimestampNTZ && !treatNTZasUTC) || useSessionTimezone) {
      ts = ArrowResultUtil.moveToTimeZone(ts, NTZ, tz);
    }
    return ArrowDateUtil.adjustTimestamp(ts);
  }
}
