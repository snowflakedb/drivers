package net.snowflake.client.internal.core.arrow;

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
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.SmallIntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class SmallIntToFixedConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  /** Random seed */
  private final Random random = new Random();

  @Test
  public void testFixedNoScale() {
    final int rowCount = 1000;
    List<Short> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add((short) random.nextInt(1 << 16));
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    SmallIntVector vector = new SmallIntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new SmallIntToFixedConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      short shortVal = converter.toShort(i);
      Object longObject = converter.toObject(i); // the logical type is long
      String shortString = converter.toString(i);
      if (shortString != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertEquals((short) 0, shortVal);
        assertNull(longObject);
        assertNull(shortString);
        assertNull(converter.toBytes(i));
      } else {
        assertEquals(expectedValues.get(i), shortVal);
        assertEquals((long) expectedValues.get(i), longObject);
        assertEquals(expectedValues.get(i).toString(), shortString);
        ByteBuffer bb = ByteBuffer.wrap(converter.toBytes(i));
        assertEquals(shortVal, bb.getShort());
      }
    }
    vector.clear();
  }

  @Test
  public void testFixedWithScale() {
    final int rowCount = 1000;
    List<Short> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add((short) random.nextInt(1 << 16));
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "3");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    SmallIntVector vector = new SmallIntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new SmallIntToScaledFixedConverter(vector, 0, this, 3);
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
            String.format(format, (float) expectedValues.get(i) / ArrowResultUtil.powerOfTen(3));
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

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    SmallIntVector vector = new SmallIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 200);

    final ArrowVectorConverter converter = new SmallIntToScaledFixedConverter(vector, 0, this, 3);

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

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    // test value which is out of range of byte
    SmallIntVector vectorFoo = new SmallIntVector("col_one", fieldType, allocator);
    vectorFoo.setSafe(0, 200);
    vectorFoo.setSafe(1, -200);

    final ArrowVectorConverter converterFoo = new SmallIntToFixedConverter(vectorFoo, 0, this);

    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toByte(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converterFoo.toByte(1));
    vectorFoo.clear();

    // test value which is in range of byte, all get method should return
    SmallIntVector vectorBar = new SmallIntVector("col_one", fieldType, allocator);
    vectorBar.setSafe(0, 10);
    vectorBar.setSafe(1, -10);

    final ArrowVectorConverter converterBar = new SmallIntToFixedConverter(vectorBar, 0, this);

    assertEquals((byte) 10, converterBar.toByte(0));
    assertEquals((byte) -10, converterBar.toByte(1));
    assertEquals(10, converterBar.toInt(0));
    assertEquals(-10, converterBar.toInt(1));
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

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    SmallIntVector vector = new SmallIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    ArrowVectorConverter converter = new SmallIntToFixedConverter(vector, 0, this);

    assertFalse(converter.toBoolean(0));
    assertTrue(converter.toBoolean(1));
    assertFalse(converter.toBoolean(2));
    assertFalse(converter.isNull(0));
    assertFalse(converter.isNull(1));
    assertTrue(converter.isNull(2));
    assertFalse(converter.isNull(3));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }

  @Test
  public void testGetBooleanWithScale() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "3");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.SMALLINT.getType(), null, customFieldMeta);

    SmallIntVector vector = new SmallIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    final ArrowVectorConverter converter = new SmallIntToScaledFixedConverter(vector, 0, this, 3);

    assertFalse(converter.toBoolean(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }
}
