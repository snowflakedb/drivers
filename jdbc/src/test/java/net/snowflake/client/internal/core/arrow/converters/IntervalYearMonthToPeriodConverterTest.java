package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.time.Period;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link IntervalYearMonthToPeriodConverter}. Snowflake stores the interval as a
 * signed total-months integer whose physical Arrow width (SB2/SB4/SB8) depends on the declared
 * precision, so the converter reads each width through the same signed accessor. Days in the
 * resulting {@link Period} are always 0.
 */
public class IntervalYearMonthToPeriodConverterTest extends BaseConverterTest {

  private static Map<String, String> intervalFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", SnowflakeType.INTERVAL_YEAR_MONTH.name());
    return meta;
  }

  private Column createSmallIntVector(Short... months) {
    Column vector =
        ColumnBuilder.of("col_interval_ym_sb2", Column.PhysicalKind.INT16, intervalFieldMeta());
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        ColumnBuilder.setNull(vector, i);
      } else {
        ColumnBuilder.set(vector, i, months[i]);
      }
    }
    ColumnBuilder.setValueCount(vector, months.length);
    return vector;
  }

  private Column createIntVector(Integer... months) {
    Column vector =
        ColumnBuilder.of("col_interval_ym_sb4", Column.PhysicalKind.INT32, intervalFieldMeta());
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        ColumnBuilder.setNull(vector, i);
      } else {
        ColumnBuilder.set(vector, i, months[i]);
      }
    }
    ColumnBuilder.setValueCount(vector, months.length);
    return vector;
  }

  private Column createBigIntVector(Long... months) {
    Column vector =
        ColumnBuilder.of("col_interval_ym_sb8", Column.PhysicalKind.INT64, intervalFieldMeta());
    for (int i = 0; i < months.length; i++) {
      if (months[i] == null) {
        ColumnBuilder.setNull(vector, i);
      } else {
        ColumnBuilder.set(vector, i, months[i]);
      }
    }
    ColumnBuilder.setValueCount(vector, months.length);
    return vector;
  }

  @Test
  public void shouldConvertSmallIntMonthsToPeriod() throws Exception {
    // 25 months = 2 years 1 month; 12 months = 1 year exactly; 5 months = 0 years 5 months.
    Column vector = createSmallIntVector((short) 25, (short) 12, (short) 5);
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
    Column vector = createSmallIntVector((short) -14);
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
    // A value beyond the SB2 range exercises the SB4 (Column) backing.
    int months = 40_000; // 3333 years 4 months
    Column vector = createIntVector(months);
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
    // A value beyond the SB4 range exercises the SB8 (Column) backing.
    long months = 3_000_000_000L;
    Column vector = createBigIntVector(months);
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
    Column vector = createSmallIntVector((short) 12, null);
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
    Column vector = createSmallIntVector((short) 25);
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
    Column vector = createIntVector(40_000);
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
    Column vector = createBigIntVector(3_000_000_000L);
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
