package net.snowflake.client.internal.core.arrow.cursor;

public final class CursorState {
  private boolean wasNull = false;
  private int currentRow = -1;
  private boolean afterLast = false;

  private int currentRowInBatch = -1;
  private int currentBatchRowCount = 0;
  /** The reader was drained while the cursor was still before the first row. */
  private boolean drained = false;

  public void reset() {
    wasNull = false;
    currentRow = -1;
    afterLast = false;
    currentRowInBatch = -1;
    currentBatchRowCount = 0;
    drained = false;
  }

  public boolean wasNull() {
    return wasNull;
  }

  public void setWasNull(boolean wasNull) {
    this.wasNull = wasNull;
  }

  public int getCurrentRow() {
    return currentRow;
  }

  public void incrementRow() {
    currentRow++;
  }

  public boolean isAfterLast() {
    return afterLast;
  }

  public void setAfterLast() {
    this.afterLast = true;
  }

  public int getCurrentRowInBatch() {
    return currentRowInBatch;
  }

  void incrementRowInBatch() {
    currentRowInBatch++;
  }

  int getCurrentBatchRowCount() {
    return currentBatchRowCount;
  }

  boolean hasNextRowInBatch() {
    return currentRowInBatch + 1 < currentBatchRowCount;
  }

  void startNewBatch(int rowCount) {
    this.currentBatchRowCount = rowCount;
    this.currentRowInBatch = 0;
  }

  /**
   * Keeps a batch that was read before {@code next()} positioned the cursor on it. The cursor stays
   * before the first row; the next {@code next()} advances into this batch instead of reading
   * another one.
   */
  void holdBatch(int rowCount) {
    this.currentBatchRowCount = rowCount;
    this.currentRowInBatch = -1;
    this.drained = false;
  }

  void markDrained() {
    this.drained = true;
  }

  boolean isDrained() {
    return drained;
  }

  boolean hasLoadedBatch() {
    return currentBatchRowCount > 0;
  }
}
