package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.Float8Vector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class DoubleToRealConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  /** Random seed */
  private final Random random = new Random();

  @Test
  public void testConvertToDouble() {
    final int rowCount = 1000;
    List<Double> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(random.nextDouble());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "REAL");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.FLOAT8.getType(), null, customFieldMeta);

    Float8Vector vector = new Float8Vector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new DoubleToRealConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      double doubleVal = converter.toDouble(i);
      float floatVal = converter.toFloat(i);
      Object doubleObject = converter.toObject(i);
      String doubleString = converter.toString(i);
      if (doubleObject != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertEquals(0.0d, doubleVal);
        assertEquals(0.0f, floatVal);
        assertNull(doubleObject);
        assertNull(doubleString);
        assertFalse(converter.toBoolean(i));
        assertNull(converter.toBytes(i));
      } else {
        assertEquals(expectedValues.get(i), doubleVal);
        assertEquals(expectedValues.get(i).floatValue(), floatVal);
        assertEquals(expectedValues.get(i), doubleObject);
        assertEquals(expectedValues.get(i).toString(), doubleString);
        int index = i;
        TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(index));
        ByteBuffer bb = ByteBuffer.wrap(converter.toBytes(i));
        assertEquals(doubleVal, bb.getDouble());
      }
    }
    vector.clear();
  }
}
