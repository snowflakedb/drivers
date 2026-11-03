package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class IntToScaledFixedConverterTest extends BaseConverterTest {

  @Test
  public void shouldConvertScaledFixedValue() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "2");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT32, customFieldMeta);
    ColumnBuilder.set(vector, 0, 12_345);
    ColumnBuilder.setNull(vector, 1);
    ColumnBuilder.setValueCount(vector, 2);

    try {
      IntToScaledFixedConverter converter = new IntToScaledFixedConverter(vector, 0, this, 2);
      BigDecimal expected = BigDecimal.valueOf(12_345, 2);
      assertEquals(expected, converter.toBigDecimal(0));
      assertEquals(expected, converter.toObject(0));
      assertEquals(
          String.format(ArrowResultUtil.getStringFormat(2), 123.45), converter.toString(0));
      assertEquals(123.45f, converter.toFloat(0), 0.0001f);
      assertEquals(123.45, converter.toDouble(0), 0.0001);

      assertNull(converter.toBigDecimal(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
      assertEquals(0.0f, converter.toFloat(1));
      assertEquals(0.0, converter.toDouble(1));
      assertEquals((short) 0, converter.toShort(1));
      assertEquals(0, converter.toInt(1));
      assertEquals(0L, converter.toLong(1));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertScaledFixedExactOneToTrue() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "2");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT32, customFieldMeta);
    ColumnBuilder.set(vector, 0, 100);
    ColumnBuilder.setValueCount(vector, 1);

    try {
      IntToScaledFixedConverter converter = new IntToScaledFixedConverter(vector, 0, this, 2);
      assertTrue(converter.toBoolean(0));
    } finally {
      vector.close();
    }
  }
}
