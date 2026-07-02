package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.nio.ByteBuffer;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.ResourceLock;
import org.junit.jupiter.api.parallel.Resources;

// A test here mutates the JVM default timezone; lock the shared TIME_ZONE resource so the class is
// serialized against other timezone-sensitive tests if parallel execution is ever enabled.
@ResourceLock(Resources.TIME_ZONE)
public class TimeConverterTest extends BaseConverterTest {
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  private static Map<String, String> timeFieldMeta(int scale) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "TIME");
    meta.put("scale", String.valueOf(scale));
    return meta;
  }

  private IntVector createIntVector(int scale, int... values) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, timeFieldMeta(scale));
    IntVector vector = new IntVector("col_time", fieldType, allocator);
    for (int i = 0; i < values.length; i++) {
      vector.setSafe(i, values[i]);
    }
    vector.setValueCount(values.length);
    return vector;
  }

  private BigIntVector createBigIntVector(int scale, long... values) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.BIGINT.getType(), null, timeFieldMeta(scale));
    BigIntVector vector = new BigIntVector("col_time", fieldType, allocator);
    for (int i = 0; i < values.length; i++) {
      vector.setSafe(i, values[i]);
    }
    vector.setValueCount(values.length);
    return vector;
  }

  @Test
  public void shouldConvertScale0SecondsToTime() throws Exception {
    // 12:34:56 → 45296 seconds since midnight
    IntVector vector = createIntVector(0, 45296, 0, 86399);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertEquals(new Time(45296L * 1000L), converter.toTime(0));
      assertEquals(new Time(0L), converter.toTime(1));
      assertEquals(new Time(86399L * 1000L), converter.toTime(2));

      assertEquals("12:34:56", converter.toString(0));
      assertEquals("00:00:00", converter.toString(1));
      assertEquals("23:59:59", converter.toString(2));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertScale3MillisecondsToTime() throws Exception {
    // TIME(3): 12:34:56.789 → (45296 * 1000 + 789) units of milliseconds since midnight
    long unitsScale3 = 45296L * 1000L + 789L;
    BigIntVector vector = createBigIntVector(3, unitsScale3);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 3);
      // Time has millisecond resolution; nanos-of-day = (45296000 + 789) * 10^6 → millis = 45296789
      assertEquals(new Time(45296_789L), converter.toTime(0));
      // Default formatter is HH:mm:ss; fractional seconds are not shown.
      assertEquals("12:34:56", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldTruncateScale9NanosecondsToMillis() throws Exception {
    // TIME(9): 12:34:56.123456789 → 45296 * 10^9 + 123456789
    long unitsScale9 = 45296L * 1_000_000_000L + 123_456_789L;
    BigIntVector vector = createBigIntVector(9, unitsScale9);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 9);
      // java.sql.Time has millisecond resolution: 45296123 ms.
      assertEquals(new Time(45296_123L), converter.toTime(0));
      assertEquals("12:34:56", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullIndex() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, timeFieldMeta(0));
    IntVector vector = new IntVector("col_time", fieldType, allocator);
    vector.setSafe(0, 45296);
    vector.setNull(1);
    vector.setValueCount(2);

    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertEquals(new Time(45296L * 1000L), converter.toTime(0));
      assertNull(converter.toTime(1));
      assertNull(converter.toString(1));
      assertNull(converter.toObject(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnTimeFromToObject() throws Exception {
    IntVector vector = createIntVector(0, 45296);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      Object obj = converter.toObject(0);
      assertInstanceOf(Time.class, obj);
      assertEquals(new Time(45296L * 1000L), obj);
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchIntVectorConverterViaUtil() throws Exception {
    IntVector vector = createIntVector(0, 45296);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(TimeConverter.class, converter);
      assertEquals(new Time(45296L * 1000L), converter.toTime(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchBigIntVectorConverterViaUtil() throws Exception {
    BigIntVector vector = createBigIntVector(9, 45296L * 1_000_000_000L);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(TimeConverter.class, converter);
      assertEquals(new Time(45296L * 1000L), converter.toTime(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldFormatToStringHonoringContextFormat() throws Exception {
    long unitsScale9 = 45296L * 1_000_000_000L + 123_456_789L;
    BigIntVector vector = createBigIntVector(9, unitsScale9);
    DataConversionContext ctx =
        new DataConversionContext() {
          @Override
          public SnowflakeDateTimeFormat getTimeFormatter() {
            return SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI:SS.FF3");
          }
        };
    try {
      TimeConverter converter = new TimeConverter(vector, 0, ctx, 9);
      assertEquals("12:34:56.123", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  private static DataConversionContext sessionTimezoneContext() {
    return new DataConversionContext() {
      @Override
      public boolean isUseSessionTimezone() {
        return true;
      }
    };
  }

  @Test
  public void shouldAnchorWallClockInJvmTimezoneWhenUseSessionTimezone() throws Exception {
    // With JDBC_USE_SESSION_TIMEZONE the wall-clock fields are anchored in the JVM default TZ, so
    // Time#toString() (which renders in the local TZ) reads back the original time-of-day.
    TimeZone original = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Asia/Tokyo"));
    IntVector vector = createIntVector(0, 45296); // 12:34:56
    try {
      TimeConverter sessionTz = new TimeConverter(vector, 0, sessionTimezoneContext(), 0);
      assertEquals("12:34:56", sessionTz.toTime(0).toString());

      // Default (UTC-anchored) path renders shifted by the TZ offset under a non-UTC zone.
      TimeConverter utc = new TimeConverter(vector, 0, this, 0);
      assertEquals("21:34:56", utc.toTime(0).toString());
    } finally {
      vector.close();
      TimeZone.setDefault(original);
    }
  }

  @Test
  public void shouldConvertToTimestampByDefault() throws Exception {
    long unitsScale3 = 45296L * 1000L + 789L; // 12:34:56.789
    BigIntVector vector = createBigIntVector(3, unitsScale3);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 3);
      // Default path: new Timestamp(toTime().getTime()) == new Timestamp(millisOfDay).
      assertEquals(new Timestamp(45296_789L), converter.toTimestamp(0, null));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldPreserveNanosInTimestampWhenUseSessionTimezone() throws Exception {
    long unitsScale9 = 45296L * 1_000_000_000L + 123_456_789L; // 12:34:56.123456789
    BigIntVector vector = createBigIntVector(9, unitsScale9);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, sessionTimezoneContext(), 9);
      Timestamp ts = converter.toTimestamp(0, null);
      // Anchored at 1970-01-01 in UTC, carrying full nanosecond precision.
      assertEquals(45296_123L, ts.getTime());
      assertEquals(123_456_789, ts.getNanos());
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullTimestampForNullIndex() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, timeFieldMeta(0));
    IntVector vector = new IntVector("col_time", fieldType, allocator);
    vector.setNull(0);
    vector.setValueCount(1);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertNull(converter.toTimestamp(0, null));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnFalseBooleanForNullIndex() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, timeFieldMeta(0));
    IntVector vector = new IntVector("col_time", fieldType, allocator);
    vector.setNull(0);
    vector.setValueCount(1);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertFalse(converter.toBoolean(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldThrowWhenConvertingTimeToBoolean() throws Exception {
    IntVector vector = createIntVector(0, 45296);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullBytesForNullIndex() throws Exception {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, timeFieldMeta(0));
    IntVector vector = new IntVector("col_time", fieldType, allocator);
    vector.setNull(0);
    vector.setValueCount(1);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertNull(converter.toBytes(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnRawBigEndianBytesForIntBackedTime() throws Exception {
    // 45296 seconds since midnight, big-endian 4-byte image (matches snowflake-jdbc IntToTime).
    IntVector vector = createIntVector(0, 45296);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 0);
      assertArrayEquals(
          ByteBuffer.allocate(Integer.BYTES).putInt(45296).array(), converter.toBytes(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldThrowWhenConvertingBigIntBackedTimeToBytes() throws Exception {
    // snowflake-jdbc's BigIntToTimeConverter does not implement toBytes; BIGINT-backed TIME stays
    // an unsupported conversion.
    BigIntVector vector = createBigIntVector(9, 45296L * 1_000_000_000L);
    try {
      TimeConverter converter = new TimeConverter(vector, 0, this, 9);
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBytes(0));
    } finally {
      vector.close();
    }
  }
}
