package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.Period;
import java.util.TimeZone;
import net.snowflake.client.jdbc.SFException;

public interface ArrowVectorConverter {

  boolean isNull(int index);

  boolean toBoolean(int index) throws SFException;

  byte toByte(int index) throws SFException;

  short toShort(int index) throws SFException;

  int toInt(int index) throws SFException;

  long toLong(int index) throws SFException;

  double toDouble(int index) throws SFException;

  float toFloat(int index) throws SFException;

  byte[] toBytes(int index) throws SFException;

  String toString(int index) throws SFException;

  Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException;

  Time toTime(int index) throws SFException;

  Timestamp toTimestamp(int index, TimeZone tz) throws SFException;

  BigDecimal toBigDecimal(int index) throws SFException;

  Period toPeriod(int index) throws SFException;

  Duration toDuration(int index) throws SFException;

  Object toObject(int index) throws SFException;
}
