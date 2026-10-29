package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
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
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class DoubleToRealConverterTest extends BaseConverterTest {
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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.FLOAT64, customFieldMeta);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        ColumnBuilder.setNull(vector, i);
        nullValIndex.add(i);
      } else {
        ColumnBuilder.set(vector, i, expectedValues.get(i));
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
    ColumnBuilder.clear(vector);
  }

  @Test
  public void shouldConvertDoubleToIntegralAndBigDecimalAndReloadInPlace() throws Exception {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "REAL");
    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.FLOAT64, customFieldMeta);
    try {
      ColumnBuilder.set(vector, 0, 42.75d);
      ColumnBuilder.setNull(vector, 1);
      ColumnBuilder.setValueCount(vector, 2);

      DoubleToRealConverter converter = new DoubleToRealConverter(vector, 0, this);

      assertEquals((short) 42, converter.toShort(0));
      assertEquals(42, converter.toInt(0));
      assertEquals(42L, converter.toLong(0));
      assertEquals(BigDecimal.valueOf(42.75d), converter.toBigDecimal(0));

      assertEquals((short) 0, converter.toShort(1));
      assertEquals(0, converter.toInt(1));
      assertEquals(0L, converter.toLong(1));
      assertNull(converter.toBigDecimal(1));

      ColumnBuilder.set(vector, 0, -12.5d);
      assertEquals(-12.5d, converter.toDouble(0));
      assertEquals(-12, converter.toInt(0));
      assertEquals(BigDecimal.valueOf(-12.5d), converter.toBigDecimal(0));
    } finally {
      vector.close();
    }
  }
}
