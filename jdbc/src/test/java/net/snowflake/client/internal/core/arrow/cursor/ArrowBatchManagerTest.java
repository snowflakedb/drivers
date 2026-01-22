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
      BatchState batch = new BatchState();
      SchemaState schema = new SchemaState();
      ArrowBatchManager manager = new ArrowBatchManager(cursor, batch, resources, schema);

      assertTrue(manager.fetchNextRow());
      assertEquals(0, batch.getCurrentRowInBatch());
      assertEquals(2, batch.getCurrentBatchRowCount());
      assertFalse(cursor.isAfterLast());

      assertTrue(manager.fetchNextRow());
      assertEquals(1, batch.getCurrentRowInBatch());

      assertTrue(manager.fetchNextRow());
      assertEquals(0, batch.getCurrentRowInBatch());
      assertEquals(1, batch.getCurrentBatchRowCount());

      assertFalse(manager.fetchNextRow());
      assertTrue(cursor.isAfterLast());
    }
  }

  @Test
  public void testPrefetchNextBatchForIsLastWithNextBatch() throws Exception {
    try (ArrowCursorTestUtils.TestResources resourcesHolder =
        ArrowCursorTestUtils.createIntResources(new int[] {1}, new int[] {2})) {
      ArrowResources resources = resourcesHolder.getResources();
      CursorState cursor = new CursorState();
      BatchState batch = new BatchState();
      SchemaState schema = new SchemaState();
      ArrowBatchManager manager = new ArrowBatchManager(cursor, batch, resources, schema);

      assertTrue(manager.fetchNextRow());
      assertFalse(batch.hasPrefetchedBatch());

      manager.prefetchNextBatchForIsLast();
      assertTrue(batch.hasPrefetchedBatch());
      assertFalse(cursor.isOnLastRow());
      assertFalse(cursor.isAfterLast());
    }
  }

  @Test
  public void testPrefetchNextBatchForIsLastAtEnd() throws Exception {
    try (ArrowCursorTestUtils.TestResources resourcesHolder =
        ArrowCursorTestUtils.createIntResources(new int[] {1})) {
      ArrowResources resources = resourcesHolder.getResources();
      CursorState cursor = new CursorState();
      BatchState batch = new BatchState();
      SchemaState schema = new SchemaState();
      ArrowBatchManager manager = new ArrowBatchManager(cursor, batch, resources, schema);

      assertTrue(manager.fetchNextRow());
      manager.prefetchNextBatchForIsLast();

      assertFalse(batch.hasPrefetchedBatch());
      assertTrue(cursor.isOnLastRow());
      assertFalse(cursor.isAfterLast());
    }
  }
}
