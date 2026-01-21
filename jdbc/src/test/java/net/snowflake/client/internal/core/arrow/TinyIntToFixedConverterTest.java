package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
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
import org.apache.arrow.vector.TinyIntVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class TinyIntToFixedConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  /** Random seed */
  private final Random random = new Random();

  @Test
  public void testFixedNoScale() {
    final int rowCount = 1000;
    List<Byte> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add((byte) random.nextInt(1 << 8));
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    TinyIntVector vector = new TinyIntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new TinyIntToFixedConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      byte byteVal = converter.toByte(i);
      Object longObject = converter.toObject(i); // the logical type is long
      String byteString = converter.toString(i);

      if (nullValIndex.contains(i)) {
        assertEquals((byte) 0, byteVal);
        assertNull(longObject);
        assertNull(byteString);
      } else {
        assertEquals(expectedValues.get(i), byteVal);
        assertEquals((long) expectedValues.get(i), longObject);
        assertEquals(expectedValues.get(i).toString(), byteString);
      }
    }
    vector.clear();
  }

  @Test
  public void testFixedWithScale() {
    final int rowCount = 1000;
    List<Byte> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add((byte) random.nextInt(1 << 8));
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "1");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    TinyIntVector vector = new TinyIntVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new TinyIntToScaledFixedConverter(vector, 0, this, 1);
    String format = ArrowResultUtil.getStringFormat(1);

    for (int i = 0; i < rowCount; i++) {
      BigDecimal bigDecimalVal = converter.toBigDecimal(i);
      Object objectVal = converter.toObject(i);
      String stringVal = converter.toString(i);

      if (nullValIndex.contains(i)) {
        assertNull(bigDecimalVal);
        assertNull(objectVal);
        assertNull(stringVal);
      } else {
        BigDecimal expectedVal = BigDecimal.valueOf(expectedValues.get(i), 1);
        assertEquals(expectedVal, bigDecimalVal);
        assertEquals(expectedVal, objectVal);
        String expectedString =
            String.format(format, (float) expectedValues.get(i) / ArrowResultUtil.powerOfTen(1));
        assertEquals(expectedString, stringVal);
      }
    }

    vector.clear();
  }

  @Test
  public void testInvalidConversion() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "1");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    TinyIntVector vector = new TinyIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 200);

    final ArrowVectorConverter converter = new TinyIntToScaledFixedConverter(vector, 0, this, 1);

    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(0));
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
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    // test value which is in range of byte, all get method should return
    TinyIntVector vectorBar = new TinyIntVector("col_one", fieldType, allocator);
    vectorBar.setSafe(0, 10);
    vectorBar.setSafe(1, -10);

    final ArrowVectorConverter converterBar = new TinyIntToFixedConverter(vectorBar, 0, this);

    assertEquals((short) 10, converterBar.toShort(0));
    assertEquals((short) -10, converterBar.toShort(1));
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
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    TinyIntVector vector = new TinyIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    ArrowVectorConverter converter = new TinyIntToFixedConverter(vector, 0, this);

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

    FieldType fieldType =
        new FieldType(true, Types.MinorType.TINYINT.getType(), null, customFieldMeta);

    TinyIntVector vector = new TinyIntVector("col_one", fieldType, allocator);
    vector.setSafe(0, 0);
    vector.setSafe(1, 1);
    vector.setNull(2);
    vector.setSafe(3, 5);

    final ArrowVectorConverter converter = new TinyIntToScaledFixedConverter(vector, 0, this, 3);

    assertFalse(converter.toBoolean(0));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    assertFalse(converter.isNull(0));
    assertFalse(converter.isNull(1));
    assertTrue(converter.isNull(2));
    assertFalse(converter.isNull(3));
    vector.close();
  }
}
