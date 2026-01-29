package net.snowflake.client.internal.core.arrow.cursor;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.ipc.ArrowStreamReader;
import org.apache.arrow.vector.ipc.ArrowStreamWriter;

final class ArrowCursorTestUtils {
  private ArrowCursorTestUtils() {}

  static TestResources createIntResources(int[]... batches) throws IOException {
    RootAllocator allocator = new RootAllocator(Long.MAX_VALUE);
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    try (IntVector vector = new IntVector("col", allocator);
        VectorSchemaRoot root = VectorSchemaRoot.of(vector);
        ArrowStreamWriter writer = new ArrowStreamWriter(root, null, output)) {
      writer.start();
      for (int[] batch : batches) {
        vector.allocateNew(batch.length);
        for (int rowIndex = 0; rowIndex < batch.length; rowIndex++) {
          vector.setSafe(rowIndex, batch[rowIndex]);
        }
        vector.setValueCount(batch.length);
        root.setRowCount(batch.length);
        writer.writeBatch();
        vector.clear();
      }
      writer.end();
    }
    ArrowStreamReader reader =
        new ArrowStreamReader(new ByteArrayInputStream(output.toByteArray()), allocator);
    return new TestResources(new ArrowResources(null, allocator, reader));
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
    public void close() throws Exception {
      resources.closeReader();
      resources.closeStream();
      resources.closeAllocator();
    }
  }
}
