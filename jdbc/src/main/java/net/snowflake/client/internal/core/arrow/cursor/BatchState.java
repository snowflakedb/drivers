package net.snowflake.client.internal.core.arrow.cursor;

public final class BatchState {
  private int currentRowInBatch = -1;
  private int currentBatchRowCount = 0;

  public void reset() {
    currentRowInBatch = -1;
    currentBatchRowCount = 0;
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
