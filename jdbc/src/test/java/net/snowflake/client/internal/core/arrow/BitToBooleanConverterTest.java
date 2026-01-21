package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BitVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class BitToBooleanConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  private final Random random = new Random();

  @Test
  public void testConvertToString() {
    final int rowCount = 1000;
    List<Boolean> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(random.nextBoolean());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BOOLEAN");

    FieldType fieldType = new FieldType(true, Types.MinorType.BIT.getType(), null, customFieldMeta);

    BitVector vector = new BitVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i) ? 1 : 0);
      }
    }

    ArrowVectorConverter converter = new BitToBooleanConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      boolean boolVal = converter.toBoolean(i);
      Object objectVal = converter.toObject(i);
      String stringVal = converter.toString(i);

      if (stringVal != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertFalse(boolVal);
        assertNull(objectVal);
        assertNull(stringVal);
        assertNull(converter.toBytes(i));
      } else {
        assertEquals(expectedValues.get(i), boolVal);
        assertEquals(expectedValues.get(i), objectVal);
        assertEquals(Boolean.toString(expectedValues.get(i)).toUpperCase(), stringVal);
        if (boolVal) {
          assertEquals((byte) 0x1, converter.toBytes(i)[0]);
        } else {
          assertEquals((byte) 0x0, converter.toBytes(i)[0]);
        }
      }
    }
    vector.clear();
  }
}
