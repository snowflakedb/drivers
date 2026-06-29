package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.DateDayVector;
import org.apache.arrow.vector.ValueVector;

/**
 * Converts an Arrow {@link DateDayVector} (epoch days) to JDBC types, mirroring snowflake-jdbc's
 * {@code DateConverter}. Timezone-aware materialization (and the Julian→Gregorian correction for
 * pre-1582-10-05 dates) is gated on {@code JDBC_FORMAT_DATE_WITH_TIMEZONE}; see {@link
 * DataConversionContext}.
 */
public class DateConverter extends AbstractArrowVectorConverter {
  private static final TimeZone TIME_ZONE_UTC = TimeZone.getTimeZone("UTC");

  private final DateDayVector dateVector;

  public DateConverter(ValueVector fieldVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.DATE.name(), fieldVector, columnIndex, context);
    this.dateVector = (DateDayVector) fieldVector;
  }

  private int getEpochDays(int index) {
    return dateVector.get(index);
  }

  private LocalDate getLocalDate(int index) {
    return LocalDate.ofEpochDay(getEpochDays(index));
  }

  private Date getDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return getDate(getEpochDays(index), jvmTz, context.getSessionTimeZone(), useDateFormat);
  }

  /**
   * Mirrors snowflake-jdbc's {@code DateConverter.getDate(int, TimeZone, TimeZone, boolean)}: the
   * timezone shift (and Julian→Gregorian correction) is applied only when a JVM timezone and a
   * session timezone are both present and the date-with-timezone format is requested. Otherwise the
   * raw epoch-day date is returned.
   */
  public static Date getDate(
      int value, TimeZone jvmTz, TimeZone sessionTimeZone, boolean useDateFormat)
      throws SFException {
    if (jvmTz == null || sessionTimeZone == null || !useDateFormat) {
      return ArrowDateUtil.getDate(value);
    }
    return ArrowDateUtil.getDate(value, jvmTz, sessionTimeZone);
  }

  @Override
  public Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException {
    return getDate(index, jvmTz, useDateFormat);
  }

  @Override
  public String toString(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    Date date = getDate(index, TIME_ZONE_UTC, getUseDateFormat(false));
    return date == null ? null : ArrowDateUtil.getDateAsString(date, context.getDateFormatter());
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toDate(index, TimeZone.getDefault(), getUseDateFormat(false));
  }

  @Override
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    Date date = toDate(index, tz, getUseDateFormat(true));
    return date == null ? null : new Timestamp(date.getTime());
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

  /**
   * Whether {@code toString}/{@code toObject}/{@code toTimestamp} should shift DATEs by the
   * session-timezone offset. Mirrors snowflake-jdbc's {@code DateConverter.getUseDateFormat}:
   * returns {@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE && defaultValue}.
   *
   * <p>The shift is gated on the CLIENT property {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} (default
   * {@code false}), which is what legacy honors — NOT the ACCOUNT-level default the server reports
   * (often {@code true}). We therefore avoid {@link
   * DataConversionContext#isFormatDateWithTimezone()}, which reads the server value: using it would
   * over-shift DATEs by one day for zones ahead of UTC. We cannot read the client property yet
   * (TODO SNOW-3243330 in {@link SessionDataConversionContext}), so we keep it at its {@code false}
   * default here. Only {@code getDate(col, Calendar)} honors the runtime flag, via {@link
   * DataConversionContext#isFormatDateWithTimezone()} in the result set.
   */
  private boolean getUseDateFormat(boolean defaultValue) {
    return context.isDefaultFormatDateWithTimezone() && defaultValue;
  }
}
