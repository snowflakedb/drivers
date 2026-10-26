package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

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

      assertNull(converter.toBigDecimal(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    } finally {
      vector.close();
    }
  }
}
