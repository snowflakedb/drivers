package net.snowflake.client.internal.core.arrow.converters;

import java.sql.Time;
import java.sql.Timestamp;
import java.time.LocalTime;
import java.util.TimeZone;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import org.apache.arrow.vector.BaseIntVector;
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
  public Time toTime(int index) throws SFException {
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
  public Timestamp toTimestamp(int index, TimeZone tz) throws SFException {
    if (isNull(index)) {
      return null;
    }
    if (context.isUseSessionTimezone()) {
      // Equivalent of snowflake-jdbc's SnowflakeTimestampWithTimezone(millisOfDay, nanos, UTC):
      // a timestamp at 1970-01-01 in UTC carrying the time-of-day with nanosecond precision.
      LocalTime localTime = getLocalTime(index);
      Timestamp ts = new Timestamp(localTime.toNanoOfDay() / MILLIS_PER_NANO);
      ts.setNanos(localTime.getNano());
      return ts;
    }
    return new Timestamp(toTime(index).getTime());
  }

  @Override
  public String toString(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return timeFormatter.format(getLocalTime(index), scale);
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toTime(index);
  }
}
