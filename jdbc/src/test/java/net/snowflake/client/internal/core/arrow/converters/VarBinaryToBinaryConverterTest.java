package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
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
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.VarBinaryVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class VarBinaryToBinaryConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  private final Random random = new Random();

  @Test
  public void testConvertToString() {
    final int rowCount = 1000;
    List<byte[]> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(TestHelper.randomString(random, 20).getBytes());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BINARY");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARBINARY.getType(), null, customFieldMeta);

    VarBinaryVector vector = new VarBinaryVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new VarBinaryToBinaryConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      String stringVal = converter.toString(i);
      Object objectVal = converter.toObject(i);
      byte[] bytesVal = converter.toBytes(i);
      if (stringVal != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertNull(stringVal);
        assertNull(objectVal);
        assertNull(bytesVal);
        assertFalse(converter.toBoolean(i));
      } else {
        String expectedString = new String(expectedValues.get(i));
        assertEquals(expectedString, stringVal);
        assertArrayEquals(expectedValues.get(i), bytesVal);
        assertArrayEquals(expectedValues.get(i), (byte[]) objectVal);
        int index = i;
        TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(index));
      }
    }
    vector.clear();
  }
}
