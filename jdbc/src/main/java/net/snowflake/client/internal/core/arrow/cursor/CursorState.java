package net.snowflake.client.internal.core.arrow.cursor;

public final class CursorState {
  private boolean wasNull = false;
  private int currentRow = -1;
  private boolean afterLast = false;

  private int currentRowInBatch = -1;
  private int currentBatchRowCount = 0;

  public void reset() {
    wasNull = false;
    currentRow = -1;
    afterLast = false;
    currentRowInBatch = -1;
    currentBatchRowCount = 0;
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

  boolean hasLoadedBatch() {
    return currentRowInBatch >= 0;
  }
}
