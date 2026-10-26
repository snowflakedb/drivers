package net.snowflake.client.internal.core.arrow.cursor;

import java.util.ArrayList;
import java.util.List;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;

final class ArrowCursorTestUtils {
  private ArrowCursorTestUtils() {}

  static TestResources createIntResources(int[]... batches) {
    List<ColumnBatch> roots = new ArrayList<>();
    for (int[] batch : batches) {
      Column vector = ColumnBuilder.of("col", Column.PhysicalKind.INT32, null);
      for (int rowIndex = 0; rowIndex < batch.length; rowIndex++) {
        ColumnBuilder.set(vector, rowIndex, batch[rowIndex]);
      }
      ColumnBuilder.setValueCount(vector, batch.length);
      ColumnBatch root = ColumnBatch.of(vector);
      root.setRowCount(batch.length);
      roots.add(root);
    }
    return new TestResources(new ArrowResources(new ColumnBatchListReader(roots)));
  }

  static final class TestResources implements AutoCloseable {
    private final ArrowResources resources;

    private TestResources(ArrowResources resources) {
      this.resources = resources;
    }

    ArrowResources getResources() {
      return resources;
    }

    @Override
    public void close() {
      resources.closeAll();
    }
  }
}
