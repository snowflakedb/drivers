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
    assertEquals(2, state.currentIndex(), "Advancing past the end must not move the cursor");
  }

  @Test
  void currentIndexBeforeFirstAdvanceHasNoStatementType() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addStatementTypeIds(0x1000L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertEquals(-1, state.currentIndex());
    assertFalse(state.hasStatementTypeFor(state.currentIndex()));
    assertFalse(state.producesResultSet(state.currentIndex()));
  }

  @Test
  void negativeIndexHasNoStatementTypeWhenTypesAreEmpty() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertFalse(state.hasStatementTypeFor(-1));
    assertFalse(state.producesResultSet(-1));
  }

  @Test
  void childWithoutStatementTypeReportsNoTypeAndNoResultSet() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addQueryIds("q2")
            .addStatementTypeIds(0x1000L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertEquals("q1", state.advance());
    assertTrue(state.producesResultSet(state.currentIndex()));
    assertEquals("q2", state.advance());
    assertFalse(
        state.hasStatementTypeFor(state.currentIndex()),
        "Callers must fall back to the child descriptor when the server sent fewer types");
    assertFalse(state.producesResultSet(state.currentIndex()));
  }

  @Test
  void extraStatementTypesBeyondChildrenDoNotExtendNavigation() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("q1")
            .addStatementTypeIds(0x3100L)
            .addStatementTypeIds(0x1000L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertEquals("q1", state.advance());
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
  void producesResultSetReflectsCopyUnloadAndPartitionedUnloadStatementTypes() {
    MultiStatementResult multi =
        MultiStatementResult.newBuilder()
            .setParent(ResultSetDescriptor.newBuilder().setQueryId("p"))
            .addQueryIds("copy-child")
            .addQueryIds("unload-child")
            .addQueryIds("partitioned-unload-child")
            .addStatementTypeIds(0x3600L)
            .addStatementTypeIds(0x3700L)
            .addStatementTypeIds(0x3701L)
            .build();

    MultiStatementState state = MultiStatementState.from(multi);

    assertFalse(
        state.producesResultSet(0),
        "COPY child should follow JDBC execute() semantics without a result set");
    assertFalse(
        state.producesResultSet(1),
        "Plain UNLOAD child should follow JDBC execute() semantics without a result set");
    assertTrue(
        state.producesResultSet(2),
        "Partitioned unload subtypes stay unmapped and expose result-set semantics");
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
