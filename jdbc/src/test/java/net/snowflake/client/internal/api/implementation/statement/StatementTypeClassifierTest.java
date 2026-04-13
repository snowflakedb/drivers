package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class StatementTypeClassifierTest {

  // Statement type IDs from StatementType enum
  private static final long INSERT_TYPE_ID = 0x3100L;
  private static final long UNMAPPED_DML_SUBTYPE_ID = 0x3901L;
  private static final long SHOW_TYPE_ID = 0x4400L;
  private static final long DESCRIBE_TYPE_ID = 0x4500L;
  private static final long LIST_TYPE_ID = 0x4701L;
  private static final long ALTER_USER_MANAGE_PATS_TYPE_ID = 0x6244L;
  private static final long GET_TYPE_ID = 0x7101L;
  private static final long PUT_TYPE_ID = 0x7102L;
  private static final long REMOVE_TYPE_ID = 0x7103L;
  private static final long USE_TYPE_ID = 0x4300L;
  private static final long TCL_TYPE_ID = 0x5000L;
  private static final long DDL_SUBTYPE_ID = 0x6100L;

  @Test
  void testMissingStatementTypeBehavesLikeUnknown() {
    ResultSetDescriptor descriptor = ResultSetDescriptor.newBuilder().build();

    assertTrue(
        StatementTypeClassifier.producesResultSet(descriptor),
        "Missing statement type should fall back to UNKNOWN result-set semantics");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(descriptor),
        "Missing statement type should not expose an update count");
  }

  @Test
  void testKnownDmlUsesRowsAffected() {
    ResultSetDescriptor descriptor = descriptor(INSERT_TYPE_ID, 7L);

    assertFalse(
        StatementTypeClassifier.producesResultSet(descriptor),
        "INSERT should not produce a result set");
    assertEquals(
        7L,
        StatementTypeClassifier.getUpdateCount(descriptor),
        "INSERT should report rows affected");
  }

  @Test
  void testUnmappedDmlSubtypeBehavesLikeUnknown() {
    ResultSetDescriptor descriptor = descriptor(UNMAPPED_DML_SUBTYPE_ID, 5L);

    assertTrue(
        StatementTypeClassifier.producesResultSet(descriptor),
        "Unmapped DML subtypes should match SFStatementType.UNKNOWN");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(descriptor),
        "Unmapped DML subtypes should not expose an update count");
  }

  @ParameterizedTest
  @ValueSource(
      longs = {
        SHOW_TYPE_ID,
        DESCRIBE_TYPE_ID,
        LIST_TYPE_ID,
        ALTER_USER_MANAGE_PATS_TYPE_ID,
        GET_TYPE_ID,
        PUT_TYPE_ID,
        REMOVE_TYPE_ID
      })
  void testResultSetProducingSpecialStatementTypes(long statementTypeId) {
    ResultSetDescriptor descriptor = descriptor(statementTypeId, 11L);

    assertTrue(
        StatementTypeClassifier.producesResultSet(descriptor),
        "Special statement types that produce result sets should keep JDBC semantics");
    assertEquals(
        -1L,
        StatementTypeClassifier.getUpdateCount(descriptor),
        "Result-set-producing statements should not expose an update count");
  }

  @ParameterizedTest
  @ValueSource(longs = {USE_TYPE_ID, TCL_TYPE_ID, DDL_SUBTYPE_ID})
  void testNonDmlNonResultSetStatementTypesReturnZeroUpdateCount(long statementTypeId) {
    ResultSetDescriptor descriptor = descriptor(statementTypeId, 13L);

    assertFalse(
        StatementTypeClassifier.producesResultSet(descriptor),
        "SCL, TCL, and DDL statement types should not produce a result set");
    assertEquals(
        0L,
        StatementTypeClassifier.getUpdateCount(descriptor),
        "Non-DML statements should return zero update count");
  }

  private static ResultSetDescriptor descriptor(long statementTypeId, long rowsAffected) {
    return ResultSetDescriptor.newBuilder()
        .setStatementTypeId(statementTypeId)
        .setRowsAffected(rowsAffected)
        .build();
  }
}
