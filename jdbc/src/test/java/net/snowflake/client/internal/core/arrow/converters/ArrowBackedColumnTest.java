package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

/**
 * {@link Column} is the live view JDBC integer, timestamp-struct, and interval converters read
 * through. The underlying buffers are reused across batches, so reads must observe in-place updates
 * rather than values captured at construction time.
 */
class ArrowBackedColumnTest {

  @Test
  void shouldReadSignedValuesThroughDataBuffersForEachIntegerWidth() {
    Column tiny = ColumnBuilder.of("tiny", Column.PhysicalKind.INT8, null);
    Column small = ColumnBuilder.of("small", Column.PhysicalKind.INT16, null);
    Column intColumn = ColumnBuilder.of("int", Column.PhysicalKind.INT32, null);
    Column big = ColumnBuilder.of("big", Column.PhysicalKind.INT64, null);
    try {
      ColumnBuilder.set(tiny, 0, (byte) -3);
      ColumnBuilder.set(small, 0, (short) 42);
      ColumnBuilder.set(intColumn, 0, -1_000_000);
      ColumnBuilder.set(big, 0, Long.MIN_VALUE + 7);

      assertEquals(-3L, tiny.getSigned(0));
      assertEquals(42L, small.getSigned(0));
      assertEquals(-1_000_000L, intColumn.getSigned(0));
      assertEquals(Long.MIN_VALUE + 7, big.getSigned(0));
    } finally {
      tiny.close();
      small.close();
      intColumn.close();
      big.close();
    }
  }

  @Test
  void shouldReflectInPlaceReloadOnIntegerVectorWithoutRebuildingColumn() {
    Column column = ColumnBuilder.of("fixed", Column.PhysicalKind.INT32, null);
    try {
      ColumnBuilder.set(column, 0, 10);
      assertEquals(10L, column.getSigned(0));

      ColumnBuilder.set(column, 0, 99);
      assertEquals(
          99L, column.getSigned(0), "column must read the batch currently loaded in the vector");
    } finally {
      column.close();
    }
  }

  @Test
  void shouldReflectNullBitmapUpdatesWithoutRebuildingColumn() {
    Column column = ColumnBuilder.of("fixed", Column.PhysicalKind.INT32, null);
    try {
      ColumnBuilder.set(column, 0, 1);
      assertFalse(column.isNull(0));

      ColumnBuilder.setNull(column, 0);
      assertTrue(column.isNull(0));
    } finally {
      column.close();
    }
  }

  @Test
  void shouldReadReloadedStructChildrenThroughChildView() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP_NTZ");
    Column epochs =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_EPOCH, Column.PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, Column.PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, 1_000L);
    ColumnBuilder.set(fractions, 0, 5);
    Column struct = ColumnBuilder.struct("col", metadata, epochs, fractions);
    try {
      ColumnBuilder.setDefined(struct, 0);
      ColumnBuilder.setValueCount(struct, 1);
      Column epochView = struct.child(AbstractArrowVectorConverter.FIELD_NAME_EPOCH);
      Column fractionView = struct.child(AbstractArrowVectorConverter.FIELD_NAME_FRACTION);

      assertEquals(1_000L, epochView.getLong(0));
      assertEquals(5, fractionView.getInt(0));

      ColumnBuilder.set(epochView, 0, 2_000L);
      ColumnBuilder.set(fractionView, 0, 9);

      assertEquals(2_000L, epochView.getLong(0));
      assertEquals(9, fractionView.getInt(0));
    } finally {
      struct.close();
    }
  }
}
