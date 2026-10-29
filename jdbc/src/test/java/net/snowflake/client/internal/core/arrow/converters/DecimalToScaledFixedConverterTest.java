package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.time.Duration;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class DecimalToScaledFixedConverterTest extends BaseConverterTest {
  private static final int PRECISION = 38;
  private static final int SCALE = 0;
  private static final long NANOS_PER_SECOND = 1_000_000_000L;

  private static Map<String, String> fixedFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "FIXED");
    meta.put("precision", String.valueOf(PRECISION));
    meta.put("scale", String.valueOf(SCALE));
    return meta;
  }

  /** A FIXED column whose decimal values are nanosecond counts (INTERVAL DAY TO SECOND). */
  private Column createNanosVector(BigDecimal... nanos) {
    Column vector = ColumnBuilder.decimal("col_interval", PRECISION, SCALE, fixedFieldMeta());
    for (int i = 0; i < nanos.length; i++) {
      if (nanos[i] == null) {
        ColumnBuilder.setNull(vector, i);
      } else {
        ColumnBuilder.set(vector, i, nanos[i]);
      }
    }
    ColumnBuilder.setValueCount(vector, nanos.length);
    return vector;
  }

  @Test
  public void shouldConvertNanosToDuration() throws Exception {
    // 1.5 seconds = 1_500_000_000 nanos.
    Column vector = createNanosVector(BigDecimal.valueOf(NANOS_PER_SECOND + 500_000_000L));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertEquals(Duration.ofSeconds(1, 500_000_000L), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertNegativeNanosToNegativeDuration() throws Exception {
    Column vector = createNanosVector(BigDecimal.valueOf(-(NANOS_PER_SECOND + 500_000_000L)));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertEquals(Duration.ofSeconds(1, 500_000_000L).negated(), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullDurationForNullIndex() throws Exception {
    Column vector = createNanosVector((BigDecimal) null);
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertNull(converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldThrowWhenDurationSecondsOverflowsLong() throws Exception {
    // 1e29 nanos / 1e9 = 1e20 seconds, which does not fit in a long.
    Column vector = createNanosVector(new BigDecimal("100000000000000000000000000000"));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchDecimalVectorToScaledFixedConverter() throws Exception {
    Column vector = createNanosVector(BigDecimal.valueOf(NANOS_PER_SECOND));
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(DecimalToScaledFixedConverter.class, converter);
      assertEquals(Duration.ofSeconds(1), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertScaleZeroDecimalToPrimitivesAndRejectOutOfRange() throws Exception {
    Column vector =
        createNanosVector(
            BigDecimal.ZERO,
            BigDecimal.ONE,
            BigDecimal.valueOf(127L),
            BigDecimal.valueOf(128L),
            BigDecimal.valueOf(32_768L),
            BigDecimal.valueOf(2_147_483_648L),
            new BigDecimal("9223372036854775808"),
            null);
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);

      assertFalse(converter.toBoolean(0));
      assertTrue(converter.toBoolean(1));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(2));

      assertEquals((byte) 127, converter.toByte(2));
      assertEquals((short) 127, converter.toShort(2));
      assertEquals(127, converter.toInt(2));
      assertEquals(127L, converter.toLong(2));
      assertEquals(127.0f, converter.toFloat(2));
      assertEquals(127.0d, converter.toDouble(2));
      assertEquals(BigDecimal.valueOf(127L), converter.toObject(2));
      assertEquals("127", converter.toString(2));
      assertArrayEquals(new byte[] {127}, converter.toBytes(2));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toByte(3));
      assertEquals((short) 128, converter.toShort(3));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(4));
      assertEquals(32_768, converter.toInt(4));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(5));
      assertEquals(2_147_483_648L, converter.toLong(5));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(6));

      assertFalse(converter.toBoolean(7));
      assertEquals((byte) 0, converter.toByte(7));
      assertEquals((short) 0, converter.toShort(7));
      assertEquals(0, converter.toInt(7));
      assertEquals(0L, converter.toLong(7));
      assertEquals(0.0f, converter.toFloat(7));
      assertEquals(0.0d, converter.toDouble(7));
      assertNull(converter.toBigDecimal(7));
      assertNull(converter.toObject(7));
      assertNull(converter.toString(7));
      assertNull(converter.toBytes(7));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldRejectIntegralConversionsOnNonZeroScaleDecimalAndReloadInPlace()
      throws Exception {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "FIXED");
    meta.put("precision", "18");
    meta.put("scale", "2");
    Column vector = ColumnBuilder.decimal("col_dec_scaled", 18, 2, meta);
    try {
      ColumnBuilder.set(vector, 0, new BigDecimal("12.34"));
      ColumnBuilder.set(vector, 1, new BigDecimal("1.00"));
      ColumnBuilder.set(vector, 2, new BigDecimal("0.00"));
      ColumnBuilder.setValueCount(vector, 3);

      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);

      assertEquals(new BigDecimal("12.34"), converter.toBigDecimal(0));
      assertEquals("12.34", converter.toString(0));
      assertEquals(12.34f, converter.toFloat(0), 0.001f);
      assertEquals(12.34d, converter.toDouble(0), 0.001d);
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toByte(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));

      assertTrue(converter.toBoolean(1));
      assertFalse(converter.toBoolean(2));

      ColumnBuilder.set(vector, 0, new BigDecimal("-99.75"));
      assertEquals(
          new BigDecimal("-99.75"),
          converter.toBigDecimal(0),
          "decimal converter must read reloaded values from the reused vector");
      assertEquals("-99.75", converter.toString(0));
    } finally {
      vector.close();
    }
  }
}
