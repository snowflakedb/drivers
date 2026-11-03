package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

public class CursorStateTest {

  @Test
  public void shouldStartBeforeFirstRowWithNoLoadedBatch() {
    CursorState state = new CursorState();

    assertFalse(state.wasNull());
    assertEquals(-1, state.getCurrentRow());
    assertEquals(-1, state.getCurrentRowInBatch());
    assertEquals(0, state.getCurrentBatchRowCount());
    assertFalse(state.isAfterLast());
    assertFalse(state.hasLoadedBatch());
    assertFalse(state.hasNextRowInBatch());
  }

  @Test
  public void shouldTrackRowPositionWithinBatch() {
    CursorState state = new CursorState();

    state.startNewBatch(2);
    assertTrue(state.hasLoadedBatch());
    assertEquals(0, state.getCurrentRowInBatch());
    assertEquals(2, state.getCurrentBatchRowCount());
    assertTrue(state.hasNextRowInBatch());

    state.incrementRow();
    state.incrementRowInBatch();
    assertEquals(0, state.getCurrentRow());
    assertEquals(1, state.getCurrentRowInBatch());
    assertFalse(state.hasNextRowInBatch());
  }

  @Test
  public void shouldRestoreInitialStateOnReset() {
    CursorState state = new CursorState();
    state.startNewBatch(2);
    state.incrementRow();
    state.setWasNull(true);
    state.setAfterLast();
    assertTrue(state.wasNull());
    assertTrue(state.isAfterLast());

    state.reset();

    assertFalse(state.wasNull());
    assertEquals(-1, state.getCurrentRow());
    assertEquals(-1, state.getCurrentRowInBatch());
    assertEquals(0, state.getCurrentBatchRowCount());
    assertFalse(state.isAfterLast());
    assertFalse(state.hasLoadedBatch());
  }
}
