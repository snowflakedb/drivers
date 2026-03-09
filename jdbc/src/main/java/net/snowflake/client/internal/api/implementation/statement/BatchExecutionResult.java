package net.snowflake.client.internal.api.implementation.statement;

import lombok.Value;

@Value
final class BatchExecutionResult {
  int[] intCounts;
  long[] longCounts;
}
