package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class TinyIntToScaledFixedConverterTest extends BaseConverterTest {

  @Test
  public void shouldConvertScaledTinyIntToFloatUnscaledBytesAndExactOneToTrue() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT8, fixedMeta(2, 1))) {
      ColumnBuilder.set(vector, 0, (byte) 12);
      ColumnBuilder.set(vector, 1, (byte) -25);
      ColumnBuilder.set(vector, 2, (byte) 10);
      ColumnBuilder.setValueCount(vector, 3);

      TinyIntToScaledFixedConverter converter =
          new TinyIntToScaledFixedConverter(vector, 0, this, 1);

      assertEquals(1.2f, converter.toFloat(0), 0.0001f);
      assertEquals(-2.5f, converter.toFloat(1), 0.0001f);
      assertArrayEquals(new byte[] {12}, converter.toBytes(0));
      assertTrue(converter.toBoolean(2));
    }
  }

  @Test
  public void shouldReturnZeroPrimitivesForNullScaledTinyInt() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT8, fixedMeta(2, 1))) {
      ColumnBuilder.setNull(vector, 0);
      ColumnBuilder.setValueCount(vector, 1);

      TinyIntToScaledFixedConverter converter =
          new TinyIntToScaledFixedConverter(vector, 0, this, 1);

      assertEquals((short) 0, converter.toShort(0));
      assertEquals(0, converter.toInt(0));
      assertEquals(0L, converter.toLong(0));
      assertEquals(0.0f, converter.toFloat(0));
      assertEquals(0.0d, converter.toDouble(0));
    }
  }
}
