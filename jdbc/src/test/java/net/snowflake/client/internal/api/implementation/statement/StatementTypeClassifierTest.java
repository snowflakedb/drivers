package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteResult;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class StatementTypeClassifierTest {

  @Test
  void testMissingStatementTypeBehavesLikeUnknown() {
    ExecuteResult executeResult = ExecuteResult.newBuilder().build();

    assertTrue(
        StatementTypeClassifier.producesResultSet(executeResult),
        "Missing statement type should fall back to UNKNOWN result-set semantics");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(executeResult),
        "Missing statement type should not expose an update count");
  }

  @Test
  void testKnownDmlUsesRowsAffected() {
    ExecuteResult executeResult = executeResult(0x3100L, 7L);

    assertFalse(
        StatementTypeClassifier.producesResultSet(executeResult),
        "INSERT should not produce a result set");
    assertEquals(
        7L,
        StatementTypeClassifier.getUpdateCount(executeResult),
        "INSERT should report rows affected");
  }

  @Test
  void testUnmappedDmlSubtypeBehavesLikeUnknown() {
    ExecuteResult executeResult = executeResult(0x3901L, 5L);

    assertTrue(
        StatementTypeClassifier.producesResultSet(executeResult),
        "Unmapped DML subtypes should match SFStatementType.UNKNOWN");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(executeResult),
        "Unmapped DML subtypes should not expose an update count");
  }

  @ParameterizedTest
  @ValueSource(longs = {0x4400L, 0x4500L, 0x4701L, 0x6244L, 0x7101L, 0x7102L, 0x7103L})
  void testResultSetProducingSpecialStatementTypes(long statementTypeId) {
    ExecuteResult executeResult = executeResult(statementTypeId, 11L);

    assertTrue(
        StatementTypeClassifier.producesResultSet(executeResult),
        "Special statement types that produce result sets should keep JDBC semantics");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(executeResult),
        "Result-set-producing statements should not expose an update count");
  }

  @ParameterizedTest
  @ValueSource(longs = {0x4300L, 0x5000L, 0x6100L})
  void testNonDmlNonResultSetStatementTypesReturnZeroUpdateCount(long statementTypeId) {
    ExecuteResult executeResult = executeResult(statementTypeId, 13L);

    assertFalse(
        StatementTypeClassifier.producesResultSet(executeResult),
        "SCL, TCL, and DDL statement types should not produce a result set");
    assertEquals(
        0L,
        StatementTypeClassifier.getUpdateCount(executeResult),
        "Non-DML statements should return zero update count");
  }

  private static ExecuteResult executeResult(long statementTypeId, long rowsAffected) {
    return ExecuteResult.newBuilder()
        .setStatementTypeId(statementTypeId)
        .setRowsAffected(rowsAffected)
        .build();
  }
}
