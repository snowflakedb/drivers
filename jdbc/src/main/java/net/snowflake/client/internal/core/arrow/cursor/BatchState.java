package net.snowflake.client.internal.core.arrow.cursor;

public final class BatchState {
  private int currentRowInBatch = -1;
  private int currentBatchRowCount = 0;
  private boolean hasLoadedBatch = false;
  private boolean hasPrefetchedBatch = false;

  public void reset() {
    currentRowInBatch = -1;
    currentBatchRowCount = 0;
    hasLoadedBatch = false;
    hasPrefetchedBatch = false;
  }

  public int getCurrentRowInBatch() {
    return currentRowInBatch;
  }

  void resetCurrentRowInBatch() {
    this.currentRowInBatch = 0;
  }

  void incrementRowInBatch() {
    currentRowInBatch++;
  }

  int getCurrentBatchRowCount() {
    return currentBatchRowCount;
  }

  void setCurrentBatchRowCount(int currentBatchRowCount) {
    this.currentBatchRowCount = currentBatchRowCount;
  }

  boolean hasLoadedBatch() {
    return hasLoadedBatch;
  }

  void resetHasLoadedBatch() {
    this.hasLoadedBatch = true;
  }

  boolean hasPrefetchedBatch() {
    return hasPrefetchedBatch;
  }

  void setHasPrefetchedBatch(boolean hasPrefetchedBatch) {
    this.hasPrefetchedBatch = hasPrefetchedBatch;
  }

  public boolean isAtLastRow() {
    return currentRowInBatch == currentBatchRowCount - 1;
  }
}
