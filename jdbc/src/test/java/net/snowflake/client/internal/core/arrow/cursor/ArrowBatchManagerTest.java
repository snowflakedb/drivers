package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import org.junit.jupiter.api.Test;

public class ArrowBatchManagerTest {

  @Test
  public void testFetchNextRowSkipsEmptyBatches() throws Exception {
    try (ArrowCursorTestUtils.TestResources resourcesHolder =
        ArrowCursorTestUtils.createIntResources(
            new int[] {}, new int[] {10, 20}, new int[] {}, new int[] {30})) {
      ArrowResources resources = resourcesHolder.getResources();
      CursorState cursor = new CursorState();
      ArrowBatchManager manager = new ArrowBatchManager(cursor, resources);

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

  @Test
  public void shouldTranslateBatchAdvanceIoExceptionToSfSqlException() {
    IOException advanceError = new IOException("stream broken");
    ArrowResources resources =
        new ArrowResources(
            new ArrowBatchReader() {
              @Override
              public boolean loadNextBatch() throws IOException {
                throw advanceError;
              }

              @Override
              public ColumnBatch getBatch() {
                throw new AssertionError("batch is never read when advancing fails");
              }

              @Override
              public void close() {}
            });
    ArrowBatchManager manager = new ArrowBatchManager(new CursorState(), resources);

    SFSQLException thrown = assertThrows(SFSQLException.class, manager::fetchNextRow);
    assertEquals("Unable to advance Arrow results", thrown.getMessage());
    assertSame(advanceError, thrown.getCause());
  }

  @Test
  public void shouldNotTouchReaderOnceCursorIsAfterLast() {
    ArrowResources resources =
        new ArrowResources(
            new ArrowBatchReader() {
              @Override
              public boolean loadNextBatch() {
                throw new AssertionError("reader must not advance after last row");
              }

              @Override
              public ColumnBatch getBatch() {
                throw new AssertionError("reader must not be read after last row");
              }

              @Override
              public void close() {}
            });
    CursorState cursor = new CursorState();
    cursor.setAfterLast();

    assertFalse(new ArrowBatchManager(cursor, resources).fetchNextRow());
  }
}
