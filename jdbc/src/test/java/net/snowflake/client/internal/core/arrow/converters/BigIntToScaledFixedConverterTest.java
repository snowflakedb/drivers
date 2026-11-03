package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class BigIntToScaledFixedConverterTest extends BaseConverterTest {

  @Test
  public void shouldConvertScaledBigIntToFloatDoubleUnscaledBytesAndExactOneToTrue() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT64, fixedMeta(18, 3))) {
      ColumnBuilder.set(vector, 0, 123_456L);
      ColumnBuilder.set(vector, 1, -9_876L);
      ColumnBuilder.set(vector, 2, 1_000L);
      ColumnBuilder.setValueCount(vector, 3);

      BigIntToScaledFixedConverter converter = new BigIntToScaledFixedConverter(vector, 0, this, 3);

      assertEquals(123.456f, converter.toFloat(0), 0.001f);
      assertEquals(123.456d, converter.toDouble(0), 0.0001d);
      assertEquals(-9.876d, converter.toDouble(1), 0.0001d);
      assertArrayEquals(
          ByteBuffer.allocate(Long.BYTES).putLong(0, 123_456L).array(), converter.toBytes(0));
      assertTrue(converter.toBoolean(2));
    }
  }

  @Test
  public void shouldReturnZeroPrimitivesForNullScaledBigInt() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT64, fixedMeta(18, 3))) {
      ColumnBuilder.setNull(vector, 0);
      ColumnBuilder.setValueCount(vector, 1);

      BigIntToScaledFixedConverter converter = new BigIntToScaledFixedConverter(vector, 0, this, 3);

      assertEquals((short) 0, converter.toShort(0));
      assertEquals(0, converter.toInt(0));
      assertEquals(0L, converter.toLong(0));
      assertEquals(0.0f, converter.toFloat(0));
      assertEquals(0.0d, converter.toDouble(0));
    }
  }

  @Test
  public void shouldConvertHighScaleBigIntToDoubleViaBigDecimalWhenScaleExceedsNine() {
    // ArrowResultUtil.powerOfTen only tabulates scales 0..9.
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT64, fixedMeta(18, 12))) {
      ColumnBuilder.set(vector, 0, 1_500_000_000_000L);
      ColumnBuilder.setValueCount(vector, 1);

      BigIntToScaledFixedConverter converter =
          new BigIntToScaledFixedConverter(vector, 0, this, 12);

      assertEquals(1.5d, converter.toDouble(0), 1e-12);
      assertEquals(1.5f, converter.toFloat(0), 1e-6f);
      assertEquals("1.500000000000", converter.toString(0));
    }
  }
}
