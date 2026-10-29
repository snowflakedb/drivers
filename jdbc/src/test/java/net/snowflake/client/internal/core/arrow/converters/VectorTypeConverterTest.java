package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

class VectorTypeConverterTest extends BaseConverterTest {
  private static final int LIST_SIZE = 3;

  @Test
  void shouldExposeVectorAsStringFromToObjectForResultSetParity() {
    try (Column vector = newVector(1, 2, 3)) {
      VectorTypeConverter converter = new VectorTypeConverter(vector, 0, this);

      assertEquals("[1,2,3]", converter.toString(0));
      Object object = converter.toObject(0);
      assertInstanceOf(String.class, object);
      assertEquals("[1,2,3]", object);
      assertEquals("[1,2,3]", new String(converter.toBytes(0), StandardCharsets.UTF_8));
    }
  }

  @Test
  void shouldReturnUnderlyingListFromToList() {
    try (Column vector = newVector(1, 2, 3)) {
      VectorTypeConverter converter = new VectorTypeConverter(vector, 0, this);

      List<?> list = converter.toList(0);
      assertEquals(Arrays.asList(1, 2, 3), list);
    }
  }

  @Test
  void shouldReturnNullForNullVectorRow() {
    try (Column vector = newVector(1, 2, 3)) {
      ColumnBuilder.setNull(vector, 1);
      ColumnBuilder.setValueCount(vector, 2);
      VectorTypeConverter converter = new VectorTypeConverter(vector, 0, this);

      assertNull(converter.toString(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toList(1));
      assertNull(converter.toBytes(1));
      assertTrue(converter.isNull(1));
    }
  }

  @Test
  void shouldReadReloadedListElementsWithoutRebuildingConverter() {
    try (Column vector = newVector(1, 2, 3)) {
      VectorTypeConverter converter = new VectorTypeConverter(vector, 0, this);
      assertEquals("[1,2,3]", converter.toString(0));

      Column child = vector.child("item");
      ColumnBuilder.set(child, 0, 10);
      ColumnBuilder.set(child, 1, 20);
      ColumnBuilder.set(child, 2, 30);
      assertEquals("[10,20,30]", converter.toString(0));
      assertEquals(Arrays.asList(10, 20, 30), converter.toList(0));
    }
  }

  private static Column newVector(int... values) {
    if (values.length != LIST_SIZE) {
      throw new IllegalArgumentException("expected " + LIST_SIZE + " elements");
    }
    Column child = ColumnBuilder.of("item", Column.PhysicalKind.INT32, null);
    for (int i = 0; i < values.length; i++) {
      ColumnBuilder.set(child, i, values[i]);
    }
    Column vector = ColumnBuilder.list("vec", child, LIST_SIZE);
    ColumnBuilder.setDefined(vector, 0);
    ColumnBuilder.setValueCount(vector, 1);
    return vector;
  }
}
