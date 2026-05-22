package net.snowflake.client.internal.api.implementation.statement;

import java.util.List;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.MultiStatementResult;

/**
 * Navigation state for multi-statement query results.
 *
 * <p>Tracks child query IDs and statement type IDs returned by the server, along with the current
 * position in the child result sequence.
 */
@RequiredArgsConstructor
class MultiStatementState {

  private final String parentQueryId;
  private final List<String> childQueryIds;
  private final List<Long> childStatementTypeIds;
  private int nextIndex = 0;

  static MultiStatementState from(MultiStatementResult multi) {
    return new MultiStatementState(
        multi.getParent().getQueryId(), multi.getQueryIdsList(), multi.getStatementTypeIdsList());
  }

  String getParentQueryId() {
    return parentQueryId;
  }

  boolean isEmpty() {
    return childQueryIds.isEmpty();
  }

  boolean hasMore() {
    return nextIndex < childQueryIds.size();
  }

  String advance() {
    if (!hasMore()) {
      return null;
    }
    return childQueryIds.get(nextIndex++);
  }

  int currentIndex() {
    return nextIndex - 1;
  }

  /**
   * Returns whether the child at the given index produces a result set, using the statement type ID
   * if available.
   */
  boolean producesResultSet(int index) {
    if (index < childStatementTypeIds.size()) {
      return StatementTypeClassifier.producesResultSet(childStatementTypeIds.get(index));
    }
    return false;
  }

  boolean hasStatementTypeFor(int index) {
    return index < childStatementTypeIds.size();
  }
}
