package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.TimeZone;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.VarCharVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class VarCharConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  private final Random random = new Random();

  @Test
  public void testConvertToString() {
    final int rowCount = 1000;
    List<String> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(TestHelper.randomString(random, 20));
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARCHAR.getType(), null, customFieldMeta);

    VarCharVector vector = new VarCharVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i).getBytes(StandardCharsets.UTF_8));
      }
    }

    ArrowVectorConverter converter = new VarCharConverter(vector, 0, this);

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
      } else {
        assertEquals(expectedValues.get(i), stringVal);
        assertEquals(expectedValues.get(i), objectVal);
        assertArrayEquals(expectedValues.get(i).getBytes(StandardCharsets.UTF_8), bytesVal);
      }
    }
    vector.clear();
  }

  @Test
  public void testGetBoolean() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARCHAR.getType(), null, customFieldMeta);

    VarCharVector vector = new VarCharVector("col_one", fieldType, allocator);
    vector.setSafe(0, "0".getBytes(StandardCharsets.UTF_8));
    vector.setSafe(1, "1".getBytes(StandardCharsets.UTF_8));
    vector.setNull(2);
    vector.setSafe(3, "5".getBytes(StandardCharsets.UTF_8));

    ArrowVectorConverter converter = new VarCharConverter(vector, 0, this);

    assertFalse(converter.toBoolean(0));
    assertTrue(converter.toBoolean(1));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }

  @Test
  public void testGetDate() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARCHAR.getType(), null, customFieldMeta);

    VarCharVector vector = new VarCharVector("col_one", fieldType, allocator);
    vector.setNull(0);
    vector.setSafe(1, "2023-10-26".getBytes(StandardCharsets.UTF_8));
    vector.setSafe(2, "abc".getBytes(StandardCharsets.UTF_8));

    ArrowVectorConverter converter = new VarCharConverter(vector, 0, this);

    TestHelper.assertSFException(
        invalidConversionErrorCode, () -> converter.toDate(1, TimeZone.getDefault(), false));
    TestHelper.assertSFException(
        invalidConversionErrorCode, () -> converter.toDate(2, TimeZone.getDefault(), false));

    vector.close();
  }
}
