package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.MultiStatementResult;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import org.junit.jupiter.api.Test;

class MultiStatementStateTest {

  @Test
  void fromMultiStatementResult() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("parent-qid"))
            .addQueryIds("child-1")
            .addQueryIds("child-2")
            .addQueryIds("child-3")
            .addStatementTypeIds(4096L)
            .addStatementTypeIds(4096L)
            .addStatementTypeIds(0L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertEquals("parent-qid", state.getParentQueryId());
    assertFalse(state.isEmpty());
    assertTrue(state.hasMore());
  }

  @Test
  void emptyMultiStatement() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("parent-qid"))
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertTrue(state.isEmpty());
    assertFalse(state.hasMore());
    assertNull(state.advance());
  }

  @Test
  void advanceIteratesThroughChildren() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addQueryIds("q2")
            .addQueryIds("q3")
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertTrue(state.hasMore());
    assertEquals("q1", state.advance());
    assertEquals(0, state.currentIndex());

    assertTrue(state.hasMore());
    assertEquals("q2", state.advance());
    assertEquals(1, state.currentIndex());

    assertTrue(state.hasMore());
    assertEquals("q3", state.advance());
    assertEquals(2, state.currentIndex());

    assertFalse(state.hasMore());
    assertNull(state.advance());
  }

  @Test
  void hasStatementTypeForRespectsAvailableTypes() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addQueryIds("q2")
            .addQueryIds("q3")
            .addStatementTypeIds(4096L)
            .addStatementTypeIds(0L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertTrue(state.hasStatementTypeFor(0));
    assertTrue(state.hasStatementTypeFor(1));
    assertFalse(state.hasStatementTypeFor(2));
  }

  @Test
  void producesResultSetDelegatesToClassifier() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addQueryIds("q2")
            .addStatementTypeIds(0x1000L) // SELECT → produces result set
            .addStatementTypeIds(0x3000L) // DML → does not produce result set
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertTrue(state.producesResultSet(0));
    assertFalse(state.producesResultSet(1));
  }

  @Test
  void producesResultSetReturnsFalseForOutOfBoundsIndex() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addStatementTypeIds(4096L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertFalse(state.producesResultSet(5));
  }
}
