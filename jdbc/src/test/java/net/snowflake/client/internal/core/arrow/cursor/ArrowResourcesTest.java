package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.junit.jupiter.api.Test;

public class ArrowResourcesTest {

  @Test
  public void testCopyVectorSchemaRootDeepCopy() {
    try (RootAllocator allocator = new RootAllocator(Long.MAX_VALUE);
        IntVector vector = new IntVector("col", allocator);
        VectorSchemaRoot root = VectorSchemaRoot.of(vector)) {
      vector.allocateNew(2);
      vector.setSafe(0, 10);
      vector.setSafe(1, 20);
      vector.setValueCount(2);
      root.setRowCount(2);

      ArrowResources resources = new ArrowResources(null, allocator, null);
      VectorSchemaRoot copy = resources.copyVectorSchemaRoot(root);
      try {
        IntVector copyVector = (IntVector) copy.getVector(0);
        assertEquals(2, copy.getRowCount());
        assertEquals(10, copyVector.get(0));
        vector.setSafe(0, 99);
        assertEquals(10, copyVector.get(0));
      } finally {
        copy.close();
      }
    }
  }
}
