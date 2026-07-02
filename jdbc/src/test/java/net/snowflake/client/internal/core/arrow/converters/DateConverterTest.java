package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Timestamp;
import java.text.SimpleDateFormat;
import java.time.LocalDate;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.DateDayVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class DateConverterTest extends BaseConverterTest {
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  private static Map<String, String> dateFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "DATE");
    return meta;
  }

  private DateDayVector createVector(int... epochDays) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.DATEDAY.getType(), null, dateFieldMeta());
    DateDayVector vector = new DateDayVector("col_date", fieldType, allocator);
    for (int i = 0; i < epochDays.length; i++) {
      vector.setSafe(i, epochDays[i]);
    }
    vector.setValueCount(epochDays.length);
    return vector;
  }

  @Test
  public void testModernDates() throws Exception {
    // 2024-01-15 = epoch day 19737, 1970-01-01 = 0, 1999-12-31 = 10956
    int day20240115 = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    int day19700101 = 0;
    int day19991231 = (int) LocalDate.of(1999, 12, 31).toEpochDay();

    DateDayVector vector = createVector(day20240115, day19700101, day19991231);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals(Date.valueOf("2024-01-15"), converter.toDate(0, null, false));
      assertEquals(Date.valueOf("1970-01-01"), converter.toDate(1, null, false));
      assertEquals(Date.valueOf("1999-12-31"), converter.toDate(2, null, false));

      assertEquals("2024-01-15", converter.toString(0));
      assertEquals("1970-01-01", converter.toString(1));
      assertEquals("1999-12-31", converter.toString(2));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testHistoricalDates() throws Exception {
    int day00010101 = (int) LocalDate.of(1, 1, 1).toEpochDay();
    int day01000301 = (int) LocalDate.of(100, 3, 1).toEpochDay();
    int day04000229 = (int) LocalDate.of(400, 2, 29).toEpochDay();
    int day15821004 = (int) LocalDate.of(1582, 10, 4).toEpochDay();
    int day15821015 = (int) LocalDate.of(1582, 10, 15).toEpochDay();
    int[] days = {day00010101, day01000301, day04000229, day15821004, day15821015};

    DateDayVector vector = createVector(days);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      // With JDBC_FORMAT_DATE_WITH_TIMEZONE=false (default), toString renders the raw epoch-day
      // date via the session DATE_OUTPUT_FORMAT formatter, exactly as snowflake-jdbc's
      // ResultUtil.getDateAsString does. The GregorianCalendar cutover (pre-1582 dates display in
      // the Julian calendar) is part of that contract. Use a plain SimpleDateFormat as an
      // INDEPENDENT oracle (it shares GregorianCalendar's cutover but not SnowflakeDateTimeFormat's
      // SQL->java pattern translation), so a regression in getDateAsString or the formatter is
      // caught instead of masked by self-delegation.
      SimpleDateFormat referenceFormat = new SimpleDateFormat("yyyy-MM-dd");
      for (int i = 0; i < days.length; i++) {
        String expected = referenceFormat.format(ArrowDateUtil.getDate(days[i]));
        assertEquals(expected, converter.toString(i), "toString mismatch for epoch day " + days[i]);
      }

      // Anchor the contract with a literal: 1582-10-15 is the first Gregorian day, so it renders
      // unchanged regardless of the JVM default timezone.
      assertEquals("1582-10-15", converter.toString(4));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testNullHandling() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.DATEDAY.getType(), null, dateFieldMeta());
    DateDayVector vector = new DateDayVector("col_date", fieldType, allocator);
    vector.setSafe(0, (int) LocalDate.of(2024, 1, 15).toEpochDay());
    vector.setNull(1);
    vector.setValueCount(2);

    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals(Date.valueOf("2024-01-15"), converter.toDate(0, null, false));
      assertNull(converter.toDate(1, null, false));
      assertNull(converter.toString(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toBigDecimal(1));
      assertEquals(0, converter.toInt(1));
      assertEquals(0L, converter.toLong(1));
      assertEquals(0.0f, converter.toFloat(1));
      assertEquals(0.0, converter.toDouble(1));
      assertEquals(0, converter.toShort(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToObject() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      Object obj = converter.toObject(0);
      assertInstanceOf(Date.class, obj);
      assertEquals(Date.valueOf("2024-01-15"), obj);
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToTimestamp() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      Timestamp ts = converter.toTimestamp(0, null);
      assertEquals(Timestamp.valueOf("2024-01-15 00:00:00"), ts);
    } finally {
      vector.close();
    }
  }

  /**
   * Locks the snowflake-jdbc behavior surfaced by the DateTimeParityTest: with the default "format
   * date with timezone" session default (true), the runtime JDBC_FORMAT_DATE_WITH_TIMEZONE flag
   * must NOT shift toObject/toString (they use the simple epoch-day date), while toTimestamp with
   * an explicit timezone always shifts by the session-vs-passed-timezone offset and toTimestamp
   * with a null timezone stays simple.
   */
  @Test
  public void shouldNotShiftObjectOrStringWhenFormatFlagSetButTimestampHonorsCalendar()
      throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    TimeZone sessionTz = TimeZone.getTimeZone("Asia/Tokyo");
    TimeZone utc = TimeZone.getTimeZone("UTC");
    // Runtime flag on (profile DATE_WITH_TZ=t) but the connection-time default stays true,
    // mirroring
    // SFBaseSession.defaultFormatDateWithTimezone.
    DataConversionContext ctx =
        new DataConversionContext() {
          @Override
          public TimeZone getSessionTimeZone() {
            return sessionTz;
          }

          @Override
          public boolean isFormatDateWithTimezone() {
            return true;
          }
        };
    try {
      DateConverter converter = new DateConverter(vector, 0, ctx);

      // toObject / toString: no timezone shift despite the runtime flag.
      assertEquals(ArrowDateUtil.getDate(epochDay), converter.toObject(0));
      assertEquals(
          ArrowDateUtil.getDateAsString(ArrowDateUtil.getDate(epochDay), ctx.getDateFormatter()),
          converter.toString(0));

      // toTimestamp(null): simple, no shift.
      assertEquals(
          new Timestamp(ArrowDateUtil.getDate(epochDay).getTime()), converter.toTimestamp(0, null));

      // toTimestamp(UTC): always shifts by the UTC-vs-session offset (independent of the flag).
      assertEquals(
          new Timestamp(ArrowDateUtil.getDate(epochDay, utc, sessionTz).getTime()),
          converter.toTimestamp(0, utc));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testNumericConversions() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals(epochDay, converter.toInt(0));
      assertEquals((long) epochDay, converter.toLong(0));
      assertEquals((float) epochDay, converter.toFloat(0));
      assertEquals((double) epochDay, converter.toDouble(0));
      assertEquals(BigDecimal.valueOf(epochDay), converter.toBigDecimal(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToShortOverflow() throws Exception {
    // Epoch day for 2024-01-15 is 19737, which is within Short.MAX_VALUE (32767),
    // so use a later date whose epoch day definitely overflows a short
    int epochDay = (int) LocalDate.of(2060, 1, 1).toEpochDay(); // ~32873, > Short.MAX_VALUE
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToShortInRange() throws Exception {
    // Epoch day 100 (1970-04-11) is well within short range
    DateDayVector vector = createVector(100);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals((short) 100, converter.toShort(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToBooleanThrows() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testConverterDispatchViaUtil() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.DATEDAY.getType(), null, dateFieldMeta());
    DateDayVector vector = new DateDayVector("col_date", fieldType, allocator);
    vector.setSafe(0, (int) LocalDate.of(2024, 1, 15).toEpochDay());
    vector.setValueCount(1);

    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(DateConverter.class, converter);
      assertEquals(Date.valueOf("2024-01-15"), converter.toDate(0, null, false));
    } finally {
      vector.close();
    }
  }

  @Test
  public void testToStringDefaultFormat() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals("2024-01-15", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldIgnoreTimezoneWhenNotFormatted() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      // With useDateFormat=false the timezone argument is ignored: every variant returns the raw
      // epoch-day date (mirrors snowflake-jdbc's getDate(value) fast path).
      Date date1 = converter.toDate(0, TimeZone.getTimeZone("UTC"), false);
      Date date2 = converter.toDate(0, TimeZone.getTimeZone("America/Los_Angeles"), false);
      Date date3 = converter.toDate(0, null, false);

      assertEquals(ArrowDateUtil.getDate(epochDay), date1);
      assertEquals(date1, date2);
      assertEquals(date2, date3);
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldApplyTimezoneShiftWhenFormatted() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    // Session timezone Asia/Tokyo, JVM-side request UTC: a non-zero offset must be applied,
    // matching ArrowDateUtil.getDate(day, jvmTz, sessionTz).
    TimeZone sessionTz = TimeZone.getTimeZone("Asia/Tokyo");
    DataConversionContext ctx =
        new DataConversionContext() {
          @Override
          public TimeZone getSessionTimeZone() {
            return sessionTz;
          }

          @Override
          public boolean isFormatDateWithTimezone() {
            return true;
          }
        };
    try {
      DateConverter converter = new DateConverter(vector, 0, ctx);

      Date shifted = converter.toDate(0, TimeZone.getTimeZone("UTC"), true);
      assertEquals(
          ArrowDateUtil.getDate(epochDay, TimeZone.getTimeZone("UTC"), sessionTz), shifted);
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldHonorSessionDateFormatInToString() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    SnowflakeDateTimeFormat monFormat = SnowflakeDateTimeFormat.fromSqlFormat("DD-MON-YYYY");
    DataConversionContext ctx =
        new DataConversionContext() {
          @Override
          public SnowflakeDateTimeFormat getDateFormatter() {
            return monFormat;
          }
        };
    try {
      DateConverter converter = new DateConverter(vector, 0, ctx);

      // toString must route through the session DATE_OUTPUT_FORMAT formatter, not a hard-coded
      // ISO pattern.
      String expected = ArrowDateUtil.getDateAsString(ArrowDateUtil.getDate(epochDay), monFormat);
      assertEquals(expected, converter.toString(0));
      // Sanity: the custom format differs from the default ISO rendering.
      assertNotEquals("2024-01-15", expected);
    } finally {
      vector.close();
    }
  }
}
