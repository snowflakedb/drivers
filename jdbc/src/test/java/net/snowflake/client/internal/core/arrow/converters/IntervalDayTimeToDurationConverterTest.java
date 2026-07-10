package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.time.Duration;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link IntervalDayTimeToDurationConverter}. The underlying Arrow vector is always
 * a signed {@code Int64} holding the total number of nanoseconds in the interval.
 */
public class IntervalDayTimeToDurationConverterTest extends BaseConverterTest {
  private static final long NANOS_IN_SECOND = 1_000_000_000L;
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  private static Map<String, String> intervalFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", SnowflakeType.INTERVAL_DAY_TIME.name());
    return meta;
  }

  private BigIntVector createVector(Long... nanos) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.BIGINT.getType(), null, intervalFieldMeta());
    BigIntVector vector = new BigIntVector("col_interval_day_time", fieldType, allocator);
    for (int i = 0; i < nanos.length; i++) {
      if (nanos[i] == null) {
        vector.setNull(i);
      } else {
        vector.setSafe(i, nanos[i]);
      }
    }
    vector.setValueCount(nanos.length);
    return vector;
  }

  @Test
  public void shouldConvertPositiveIntervalToDuration() throws Exception {
    // 1 day 2 hours 3 minutes 4.5 seconds expressed in nanoseconds.
    long totalNanos = (((24L + 2) * 3600) + (3 * 60) + 4) * NANOS_IN_SECOND + 500_000_000L;
    BigIntVector vector = createVector(totalNanos);
    try {
      IntervalDayTimeToDurationConverter converter =
          new IntervalDayTimeToDurationConverter(vector, 0, this);

      Duration expected = Duration.ofNanos(totalNanos);
      assertEquals(expected, converter.toDuration(0));
      assertEquals(expected, converter.toObject(0));
      assertEquals(expected.toString(), converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertNegativeIntervalToDuration() throws Exception {
    // Snowflake stores negative intervals as negative total-nanos; the converter must round-trip
    // to a negative Duration (it negates a positive Duration to avoid Duration.ofSeconds overflow).
    long totalNanos = -(90L * NANOS_IN_SECOND + 250_000_000L); // -1 min 30.25 s
    BigIntVector vector = createVector(totalNanos);
    try {
      IntervalDayTimeToDurationConverter converter =
          new IntervalDayTimeToDurationConverter(vector, 0, this);

      Duration expected = Duration.ofNanos(totalNanos);
      assertEquals(expected, converter.toDuration(0));
      assertTrue(
          converter.toDuration(0).isNegative(), "negative interval must yield negative Duration");
      assertEquals("PT-1M-30.25S", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertZeroIntervalToZeroDuration() throws Exception {
    BigIntVector vector = createVector(0L);
    try {
      IntervalDayTimeToDurationConverter converter =
          new IntervalDayTimeToDurationConverter(vector, 0, this);

      assertEquals(Duration.ZERO, converter.toDuration(0));
      assertEquals("PT0S", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertSubSecondIntervalToDuration() throws Exception {
    // A value smaller than one second must not lose its nanosecond fraction.
    long totalNanos = 123_456_789L;
    BigIntVector vector = createVector(totalNanos);
    try {
      IntervalDayTimeToDurationConverter converter =
          new IntervalDayTimeToDurationConverter(vector, 0, this);

      Duration duration = converter.toDuration(0);
      assertEquals(0, duration.getSeconds());
      assertEquals(123_456_789, duration.getNano());
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullInterval() throws Exception {
    BigIntVector vector = createVector(NANOS_IN_SECOND, null);
    try {
      IntervalDayTimeToDurationConverter converter =
          new IntervalDayTimeToDurationConverter(vector, 0, this);

      assertNull(converter.toDuration(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchViaConverterUtil() throws Exception {
    BigIntVector vector = createVector(5L * NANOS_IN_SECOND);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertInstanceOf(IntervalDayTimeToDurationConverter.class, converter);
      assertEquals(Duration.ofSeconds(5), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }
}
