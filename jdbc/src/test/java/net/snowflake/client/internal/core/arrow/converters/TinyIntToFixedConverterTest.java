package net.snowflake.client.internal.core.arrow.converters;

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
import net.snowflake.client.internal.core.arrow.ArrowResultUtil;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class TinyIntToFixedConverterTest extends BaseConverterTest {
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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        ColumnBuilder.setNull(vector, i);
        nullValIndex.add(i);
      } else {
        ColumnBuilder.set(vector, i, expectedValues.get(i));
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
    ColumnBuilder.clear(vector);
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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        ColumnBuilder.setNull(vector, i);
        nullValIndex.add(i);
      } else {
        ColumnBuilder.set(vector, i, expectedValues.get(i));
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

    ColumnBuilder.clear(vector);
  }

  @Test
  public void testInvalidConversion() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "1");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    ColumnBuilder.set(vector, 0, 200);

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
    ColumnBuilder.clear(vector);
  }

  @Test
  public void testNullAfterNonNullDoesNotReturnStaleValue() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    ColumnBuilder.set(vector, 0, 42);
    ColumnBuilder.setNull(vector, 1);

    ArrowVectorConverter converter = new TinyIntToFixedConverter(vector, 0, this);

    assertEquals(42L, converter.toLong(0));
    assertEquals(0L, converter.toLong(1));
    assertNull(converter.toObject(1));

    ColumnBuilder.clear(vector);
  }

  @Test
  public void testGetSmallerIntegralType() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    Column vectorBar = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    ColumnBuilder.set(vectorBar, 0, 10);
    ColumnBuilder.set(vectorBar, 1, -10);

    final ArrowVectorConverter converterBar = new TinyIntToFixedConverter(vectorBar, 0, this);

    assertEquals((short) 10, converterBar.toShort(0));
    assertEquals((short) -10, converterBar.toShort(1));
    assertEquals(10, converterBar.toInt(0));
    assertEquals(-10, converterBar.toInt(1));
    assertEquals(10L, converterBar.toLong(0));
    assertEquals(-10L, converterBar.toLong(1));
    ColumnBuilder.clear(vectorBar);
  }

  @Test
  public void testGetBooleanNoScale() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");
    customFieldMeta.put("precision", "10");
    customFieldMeta.put("scale", "0");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    ColumnBuilder.set(vector, 0, 0);
    ColumnBuilder.set(vector, 1, 1);
    ColumnBuilder.setNull(vector, 2);
    ColumnBuilder.set(vector, 3, 5);

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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.INT8, customFieldMeta);
    ColumnBuilder.set(vector, 0, 0);
    ColumnBuilder.set(vector, 1, 1);
    ColumnBuilder.setNull(vector, 2);
    ColumnBuilder.set(vector, 3, 5);

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
