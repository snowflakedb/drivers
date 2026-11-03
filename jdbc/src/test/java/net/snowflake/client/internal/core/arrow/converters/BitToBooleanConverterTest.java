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
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class BitToBooleanConverterTest extends BaseConverterTest {
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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.BOOL, customFieldMeta);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        ColumnBuilder.setNull(vector, i);
        nullValIndex.add(i);
      } else {
        ColumnBuilder.set(vector, i, expectedValues.get(i) ? 1 : 0);
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
    ColumnBuilder.clear(vector);
  }

  @Test
  public void shouldReadReloadedBitsAcrossByteBoundariesWithoutRebuildingConverter() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BOOLEAN");
    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.BOOL, customFieldMeta);
    try {
      ColumnBuilder.set(vector, 0, 1);
      ColumnBuilder.set(vector, 7, 0);
      ColumnBuilder.set(vector, 8, 0);
      ColumnBuilder.set(vector, 9, 1);
      ColumnBuilder.setValueCount(vector, 10);

      BitToBooleanConverter converter = new BitToBooleanConverter(vector, 0, this);
      assertTrue(converter.toBoolean(0));
      assertFalse(converter.toBoolean(7));
      assertFalse(converter.toBoolean(8));
      assertTrue(converter.toBoolean(9));

      ColumnBuilder.set(vector, 0, 0);
      ColumnBuilder.set(vector, 7, 1);
      ColumnBuilder.set(vector, 8, 1);
      ColumnBuilder.set(vector, 9, 0);

      assertFalse(converter.toBoolean(0));
      assertTrue(converter.toBoolean(7));
      assertTrue(converter.toBoolean(8));
      assertFalse(converter.toBoolean(9));
      assertEquals("TRUE", converter.toString(8));
      assertEquals("FALSE", converter.toString(9));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertBitToNumericTypesAndRejectUnsupportedConversions() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BOOLEAN");
    try (Column vector = ColumnBuilder.of("col_bool", Column.PhysicalKind.BOOL, customFieldMeta)) {
      ColumnBuilder.set(vector, 0, 1);
      ColumnBuilder.set(vector, 1, 0);
      ColumnBuilder.setNull(vector, 2);
      ColumnBuilder.setValueCount(vector, 3);

      BitToBooleanConverter converter = new BitToBooleanConverter(vector, 0, this);

      assertEquals((short) 1, converter.toShort(0));
      assertEquals(1, converter.toInt(0));
      assertEquals(1L, converter.toLong(0));
      assertEquals(1.0f, converter.toFloat(0));
      assertEquals(1.0d, converter.toDouble(0));
      assertEquals(BigDecimal.ONE, converter.toBigDecimal(0));

      assertEquals((short) 0, converter.toShort(1));
      assertEquals(0, converter.toInt(1));
      assertEquals(0L, converter.toLong(1));
      assertEquals(0.0f, converter.toFloat(1));
      assertEquals(0.0d, converter.toDouble(1));
      assertEquals(BigDecimal.ZERO, converter.toBigDecimal(1));

      assertEquals((short) 0, converter.toShort(2));
      assertEquals(0, converter.toInt(2));
      assertEquals(0L, converter.toLong(2));
      assertEquals(0.0f, converter.toFloat(2));
      assertEquals(0.0d, converter.toDouble(2));
      assertNull(converter.toBigDecimal(2));
      assertNull(converter.toPeriod(2));
      assertNull(converter.toDuration(2));
      assertNull(converter.toList(2));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toByte(0));
      TestHelper.assertSFException(
          invalidConversionErrorCode, () -> converter.toDate(0, TimeZone.getDefault(), false));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toTime(0));
      TestHelper.assertSFException(
          invalidConversionErrorCode, () -> converter.toTimestamp(0, TimeZone.getDefault()));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toPeriod(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toDuration(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toList(0));
    }
  }
}
