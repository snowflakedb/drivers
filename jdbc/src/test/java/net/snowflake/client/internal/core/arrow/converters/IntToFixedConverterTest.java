package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.TimeZone;
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class IntToFixedConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  /** Random seed */
  private final Random random = new Random();

  @Test
  public void testFixedNoScale() {
    final int rowCount = 1000;
    List<Integer> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(random.nextInt());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    IntVector vector = new IntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new IntToFixedConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      int intVal = converter.toInt(i);
      Object longObj = converter.toObject(i);
      String intString = converter.toString(i);
      if (intString != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertEquals(0, intVal);
        assertNull(longObj);
        assertNull(intString);
        assertNull(converter.toBytes(i));
      } else {
        assertEquals(expectedValues.get(i), intVal);
        assertEquals((long) expectedValues.get(i), longObj);
        assertEquals(expectedValues.get(i).toString(), intString);
        ByteBuffer bb = ByteBuffer.wrap(converter.toBytes(i));
        assertEquals(intVal, bb.getInt());
      }
    }
    vector.clear();
  }

  @Test
  public void testFixedWithScale() {
    final int rowCount = 1000;
    List<Integer> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(random.nextInt());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "3");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    IntVector vector = new IntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new IntToScaledFixedConverter(vector, 0, this, 3);
    String format = ArrowResultUtil.getStringFormat(3);

    for (int i = 0; i < rowCount; i++) {
      BigDecimal bigDecimalVal = converter.toBigDecimal(i);
      Object objectVal = converter.toObject(i);
      String stringVal = converter.toString(i);

      if (nullValIndex.contains(i)) {
        assertNull(bigDecimalVal);
        assertNull(objectVal);
        assertNull(stringVal);
        assertNull(converter.toBytes(i));
      } else {
        BigDecimal expectedVal = BigDecimal.valueOf(expectedValues.get(i), 3);
        assertEquals(expectedVal, bigDecimalVal);
        assertEquals(expectedVal, objectVal);
        String expectedString =
            String.format(format, (double) expectedValues.get(i) / ArrowResultUtil.powerOfTen(3));
        assertEquals(expectedString, stringVal);
        assertNotNull(converter.toBytes(i));
      }
    }

    vector.clear();
  }

  @Test
  public void testInvalidConversion() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "3");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    IntVector vector = new IntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 33000);

    final ArrowVectorConverter converter = new IntToScaledFixedConverter(vector, 0, this, 3);

    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toByte(0));
    TestHelper.assertSFException(
        invalidConversionErrorCode, () -> converter.toDate(0, TimeZone.getDefault(), false));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toTime(0));
    TestHelper.assertSFException(
        invalidConversionErrorCode, () -> converter.toTimestamp(0, TimeZone.getDefault()));
    vector.clear();
  }

  @Test
  public void testGetSmallerIntegralType() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    // test value which is out of range of short/byte
    IntVector vectorFoo = new IntVector("col_one", fieldType, allocator);
    vectorFoo.setSafe(0, 33000);
    vectorFoo.setSafe(1, -33000);

    final ArrowVectorConverter converterFoo = new IntToFixedConverter(vectorFoo, 0, this);

    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toShort(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toByte(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toShort(1));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toByte(1));

    assertEquals(33000L, converterFoo.toLong(0));
    assertEquals(-33000L, converterFoo.toLong(1));
    vectorFoo.clear();

    // test value which is in range of byte
    IntVector vectorBar = new IntVector("col_one", fieldType, allocator);
    vectorBar.setSafe(0, 10);
    vectorBar.setSafe(1, -10);

    final ArrowVectorConverter converterBar = new IntToFixedConverter(vectorBar, 0, this);

    assertEquals((byte) 10, converterBar.toByte(0));
    assertEquals((byte) -10, converterBar.toByte(1));
    assertEquals((short) 10, converterBar.toShort(0));
    assertEquals((short) -10, converterBar.toShort(1));
    assertEquals(10L, converterBar.toLong(0));
    assertEquals(-10L, converterBar.toLong(1));
    vectorBar.clear();
  }

  @Test
  public void testGetBooleanNoScale() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    IntVector vector = new IntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    ArrowVectorConverter converter = new IntToFixedConverter(vector, 0, this);

    assertFalse(converter.toBoolean(0));
    assertTrue(converter.toBoolean(1));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }

  @Test
  public void testGetBooleanWithScale() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "3");

    FieldType fieldType = new FieldType(true, Types.MinorType.INT.getType(), null, customFieldMeta);

    IntVector vector = new IntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    final ArrowVectorConverter converter = new IntToScaledFixedConverter(vector, 0, this, 3);

    assertFalse(converter.toBoolean(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }
}
