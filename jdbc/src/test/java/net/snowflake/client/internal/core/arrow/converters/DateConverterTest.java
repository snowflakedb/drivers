package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.sql.Date;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
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

    DateDayVector vector =
        createVector(day00010101, day01000301, day04000229, day15821004, day15821015);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      assertEquals("0001-01-01", converter.toString(0));
      assertEquals("0100-03-01", converter.toString(1));
      assertEquals("0400-02-29", converter.toString(2));
      assertEquals("1582-10-04", converter.toString(3));
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
  public void testToDateIgnoresTimezone() throws Exception {
    int epochDay = (int) LocalDate.of(2024, 1, 15).toEpochDay();
    DateDayVector vector = createVector(epochDay);
    try {
      DateConverter converter = new DateConverter(vector, 0, this);

      // Should return same result regardless of timezone parameter
      Date date1 = converter.toDate(0, TimeZone.getTimeZone("UTC"), false);
      Date date2 = converter.toDate(0, TimeZone.getTimeZone("America/Los_Angeles"), true);
      Date date3 = converter.toDate(0, null, false);

      assertEquals(date1, date2);
      assertEquals(date2, date3);
    } finally {
      vector.close();
    }
  }
}
