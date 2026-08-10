package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.TimeZone;

public interface ArrowVectorConverter {

  boolean isNull(int index);

  boolean toBoolean(int index);

  byte toByte(int index);

  short toShort(int index);

  int toInt(int index);

  long toLong(int index);

  double toDouble(int index);

  float toFloat(int index);

  byte[] toBytes(int index);

  String toString(int index);

  Date toDate(int index, TimeZone jvmTz, boolean useDateFormat);

  Time toTime(int index);

  Timestamp toTimestamp(int index, TimeZone tz);

  BigDecimal toBigDecimal(int index);

  Period toPeriod(int index);

  Duration toDuration(int index);

  Object toObject(int index);
}
