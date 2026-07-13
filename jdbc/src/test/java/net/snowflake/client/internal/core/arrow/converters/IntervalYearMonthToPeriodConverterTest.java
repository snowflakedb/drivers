package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.time.Period;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.SmallIntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link IntervalYearMonthToPeriodConverter}. Snowflake stores the interval as a
 * signed total-months integer whose physical Arrow width (SB2/SB4/SB8) depends on the declared
 * precision, so the converter must handle {@link SmallIntVector}, {@link IntVector}, and {@link
 * BigIntVector} backings. Days in the resulting {@link Period} are always 0.
 */
public class IntervalYearMonthToPeriodConverterTest extends BaseConverterTest {
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  private static Map<String, String> intervalFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", SnowflakeType.INTERVAL_YEAR_MONTH.name());
    return meta;
  }

  private SmallIntVector createSmallIntVector(Short... months) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, intervalFieldMeta());
    SmallIntVector vector = new SmallIntVector("col_interval_ym_sb2", fieldType, allocator);
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        vector.setNull(i);
      } else {
        vector.setSafe(i, months[i]);
      }
    }
    vector.setValueCount(months.length);
    return vector;
  }

  private IntVector createIntVector(Integer... months) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.INT.getType(), null, intervalFieldMeta());
    IntVector vector = new IntVector("col_interval_ym_sb4", fieldType, allocator);
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        vector.setNull(i);
      } else {
        vector.setSafe(i, months[i]);
      }
    }
    vector.setValueCount(months.length);
    return vector;
  }

  private BigIntVector createBigIntVector(Long... months) {
    FieldType fieldType =
        new FieldType(true, Types.MinorType.BIGINT.getType(), null, intervalFieldMeta());
    BigIntVector vector = new BigIntVector("col_interval_ym_sb8", fieldType, allocator);
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        vector.setNull(i);
      } else {
        vector.setSafe(i, months[i]);
      }
    }
    vector.setValueCount(months.length);
    return vector;
  }

  @Test
  public void shouldConvertSmallIntMonthsToPeriod() throws Exception {
    // 25 months = 2 years 1 month; 12 months = 1 year exactly; 5 months = 0 years 5 months.
    SmallIntVector vector = createSmallIntVector((short) 25, (short) 12, (short) 5);
    try {
      IntervalYearMonthToPeriodConverter converter =
          new IntervalYearMonthToPeriodConverter(vector, 0, this);

      assertEquals(Period.of(2, 1, 0), converter.toPeriod(0));
      assertEquals(Period.of(1, 0, 0), converter.toPeriod(1));
      assertEquals(Period.of(0, 5, 0), converter.toPeriod(2));
      assertEquals(Period.of(2, 1, 0), converter.toObject(0));
      assertEquals(Period.of(2, 1, 0).toString(), converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertNegativeSmallIntMonthsToPeriod() throws Exception {
    // -14 months = -1 year -2 months (Java integer division truncates toward zero).
    SmallIntVector vector = createSmallIntVector((short) -14);
    try {
      IntervalYearMonthToPeriodConverter converter =
          new IntervalYearMonthToPeriodConverter(vector, 0, this);

      assertEquals(Period.of(-1, -2, 0), converter.toPeriod(0));
      assertEquals("P-1Y-2M", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertIntMonthsToPeriod() throws Exception {
    // A value beyond the SB2 range exercises the SB4 (IntVector) backing.
    int months = 40_000; // 3333 years 4 months
    IntVector vector = createIntVector(months);
    try {
      IntervalYearMonthToPeriodConverter converter =
          new IntervalYearMonthToPeriodConverter(vector, 0, this);

      assertEquals(Period.of(months / 12, months % 12, 0), converter.toPeriod(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertBigIntMonthsToPeriod() throws Exception {
    // A value beyond the SB4 range exercises the SB8 (BigIntVector) backing.
    long months = 3_000_000_000L;
    BigIntVector vector = createBigIntVector(months);
    try {
      IntervalYearMonthToPeriodConverter converter =
          new IntervalYearMonthToPeriodConverter(vector, 0, this);

      assertEquals(Period.of((int) (months / 12), (int) (months % 12), 0), converter.toPeriod(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullInterval() throws Exception {
    SmallIntVector vector = createSmallIntVector((short) 12, null);
    try {
      IntervalYearMonthToPeriodConverter converter =
          new IntervalYearMonthToPeriodConverter(vector, 0, this);

      assertNull(converter.toPeriod(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchSmallIntViaConverterUtil() throws Exception {
    SmallIntVector vector = createSmallIntVector((short) 25);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertInstanceOf(IntervalYearMonthToPeriodConverter.class, converter);
      assertEquals(Period.of(2, 1, 0), converter.toPeriod(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchIntViaConverterUtil() throws Exception {
    IntVector vector = createIntVector(40_000);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertInstanceOf(IntervalYearMonthToPeriodConverter.class, converter);
      assertEquals(Period.of(40_000 / 12, 40_000 % 12, 0), converter.toPeriod(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchBigIntViaConverterUtil() throws Exception {
    BigIntVector vector = createBigIntVector(3_000_000_000L);
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertInstanceOf(IntervalYearMonthToPeriodConverter.class, converter);
      assertEquals(
          Period.of((int) (3_000_000_000L / 12), (int) (3_000_000_000L % 12), 0),
          converter.toPeriod(0));
    } finally {
      vector.close();
    }
  }
}
