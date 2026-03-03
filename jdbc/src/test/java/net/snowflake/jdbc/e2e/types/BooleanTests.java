package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Types;
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
    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
    // Then All values should be returned as appropriate type
    // And Values should match [TRUE, FALSE, TRUE]
    Connection connection = getDefaultConnection();
    assertSingleRow(
        connection,
        "SELECT TRUE::" + BOOLEAN_TYPE + ", FALSE::" + BOOLEAN_TYPE + ", TRUE::" + BOOLEAN_TYPE,
        Arrays.asList(true, false, true));
  }

  @Test
  public void shouldSelectBooleanLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    // Then Result should contain [TRUE, FALSE]
    Connection connection = getDefaultConnection();
    assertSingleRow(
        connection,
        "SELECT TRUE::" + BOOLEAN_TYPE + ", FALSE::" + BOOLEAN_TYPE,
        Arrays.asList(true, false));
  }

  @Test
  public void shouldHandleNULLValuesFromLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
    // Then Result should contain [FALSE, NULL, TRUE, NULL]
    Connection connection = getDefaultConnection();
    assertSingleRow(
        connection,
        "SELECT FALSE::"
            + BOOLEAN_TYPE
            + ", NULL::"
            + BOOLEAN_TYPE
            + ", TRUE::"
            + BOOLEAN_TYPE
            + ", NULL::"
            + BOOLEAN_TYPE,
        Arrays.asList(false, null, true, null));
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGenerator() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
    // Then Result should contain 500000 TRUE and 500000 FALSE values
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT (seq8() % 2 = 0)::"
            + BOOLEAN_TYPE
            + " AS col FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))";
    assertBooleanCounts(connection, sql, LARGE_HALF_COUNT, LARGE_HALF_COUNT, 0);
  }

  @Test
  public void shouldSelectBooleanValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
    // And Row (TRUE, FALSE, TRUE) is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain [TRUE, FALSE, TRUE]
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection,
            "ud_boolean_",
            "c1 " + BOOLEAN_TYPE + ", c2 " + BOOLEAN_TYPE + ", c3 " + BOOLEAN_TYPE);
    execute(connection, "INSERT INTO " + tableName + " VALUES (TRUE, FALSE, TRUE)");
    assertSingleRow(connection, "SELECT * FROM " + tableName, Arrays.asList(true, false, true));
  }

  @Test
  public void shouldHandleNULLValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with BOOLEAN column exists
    // And Rows [NULL, TRUE, FALSE] are inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain [NULL, TRUE, FALSE] in any order
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_boolean_", "col " + BOOLEAN_TYPE);
    execute(connection, "INSERT INTO " + tableName + " VALUES (NULL), (TRUE), (FALSE)");
    assertBooleanCounts(connection, "SELECT col FROM " + tableName, 1, 1, 1);
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
    // When Query "SELECT col FROM <table>" is executed
    // Then Result should contain 500000 TRUE and 500000 FALSE values
    Connection connection = getDefaultConnection();
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
    assertBooleanCounts(
        connection, "SELECT col FROM " + tableName, LARGE_HALF_COUNT, LARGE_HALF_COUNT, 0);
  }

  @Test
  public void shouldSelectBooleanUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed with bound boolean values
    // [TRUE, FALSE, TRUE]
    // Then Result should contain [TRUE, FALSE, TRUE]
    // When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
    // Then Result should contain [NULL]
    Connection connection = getDefaultConnection();

    try (PreparedStatement preparedStatement =
        connection.prepareStatement(
            "SELECT ?::" + BOOLEAN_TYPE + ", ?::" + BOOLEAN_TYPE + ", ?::" + BOOLEAN_TYPE)) {
      preparedStatement.setBoolean(1, true);
      preparedStatement.setBoolean(2, false);
      preparedStatement.setBoolean(3, true);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: " + BOOLEAN_TYPE);
        assertBooleanColumn(resultSet, 1, true, "Column 1 mismatch for " + BOOLEAN_TYPE);
        assertBooleanColumn(resultSet, 2, false, "Column 2 mismatch for " + BOOLEAN_TYPE);
        assertBooleanColumn(resultSet, 3, true, "Column 3 mismatch for " + BOOLEAN_TYPE);
        assertFalse(resultSet.next(), "Expected exactly one row for type: " + BOOLEAN_TYPE);
      }
    }

    try (PreparedStatement preparedStatement =
        connection.prepareStatement("SELECT ?::" + BOOLEAN_TYPE)) {
      preparedStatement.setNull(1, Types.BOOLEAN);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        assertTrue(resultSet.next(), "Expected one row for type: " + BOOLEAN_TYPE);
        assertBooleanColumn(resultSet, 1, null, "Column 1 mismatch for " + BOOLEAN_TYPE);
        assertFalse(resultSet.next(), "Expected exactly one row for type: " + BOOLEAN_TYPE);
      }
    }
  }

  private static void assertSingleRow(
      Connection connection, String sql, List<Boolean> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + BOOLEAN_TYPE);
      for (int i = 0; i < expectedValues.size(); i++) {
        assertBooleanColumn(
            resultSet,
            i + 1,
            expectedValues.get(i),
            "Column " + (i + 1) + " mismatch for " + BOOLEAN_TYPE);
      }
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + BOOLEAN_TYPE);
    }
  }

  private static void assertBooleanCounts(
      Connection connection, String sql, int expectedTrue, int expectedFalse, int expectedNull)
      throws Exception {
    BooleanCounts counts = new BooleanCounts();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      while (resultSet.next()) {
        Object objectValue = resultSet.getObject(1);
        if (objectValue == null) {
          counts.nullCount++;
          assertTrue(
              resultSet.wasNull(), "getObject should set wasNull() for NULL " + BOOLEAN_TYPE);
          assertFalse(
              resultSet.getBoolean(1), "getBoolean should return false for NULL " + BOOLEAN_TYPE);
          assertTrue(
              resultSet.wasNull(), "getBoolean should set wasNull() for NULL " + BOOLEAN_TYPE);
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
    }

    assertEquals(expectedTrue, counts.trueCount, "Unexpected TRUE count for " + BOOLEAN_TYPE);
    assertEquals(expectedFalse, counts.falseCount, "Unexpected FALSE count for " + BOOLEAN_TYPE);
    assertEquals(expectedNull, counts.nullCount, "Unexpected NULL count for " + BOOLEAN_TYPE);
  }

  private static void assertBooleanColumn(
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
