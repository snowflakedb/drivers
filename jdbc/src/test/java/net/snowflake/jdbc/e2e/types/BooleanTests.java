package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class BooleanTests extends SnowflakeIntegrationTestBase {
  private static final String BOOLEAN_TYPE = "BOOLEAN";
  private static final int LARGE_HALF_COUNT = 500_000;
  private static final int LARGE_RESULT_SET_SIZE = LARGE_HALF_COUNT * 2;

  @Test
  public void shouldCastBooleanValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
    String sql =
        "SELECT TRUE::" + BOOLEAN_TYPE + ", FALSE::" + BOOLEAN_TYPE + ", TRUE::" + BOOLEAN_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then All values should be returned as appropriate type
          List<Boolean> row = assertBooleanTypes(resultSet, 3);

          // And Values should match [TRUE, FALSE, TRUE]
          assertEquals(Arrays.asList(true, false, true), row);
        });
  }

  @Test
  public void shouldSelectBooleanLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    String sql = "SELECT TRUE::" + BOOLEAN_TYPE + ", FALSE::" + BOOLEAN_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [TRUE, FALSE]
          assertSingleRow(resultSet, Arrays.asList(true, false));
        });
  }

  @Test
  public void shouldHandleNULLValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
    String sql =
        "SELECT FALSE::"
            + BOOLEAN_TYPE
            + ", NULL::"
            + BOOLEAN_TYPE
            + ", TRUE::"
            + BOOLEAN_TYPE
            + ", NULL::"
            + BOOLEAN_TYPE;
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain [FALSE, NULL, TRUE, NULL]
          assertSingleRow(resultSet, Arrays.asList(false, null, true, null));
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGenerator() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
    String sql =
        "SELECT (seq8() % 2 = 0)::"
            + BOOLEAN_TYPE
            + " AS col FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))";
    withQueryResult(
        connection,
        sql,
        resultSet -> {

          // Then Result should contain 500000 TRUE and 500000 FALSE values
          assertBooleanResults(resultSet, LARGE_HALF_COUNT, LARGE_HALF_COUNT, 0);
        });
  }

  @Test
  public void shouldSelectBooleanValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
    String tableName =
        createTempTable(
            connection,
            "ud_boolean_",
            "c1 " + BOOLEAN_TYPE + ", c2 " + BOOLEAN_TYPE + ", c3 " + BOOLEAN_TYPE);

    // And Row (TRUE, FALSE, TRUE) is inserted
    execute(connection, "INSERT INTO " + tableName + " VALUES (TRUE, FALSE, TRUE)");

    // When Query "SELECT * FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT * FROM " + tableName,
        resultSet -> {

          // Then Result should contain [TRUE, FALSE, TRUE]
          assertSingleRow(resultSet, Arrays.asList(true, false, true));
        });
  }

  @Test
  public void shouldHandleNULLValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with BOOLEAN column exists
    String tableName = createTempTable(connection, "ud_boolean_", "col " + BOOLEAN_TYPE);

    // And Rows [NULL, TRUE, FALSE] are inserted
    execute(connection, "INSERT INTO " + tableName + " VALUES (NULL), (TRUE), (FALSE)");

    // When Query "SELECT * FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName,
        resultSet -> {

          // Then Result should contain [NULL, TRUE, FALSE] in any order
          assertBooleanResults(resultSet, 1, 1, 1);
        });
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
    String tableName = createTempTable(connection, "ud_boolean_", "col " + BOOLEAN_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT (seq8() % 2 = 0)::"
            + BOOLEAN_TYPE
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    // When Query "SELECT col FROM <table>" is executed
    withQueryResult(
        connection,
        "SELECT col FROM " + tableName,
        resultSet -> {

          // Then Result should contain 500000 TRUE and 500000 FALSE values
          assertBooleanResults(resultSet, LARGE_HALF_COUNT, LARGE_HALF_COUNT, 0);
        });
  }

  @Test
  public void shouldSelectBooleanUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values
    // [TRUE, FALSE, TRUE]
    withPreparedQueryResult(
        connection,
        "SELECT ?::" + BOOLEAN_TYPE + ", ?::" + BOOLEAN_TYPE + ", ?::" + BOOLEAN_TYPE,
        ps -> {
          ps.setBoolean(1, true);
          ps.setBoolean(2, false);
          ps.setBoolean(3, true);
        },
        resultSet -> {

          // Then Result should contain [TRUE, FALSE, TRUE]
          assertSingleRow(resultSet, Arrays.asList(true, false, true));
        });

    // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
    withPreparedQueryResult(
        connection,
        "SELECT ?::" + BOOLEAN_TYPE,
        ps -> ps.setNull(1, Types.BOOLEAN),
        resultSet -> {

          // Then Result should contain [NULL]
          assertSingleRow(resultSet, Arrays.asList((Boolean) null));
        });
  }

  private static List<Boolean> assertBooleanTypes(ResultSet resultSet, int columnCount)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: " + BOOLEAN_TYPE);
    List<Boolean> values = new ArrayList<>();
    for (int i = 1; i <= columnCount; i++) {
      Object objectValue = resultSet.getObject(i);
      assertInstanceOf(
          Boolean.class,
          objectValue,
          "Column " + i + " getObject should return Boolean for " + BOOLEAN_TYPE);
      assertFalse(resultSet.wasNull(), "Column " + i + " should not be NULL for " + BOOLEAN_TYPE);
      values.add((Boolean) objectValue);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: " + BOOLEAN_TYPE);
    return values;
  }

  private static void assertSingleRow(ResultSet resultSet, List<Boolean> expectedValues)
      throws Exception {
    assertTrue(resultSet.next(), "Expected one row for type: " + BOOLEAN_TYPE);
    for (int i = 0; i < expectedValues.size(); i++) {
      assertBooleanGetters(
          resultSet,
          i + 1,
          expectedValues.get(i),
          "Column " + (i + 1) + " mismatch for " + BOOLEAN_TYPE);
    }
    assertFalse(resultSet.next(), "Expected exactly one row for type: " + BOOLEAN_TYPE);
  }

  private static void assertBooleanResults(
      ResultSet resultSet, int expectedTrue, int expectedFalse, int expectedNull) throws Exception {
    BooleanCounts counts = new BooleanCounts();
    while (resultSet.next()) {
      Object objectValue = resultSet.getObject(1);
      if (objectValue == null) {
        counts.nullCount++;
        assertTrue(resultSet.wasNull(), "getObject should set wasNull() for NULL " + BOOLEAN_TYPE);
        assertFalse(
            resultSet.getBoolean(1), "getBoolean should return false for NULL " + BOOLEAN_TYPE);
        assertTrue(resultSet.wasNull(), "getBoolean should set wasNull() for NULL " + BOOLEAN_TYPE);
      } else {
        assertInstanceOf(
            Boolean.class, objectValue, "getObject should return Boolean for " + BOOLEAN_TYPE);
        assertFalse(resultSet.wasNull(), "getObject should not be NULL for " + BOOLEAN_TYPE);
        boolean boolValue = resultSet.getBoolean(1);
        assertFalse(resultSet.wasNull(), "getBoolean should not be NULL for " + BOOLEAN_TYPE);
        assertEquals(objectValue, boolValue, "Boolean value mismatch");
        if (boolValue) {
          counts.trueCount++;
        } else {
          counts.falseCount++;
        }
      }
    }

    assertEquals(expectedTrue, counts.trueCount, "Unexpected TRUE count for " + BOOLEAN_TYPE);
    assertEquals(expectedFalse, counts.falseCount, "Unexpected FALSE count for " + BOOLEAN_TYPE);
    assertEquals(expectedNull, counts.nullCount, "Unexpected NULL count for " + BOOLEAN_TYPE);
  }

  private static void assertBooleanGetters(
      ResultSet resultSet, int columnIndex, Boolean expected, String message) throws Exception {
    if (expected == null) {
      assertNull(resultSet.getObject(columnIndex), message + " (getObject should be NULL)");
      assertTrue(resultSet.wasNull(), message + " (getObject should set wasNull)");
      assertFalse(resultSet.getBoolean(columnIndex), message + " (getBoolean should return false)");
      assertTrue(resultSet.wasNull(), message + " (getBoolean should set wasNull)");
      return;
    }

    Object objectValue = resultSet.getObject(columnIndex);
    assertInstanceOf(Boolean.class, objectValue, message + " (getObject should return Boolean)");
    assertEquals(expected, objectValue, message + " (getObject)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");

    assertEquals(expected, resultSet.getBoolean(columnIndex), message + " (getBoolean)");
    assertFalse(resultSet.wasNull(), message + " (getBoolean should not be NULL)");
  }

  private static final class BooleanCounts {
    private int trueCount;
    private int falseCount;
    private int nullCount;
  }
}
