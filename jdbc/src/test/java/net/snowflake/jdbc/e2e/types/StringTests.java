package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class StringTests extends SnowflakeIntegrationTestBase {
  private static final String STRING_TYPE = "VARCHAR";
  private static final int LARGE_RESULT_SET_SIZE = 10_000;

  private static final String JAPANESE_TEXT = "\u65e5\u672c\u8a9e\u30c6\u30b9\u30c8";
  private static final String SNOWMAN = "\u26c4";
  private static final String COMBINING_CHAR_TEXT = "y\u0306es";
  private static final String SURROGATE_PAIR_TEXT = "\ud834\udd1e";

  @Test
  public void shouldCastStringValuesToAppropriateTypeForStringAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 'hello'::<type>, 'Hello World'::<type>, '日本語テスト'::<type>" is executed
    // Then All values should be returned as appropriate type
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT 'hello'::%1$s, 'Hello World'::%1$s, '%2$s'::%1$s", STRING_TYPE, JAPANESE_TEXT);
    assertSingleRow(connection, sql, Arrays.asList("hello", "Hello World", JAPANESE_TEXT));
  }

  @Test
  public void shouldSelectHardcodedStringLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3"
    // is executed
    // Then the result should contain:
    Connection connection = getDefaultConnection();
    assertSingleRow(
        connection,
        "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3",
        Arrays.asList("hello", "Hello World", "Snowflake Driver Test"));
  }

  @Test
  public void shouldSelectStringLiteralsWithCornerCaseValues() throws Exception {
    // Given Snowflake client is logged in
    // When Query selecting corner case string literals is executed
    // Then the result should contain expected corner case string values
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT ''::%1$s, 'X'::%1$s, '   '::%1$s, CHAR(9)::%1$s, CHAR(10)::%1$s, '%2$s'::%1$s,"
                + " '%3$s'::%1$s, ''''::%1$s, CHAR(92)::%1$s, NULL::%1$s, '%4$s'::%1$s, '%5$s'::%1$s",
            STRING_TYPE, SNOWMAN, JAPANESE_TEXT, COMBINING_CHAR_TEXT, SURROGATE_PAIR_TEXT);
    assertSingleRow(
        connection,
        sql,
        Arrays.asList(
            "",
            "X",
            "   ",
            "\t",
            "\n",
            SNOWMAN,
            JAPANESE_TEXT,
            "'",
            "\\",
            null,
            COMBINING_CHAR_TEXT,
            SURROGATE_PAIR_TEXT));
  }

  @Test
  public void shouldSelectHardcodedStringValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And A temporary table with VARCHAR column is created
    // And The table is populated with string values
    // When Query "SELECT * FROM {table}" is executed
    // Then the result should contain the inserted hardcoded string values
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_string_", "id INT, col " + STRING_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (1, 'hello'), (2, 'Hello World'), (3, 'Snowflake Driver Test')");

    assertRowsInOrder(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY id",
        Arrays.asList("hello", "Hello World", "Snowflake Driver Test"));
  }

  @Test
  public void shouldSelectCornerCaseStringValuesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And A temporary table with VARCHAR column is created
    // And The table is populated with corner case string values
    // When Query "SELECT * FROM {table}" is executed
    // Then the result should contain the inserted corner case string values
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_string_", "id INT, col " + STRING_TYPE);
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT 1, '' "
            + "UNION ALL SELECT 2, 'X' "
            + "UNION ALL SELECT 3, '   ' "
            + "UNION ALL SELECT 4, CHAR(9) "
            + "UNION ALL SELECT 5, CHAR(10) "
            + "UNION ALL SELECT 6, '"
            + SNOWMAN
            + "' "
            + "UNION ALL SELECT 7, '"
            + JAPANESE_TEXT
            + "' "
            + "UNION ALL SELECT 8, '''' "
            + "UNION ALL SELECT 9, CHAR(92) "
            + "UNION ALL SELECT 10, NULL "
            + "UNION ALL SELECT 11, '"
            + COMBINING_CHAR_TEXT
            + "' "
            + "UNION ALL SELECT 12, '"
            + SURROGATE_PAIR_TEXT
            + "'");

    assertRowsInOrder(
        connection,
        "SELECT col FROM " + tableName + " ORDER BY id",
        Arrays.asList(
            "",
            "X",
            "   ",
            "\t",
            "\n",
            SNOWMAN,
            JAPANESE_TEXT,
            "'",
            "\\",
            null,
            COMBINING_CHAR_TEXT,
            SURROGATE_PAIR_TEXT));
  }

  @Test
  public void shouldDownloadStringDataInMultipleChunks() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT
    // => 10000)) v ORDER BY id" is executed
    // Then there are 10000 rows returned
    // And all returned string values should match the generated values in order
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val "
            + "FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      int expected = 0;
      while (resultSet.next()) {
        assertEquals(expected, resultSet.getLong(1), "ID mismatch at row " + expected);
        assertFalse(resultSet.wasNull(), "ID should not be NULL at row " + expected);
        assertStringColumn(
            resultSet,
            2,
            String.valueOf(expected),
            "String value mismatch for " + STRING_TYPE + ", row " + expected);
        expected++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, expected, "Unexpected row count for " + STRING_TYPE);
    }
  }

  private static void assertSingleRow(
      Connection connection, String sql, List<String> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + STRING_TYPE);
      for (int i = 0; i < expectedValues.size(); i++) {
        assertStringColumn(
            resultSet,
            i + 1,
            expectedValues.get(i),
            "Column " + (i + 1) + " mismatch for " + STRING_TYPE);
      }
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + STRING_TYPE);
    }
  }

  private static void assertRowsInOrder(
      Connection connection, String sql, List<String> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      for (int i = 0; i < expectedValues.size(); i++) {
        assertTrue(resultSet.next(), "Missing row " + i + " for " + STRING_TYPE);
        assertStringColumn(
            resultSet,
            1,
            expectedValues.get(i),
            "Value mismatch for " + STRING_TYPE + ", row " + i);
      }
      assertFalse(resultSet.next(), "Unexpected extra rows for " + STRING_TYPE);
    }
  }

  private static void assertStringColumn(
      ResultSet resultSet, int columnIndex, String expected, String message) throws Exception {
    if (expected == null) {
      assertNull(resultSet.getObject(columnIndex), message + " (getObject should be NULL)");
      assertTrue(resultSet.wasNull(), message + " (getObject should set wasNull)");
      assertNull(resultSet.getString(columnIndex), message + " (getString should be NULL)");
      assertTrue(resultSet.wasNull(), message + " (getString should set wasNull)");
      return;
    }

    Object objectValue = resultSet.getObject(columnIndex);
    assertInstanceOf(String.class, objectValue, message + " (getObject should return String)");
    assertEquals(expected, objectValue, message + " (getObject)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");

    assertEquals(expected, resultSet.getString(columnIndex), message + " (getString)");
    assertFalse(resultSet.wasNull(), message + " (getString should not be NULL)");
  }
}
