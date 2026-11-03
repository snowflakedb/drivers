package net.snowflake.client.internal.core.arrow.vector;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import org.junit.jupiter.api.Test;

class FixedSizeListVectorTest {

  @Test
  void shouldFormatIntVectorWithoutSpaces() {
    Column vector = intVector(1, 2, 3);
    List<?> values = (List<?>) vector.getObject(0);

    assertEquals(Arrays.asList(1, 2, 3), values);
    assertEquals(1, values.get(0));
    assertEquals("[1,2,3]", values.toString());
    assertEquals(
        "[-2147483648,2147483647,0]",
        intVector(Integer.MIN_VALUE, Integer.MAX_VALUE, 0).getObject(0).toString());
  }

  @Test
  void shouldFormatFloatVectorWithoutSpaces() {
    Column vector = floatVector(1.8f, -3.4f, 6.7f, 0.0f, 2.3f);
    List<?> values = (List<?>) vector.getObject(0);

    assertEquals(Arrays.asList(1.8f, -3.4f, 6.7f, 0.0f, 2.3f), values);
    assertEquals("[1.8,-3.4,6.7,0.0,2.3]", values.toString());
  }

  @Test
  void shouldFormatEmptyVectorAsEmptyList() {
    Column vector = intVector();

    assertEquals(Collections.emptyList(), vector.getObject(0));
    assertEquals("[]", vector.getObject(0).toString());
  }

  @Test
  void shouldReturnNullForNullVector() {
    Column vector = intVector(1, 2, 3);
    ColumnBuilder.setNull(vector, 0);

    assertNull(vector.getObject(0));
  }

  @Test
  void shouldSliceMultiRowVectorsAndPreserveNullElementsInCompactList() {
    Column child = ColumnBuilder.of("item", Column.PhysicalKind.INT32, null);
    ColumnBuilder.set(child, 0, 10);
    ColumnBuilder.setNull(child, 1);
    ColumnBuilder.set(child, 2, 30);
    ColumnBuilder.set(child, 3, 40);
    ColumnBuilder.set(child, 4, 50);
    ColumnBuilder.set(child, 5, 60);

    try (Column vector = ColumnBuilder.list("v", child, 3)) {
      ColumnBuilder.setDefined(vector, 0);
      ColumnBuilder.setDefined(vector, 1);
      ColumnBuilder.setNull(vector, 2);
      ColumnBuilder.setValueCount(vector, 3);

      List<?> row0 = (List<?>) vector.getObject(0);
      assertEquals(Arrays.asList(10, null, 30), row0);
      assertEquals("[10,null,30]", row0.toString());

      List<?> row1 = (List<?>) vector.getObject(1);
      assertEquals(Arrays.asList(40, 50, 60), row1);
      assertEquals("[40,50,60]", row1.toString());

      assertNull(vector.getObject(2));
    }
  }

  @Test
  void shouldRejectUnsupportedElementKindInsideFixedSizeList() {
    Column innerItem = ColumnBuilder.of("leaf", Column.PhysicalKind.INT32, null);
    Column nestedList = ColumnBuilder.list("inner", innerItem, 1);
    ColumnBuilder.setDefined(nestedList, 0);

    try (Column outerList = ColumnBuilder.list("outer", nestedList, 1)) {
      ColumnBuilder.setDefined(outerList, 0);
      ColumnBuilder.setValueCount(outerList, 1);

      IllegalArgumentException failure =
          assertThrows(IllegalArgumentException.class, () -> outerList.getObject(0));
      assertEquals("Unsupported struct child FIXED_SIZE_LIST", failure.getMessage());
    }
  }

  private static Column intVector(int... values) {
    Column child = ColumnBuilder.of("item", Column.PhysicalKind.INT32, null);
    for (int index = 0; index < values.length; index++) {
      ColumnBuilder.set(child, index, values[index]);
    }
    return list(child, values.length);
  }

  private static Column floatVector(float... values) {
    Column child = ColumnBuilder.of("item", Column.PhysicalKind.FLOAT32, null);
    for (int index = 0; index < values.length; index++) {
      ColumnBuilder.set(child, index, values[index]);
    }
    return list(child, values.length);
  }

  private static Column list(Column child, int listSize) {
    Column vector = ColumnBuilder.list("v", child, listSize);
    ColumnBuilder.setDefined(vector, 0);
    return vector;
  }
}
