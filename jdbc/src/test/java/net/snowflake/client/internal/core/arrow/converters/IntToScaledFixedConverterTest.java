package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

public class IntToScaledFixedConverterTest extends BaseConverterTest {
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  @Test
  public void shouldConvertScaledFixedValue() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "2");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);
    IntVector vector = new IntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 12_345);
    vector.setNull(1);
    vector.setValueCount(2);

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
