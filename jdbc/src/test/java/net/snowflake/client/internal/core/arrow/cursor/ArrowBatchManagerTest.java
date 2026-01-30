package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

public class ArrowBatchManagerTest {

  @Test
  public void testFetchNextRowSkipsEmptyBatches() throws Exception {
    try (ArrowCursorTestUtils.TestResources resourcesHolder =
        ArrowCursorTestUtils.createIntResources(
            new int[] {}, new int[] {10, 20}, new int[] {}, new int[] {30})) {
      ArrowResources resources = resourcesHolder.getResources();
      CursorState cursor = new CursorState();
      SchemaState schema = new SchemaState(resources.getActiveRoot());
      ArrowBatchManager manager = new ArrowBatchManager(cursor, resources, schema);

      assertTrue(manager.fetchNextRow());
      assertEquals(0, cursor.getCurrentRowInBatch());
      assertEquals(2, cursor.getCurrentBatchRowCount());
      assertFalse(cursor.isAfterLast());

      assertTrue(manager.fetchNextRow());
      assertEquals(1, cursor.getCurrentRowInBatch());

      assertTrue(manager.fetchNextRow());
      assertEquals(0, cursor.getCurrentRowInBatch());
      assertEquals(1, cursor.getCurrentBatchRowCount());

      assertFalse(manager.fetchNextRow());
      assertTrue(cursor.isAfterLast());
    }
  }
}
