package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class SmallIntToScaledFixedConverterTest extends BaseConverterTest {

  @Test
  public void shouldConvertScaledSmallIntToFloatUnscaledBytesAndExactOneToTrue() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT16, fixedMeta(4, 2))) {
      ColumnBuilder.set(vector, 0, (short) 1234);
      ColumnBuilder.set(vector, 1, (short) -505);
      ColumnBuilder.set(vector, 2, (short) 100);
      ColumnBuilder.setValueCount(vector, 3);

      SmallIntToScaledFixedConverter converter =
          new SmallIntToScaledFixedConverter(vector, 0, this, 2);

      assertEquals(12.34f, converter.toFloat(0), 0.0001f);
      assertEquals(-5.05f, converter.toFloat(1), 0.0001f);
      assertArrayEquals(
          ByteBuffer.allocate(Short.BYTES).putShort(0, (short) 1234).array(), converter.toBytes(0));
      assertTrue(converter.toBoolean(2));
    }
  }

  @Test
  public void shouldReturnZeroPrimitivesForNullScaledSmallInt() {
    try (Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT16, fixedMeta(4, 2))) {
      ColumnBuilder.setNull(vector, 0);
      ColumnBuilder.setValueCount(vector, 1);

      SmallIntToScaledFixedConverter converter =
          new SmallIntToScaledFixedConverter(vector, 0, this, 2);

      assertEquals((short) 0, converter.toShort(0));
      assertEquals(0, converter.toInt(0));
      assertEquals(0L, converter.toLong(0));
      assertEquals(0.0f, converter.toFloat(0));
      assertEquals(0.0d, converter.toDouble(0));
    }
  }
}
