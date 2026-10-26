package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.TimeZone;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

public class VarCharConverterTest extends BaseConverterTest {
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

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.UTF8, customFieldMeta);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        ColumnBuilder.setNull(vector, i);
        nullValIndex.add(i);
      } else {
        ColumnBuilder.set(vector, i, expectedValues.get(i).getBytes(StandardCharsets.UTF_8));
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
    ColumnBuilder.clear(vector);
  }

  @Test
  public void testGetBoolean() {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.UTF8, customFieldMeta);
    ColumnBuilder.set(vector, 0, "0".getBytes(StandardCharsets.UTF_8));
    ColumnBuilder.set(vector, 1, "1".getBytes(StandardCharsets.UTF_8));
    ColumnBuilder.setNull(vector, 2);
    ColumnBuilder.set(vector, 3, "5".getBytes(StandardCharsets.UTF_8));

    ArrowVectorConverter converter = new VarCharConverter(vector, 0, this);

    assertFalse(converter.toBoolean(0));
    assertTrue(converter.toBoolean(1));
    assertFalse(converter.toBoolean(2));
    TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(3));

    vector.close();
  }

  @Test
  public void testGetDate() throws SFSQLException {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "FIXED");

    Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.UTF8, customFieldMeta);
    ColumnBuilder.setNull(vector, 0);
    ColumnBuilder.set(vector, 1, "2023-10-26".getBytes(StandardCharsets.UTF_8));
    ColumnBuilder.set(vector, 2, "abc".getBytes(StandardCharsets.UTF_8));

    ArrowVectorConverter converter = new VarCharConverter(vector, 0, this);

    assertNull(converter.toDate(0, TimeZone.getDefault(), false));
    assertEquals(Date.valueOf("2023-10-26"), converter.toDate(1, TimeZone.getDefault(), false));
    TestHelper.assertSFException(
        invalidConversionErrorCode, () -> converter.toDate(2, TimeZone.getDefault(), false));

    vector.close();
  }

  @ParameterizedTest
  @ValueSource(strings = {"MAP", "ARRAY", "OBJECT"})
  public void shouldDispatchStructuredTypeToVarCharConverterViaUtil(String logicalType)
      throws Exception {
    String json = "{\"k\":1}";
    try (Column vector = structuredVector(logicalType, json)) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertInstanceOf(VarCharConverter.class, converter);
      // The fallback surfaces the server's raw JSON text unchanged.
      assertEquals(json, converter.toString(0));
      assertEquals(json, converter.toObject(0));
    }
  }

  @ParameterizedTest
  @ValueSource(strings = {"MAP", "ARRAY", "OBJECT"})
  public void shouldReturnNullForNullStructuredValue(String logicalType) throws Exception {
    try (Column vector = structuredVector(logicalType, "{\"k\":1}")) {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);

      assertNull(converter.toString(1));
      assertNull(converter.toObject(1));
    }
  }

  private Column structuredVector(String logicalType, String json) {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", logicalType);
    Column vector = ColumnBuilder.of("col_structured", Column.PhysicalKind.UTF8, customFieldMeta);
    ColumnBuilder.set(vector, 0, json.getBytes(StandardCharsets.UTF_8));
    ColumnBuilder.setNull(vector, 1);
    ColumnBuilder.setValueCount(vector, 2);
    return vector;
  }
}
