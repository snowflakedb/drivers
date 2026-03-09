package net.snowflake.client.internal.api.implementation.statement;

final class BatchExecutionResult {
  private final int[] intCounts;
  private final long[] longCounts;

  BatchExecutionResult(int[] intCounts, long[] longCounts) {
    this.intCounts = intCounts;
    this.longCounts = longCounts;
  }

  int[] getIntCounts() {
    return intCounts;
  }

  long[] getLongCounts() {
    return longCounts;
  }
}
