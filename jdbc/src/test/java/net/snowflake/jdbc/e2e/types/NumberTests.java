package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.text.DecimalFormat;
import java.text.ParseException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class NumberTests extends SnowflakeIntegrationTestBase {
  private static final String NUMBER_TYPE = "NUMBER";
  private static final int LARGE_RESULT_SET_SIZE = 30_000;
  private static final BigDecimal DECIMAL_INCREMENT = new BigDecimal("0.12345");
  private static final String MAX_38_DIGIT = "99999999999999999999999999999999999999";
  private static final String MIN_38_DIGIT = "-99999999999999999999999999999999999999";
  private static final BigDecimal LONG_MIN_DECIMAL = BigDecimal.valueOf(Long.MIN_VALUE);
  private static final BigDecimal LONG_MAX_DECIMAL = BigDecimal.valueOf(Long.MAX_VALUE);

  @Test
  public void shouldCastNumberValuesToAppropriateTypeForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2),
    // 123.45::<type>(10,2)" is executed
    // Then All values should be returned as appropriate type
    // And Values should match [0, 123, 0.00, 123.45]
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT 0::%1$s(10,0), 123::%1$s(10,0), 0.00::%1$s(10,2), 123.45::%1$s(10,2)",
            NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection,
        sql,
        Arrays.asList(
            new BigDecimal("0"),
            new BigDecimal("123"),
            new BigDecimal("0.00"),
            new BigDecimal("123.45")));
  }

  @Test
  public void shouldSelectNumberLiteralsForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2),
    // -123.45::<type>(10,2), 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
    // Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT 0::%1$s(10,0), -456::%1$s(10,0), 1.50::%1$s(10,2), -123.45::%1$s(10,2), "
                + "123.456::%1$s(15,3), -789.012::%1$s(15,3)",
            NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection,
        sql,
        Arrays.asList(
            new BigDecimal("0"),
            new BigDecimal("-456"),
            new BigDecimal("1.50"),
            new BigDecimal("-123.45"),
            new BigDecimal("123.456"),
            new BigDecimal("-789.012")));
  }

  @Test
  public void shouldHandleHighPrecisionValuesFromLiteralsForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0),
    // 123456789012345678901234567890123456.78::<type>(38,2),
    // 1234567890123456789012345678.1234567890::<type>(38,10),
    // 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
    // Then Result should contain [12345678901234567890123456789012345678,
    // 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
    // 0.0000000000000000000000000000000000001]
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT 12345678901234567890123456789012345678::%1$s(38,0), "
                + "123456789012345678901234567890123456.78::%1$s(38,2), "
                + "1234567890123456789012345678.1234567890::%1$s(38,10), "
                + "0.0000000000000000000000000000000000001::%1$s(38,37)",
            NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection,
        sql,
        Arrays.asList(
            new BigDecimal("12345678901234567890123456789012345678"),
            new BigDecimal("123456789012345678901234567890123456.78"),
            new BigDecimal("1234567890123456789012345678.1234567890"),
            new BigDecimal("0.0000000000000000000000000000000000001")));
  }

  @Test
  public void shouldHandleScaleAndPrecisionBoundariesFromLiteralsForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0),
    // -99999999::<type>(8,0)" is executed
    // Then Result should contain [999.99, -999.99, 99999999, -99999999]
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT 999.99::%1$s(5,2), -999.99::%1$s(5,2), 99999999::%1$s(8,0), -99999999::%1$s(8,0)",
            NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection,
        sql,
        Arrays.asList(
            new BigDecimal("999.99"),
            new BigDecimal("-999.99"),
            new BigDecimal("99999999"),
            new BigDecimal("-99999999")));
  }

  @Test
  public void shouldHandleHighPrecisionBoundariesFromLiteralsForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0),
    // -99999999999999999999999999999999999999::<type>(38,0)" is executed
    // Then Result should contain max and min 38-digit integers
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT %1$s::%3$s(38,0), %2$s::%3$s(38,0)", MAX_38_DIGIT, MIN_38_DIGIT, NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection, sql, Arrays.asList(new BigDecimal(MAX_38_DIGIT), new BigDecimal(MIN_38_DIGIT)));
  }

  @Test
  public void shouldHandleNULLValuesFromLiteralsForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2),
    // 42.50::<type>(10,2)"
    // is executed
    // Then Result should contain [NULL, 42, NULL, 42.50]
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT NULL::%1$s(10,0), 42::%1$s(10,0), NULL::%1$s(10,2), 42.50::%1$s(10,2)",
            NUMBER_TYPE);
    assertSingleRowWithNulls(
        connection, sql, Arrays.asList(null, new BigDecimal("42"), null, new BigDecimal("42.50")));
  }

  @Test
  public void shouldDownloadLargeResultSetWithMultipleChunksFromGeneratorForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM
    // TABLE(GENERATOR(ROWCOUNT => 30000)) v" is executed
    // Then Column 1 should contain sequential integers from 0 to 29999
    // And Column 2 should contain sequential decimals starting from 0.12345
    Connection connection = getDefaultConnection();
    String sql =
        String.format(
            "SELECT seq8()::%1$s(38,0), (seq8() + 0.12345)::%1$s(20,5) "
                + "FROM TABLE(GENERATOR(ROWCOUNT => %2$d)) v ORDER BY 1",
            NUMBER_TYPE, LARGE_RESULT_SET_SIZE);

    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      int expectedRow = 0;
      while (resultSet.next()) {
        assertDecimalColumn(
            resultSet,
            1,
            BigDecimal.valueOf(expectedRow),
            "Column 1 mismatch for " + NUMBER_TYPE + ", row " + expectedRow);
        assertDecimalColumn(
            resultSet,
            2,
            BigDecimal.valueOf(expectedRow).add(DECIMAL_INCREMENT).setScale(5),
            "Column 2 mismatch for " + NUMBER_TYPE + ", row " + expectedRow);
        expectedRow++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, expectedRow, "Unexpected row count for " + NUMBER_TYPE);
    }
  }

  @Test
  public void shouldSelectNumbersFromTableWithMultipleScalesForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
    // And Row (123, 123.45, 123.456, 12345.67890) is inserted
    // And Row (-456, -67.89, -789.012, -98765.43210) is inserted
    // And Row (0, 0.00, 0.000, 0.00000) is inserted
    // And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain 4 rows with expected values
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection,
            "ud_number_",
            String.format(
                "c1 %1$s(10,0), c2 %1$s(10,2), c3 %1$s(15,3), c4 %1$s(20,5)", NUMBER_TYPE));
    execute(connection, "INSERT INTO " + tableName + " VALUES (123, 123.45, 123.456, 12345.67890)");
    execute(
        connection, "INSERT INTO " + tableName + " VALUES (-456, -67.89, -789.012, -98765.43210)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (0, 0.00, 0.000, 0.00000)");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES (999999, 999.99, 1000.500, 123456.78901)");

    assertRowsInOrder(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY c1, c2, c3, c4",
        Arrays.asList(
            Arrays.asList(
                new BigDecimal("-456"),
                new BigDecimal("-67.89"),
                new BigDecimal("-789.012"),
                new BigDecimal("-98765.43210")),
            Arrays.asList(
                new BigDecimal("0"),
                new BigDecimal("0.00"),
                new BigDecimal("0.000"),
                new BigDecimal("0.00000")),
            Arrays.asList(
                new BigDecimal("123"),
                new BigDecimal("123.45"),
                new BigDecimal("123.456"),
                new BigDecimal("12345.67890")),
            Arrays.asList(
                new BigDecimal("999999"),
                new BigDecimal("999.99"),
                new BigDecimal("1000.500"),
                new BigDecimal("123456.78901"))));
  }

  @Test
  public void shouldHandleHighPrecisionValuesFromTableForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
    // And Row (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78,
    // 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678) is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain [12345678901234567890123456789012345678,
    // 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
    // 1.2345678901234567890123456789012345678]
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection,
            "ud_number_",
            String.format(
                "c1 %1$s(38,0), c2 %1$s(38,2), c3 %1$s(38,10), c4 %1$s(38,37)", NUMBER_TYPE));
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (12345678901234567890123456789012345678, "
            + "123456789012345678901234567890123456.78, "
            + "1234567890123456789012345678.1234567890, "
            + "1.2345678901234567890123456789012345678)");

    assertRowsInOrder(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY c1",
        Arrays.asList(
            Arrays.asList(
                new BigDecimal("12345678901234567890123456789012345678"),
                new BigDecimal("123456789012345678901234567890123456.78"),
                new BigDecimal("1234567890123456789012345678.1234567890"),
                new BigDecimal("1.2345678901234567890123456789012345678"))));
  }

  @Test
  public void shouldHandleScaleAndPrecisionBoundariesFromTableForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(5,2), <type>(8,0)) exists
    // And Row (999.99, 99999999) is inserted
    // And Row (-999.99, -99999999) is inserted
    // And Row (123.45, 12345678) is inserted
    // And Row (0.01, 0) is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain 4 rows with expected boundary values
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection, "ud_number_", String.format("c1 %1$s(5,2), c2 %1$s(8,0)", NUMBER_TYPE));
    execute(connection, "INSERT INTO " + tableName + " VALUES (999.99, 99999999)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (-999.99, -99999999)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (123.45, 12345678)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (0.01, 0)");

    assertRowsInOrder(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY c1, c2",
        Arrays.asList(
            Arrays.asList(new BigDecimal("-999.99"), new BigDecimal("-99999999")),
            Arrays.asList(new BigDecimal("0.01"), new BigDecimal("0")),
            Arrays.asList(new BigDecimal("123.45"), new BigDecimal("12345678")),
            Arrays.asList(new BigDecimal("999.99"), new BigDecimal("99999999"))));
  }

  @Test
  public void shouldHandleHighPrecisionBoundariesFromTableForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(38,0), <type>(38,37)) exists
    // And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678)
    // is inserted
    // And Row (-99999999999999999999999999999999999999,
    // -1.2345678901234567890123456789012345678) is inserted
    // And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001)
    // is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain 3 rows with expected high precision boundary values
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection, "ud_number_", String.format("c1 %1$s(38,0), c2 %1$s(38,37)", NUMBER_TYPE));
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (99999999999999999999999999999999999999, "
            + "1.2345678901234567890123456789012345678)");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (-99999999999999999999999999999999999999, "
            + "-1.2345678901234567890123456789012345678)");
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (12345678901234567890123456789012345678, "
            + "0.0000000000000000000000000000000000001)");

    assertRowsInOrder(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY c1, c2",
        Arrays.asList(
            Arrays.asList(
                new BigDecimal("-99999999999999999999999999999999999999"),
                new BigDecimal("-1.2345678901234567890123456789012345678")),
            Arrays.asList(
                new BigDecimal("12345678901234567890123456789012345678"),
                new BigDecimal("0.0000000000000000000000000000000000001")),
            Arrays.asList(
                new BigDecimal("99999999999999999999999999999999999999"),
                new BigDecimal("1.2345678901234567890123456789012345678"))));
  }

  @Test
  public void shouldHandleNULLValuesFromTableWithMultipleScalesForNumberAndSynonyms()
      throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
    // And Row (NULL, NULL, NULL) is inserted
    // And Row (123, 123.45, 123.456) is inserted
    // And Row (NULL, NULL, NULL) is inserted
    // And Row (-456, -67.89, -789.012) is inserted
    // When Query "SELECT * FROM <table>" is executed
    // Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection,
            "ud_number_",
            String.format("c1 %1$s(10,0), c2 %1$s(10,2), c3 %1$s(15,3)", NUMBER_TYPE));
    execute(connection, "INSERT INTO " + tableName + " VALUES (NULL, NULL, NULL)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (123, 123.45, 123.456)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (NULL, NULL, NULL)");
    execute(connection, "INSERT INTO " + tableName + " VALUES (-456, -67.89, -789.012)");

    assertRowsInOrder(
        connection,
        "SELECT * FROM " + tableName + " ORDER BY c1 NULLS FIRST, c2 NULLS FIRST, c3 NULLS FIRST",
        Arrays.asList(
            Arrays.asList(null, null, null),
            Arrays.asList(null, null, null),
            Arrays.asList(
                new BigDecimal("-456"), new BigDecimal("-67.89"), new BigDecimal("-789.012")),
            Arrays.asList(
                new BigDecimal("123"), new BigDecimal("123.45"), new BigDecimal("123.456"))));
  }

  @Test
  public void shouldDownloadLargeResultSetFromTableForNumberAndSynonyms() throws Exception {
    // Given Snowflake client is logged in
    // And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from
    // 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column
    // When Query "SELECT * FROM <table>" is executed
    // Then Column 1 should contain sequential integers from 0 to 29999
    // And Column 2 should contain sequential decimals starting from 0.12345
    Connection connection = getDefaultConnection();
    String tableName =
        createTempTable(
            connection, "ud_number_", String.format("c1 %1$s(38,0), c2 %1$s(20,5)", NUMBER_TYPE));
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " SELECT seq8()::"
            + NUMBER_TYPE
            + "(38,0), (seq8() + 0.12345)::"
            + NUMBER_TYPE
            + "(20,5) FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + "))");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY c1")) {
      int expectedRow = 0;
      while (resultSet.next()) {
        assertDecimalColumn(
            resultSet,
            1,
            BigDecimal.valueOf(expectedRow),
            "Column 1 mismatch for " + NUMBER_TYPE + ", row " + expectedRow);
        assertDecimalColumn(
            resultSet,
            2,
            BigDecimal.valueOf(expectedRow).add(DECIMAL_INCREMENT).setScale(5),
            "Column 2 mismatch for " + NUMBER_TYPE + ", row " + expectedRow);
        expectedRow++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, expectedRow, "Unexpected row count for " + NUMBER_TYPE);
    }
  }

  private static void assertSingleRowWithNulls(
      Connection connection, String sql, List<BigDecimal> expectedValues) throws Exception {
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next(), "Expected one row for type: " + NUMBER_TYPE);
      for (int i = 0; i < expectedValues.size(); i++) {
        BigDecimal expected = expectedValues.get(i);
        if (expected == null) {
          assertNull(resultSet.getObject(i + 1), "Expected NULL for column " + (i + 1));
          assertTrue(resultSet.wasNull(), "Expected wasNull() for NULL column " + (i + 1));
          assertNull(
              resultSet.getBigDecimal(i + 1), "Expected getBigDecimal NULL for column " + (i + 1));
          assertTrue(
              resultSet.wasNull(),
              "Expected wasNull() after getBigDecimal NULL for column " + (i + 1));
        } else {
          assertDecimalColumn(
              resultSet,
              i + 1,
              expected,
              "Value mismatch for " + NUMBER_TYPE + ", column " + (i + 1));
        }
      }
      assertFalse(resultSet.next(), "Expected exactly one row for type: " + NUMBER_TYPE);
    }
  }

  private static void assertRowsInOrder(
      Connection connection, String sql, List<List<BigDecimal>> expectedRows) throws Exception {
    List<List<BigDecimal>> actualRows = new ArrayList<>();
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      ResultSetMetaData metaData = resultSet.getMetaData();
      int columnCount = metaData.getColumnCount();
      while (resultSet.next()) {
        List<BigDecimal> row = new ArrayList<>();
        for (int columnIndex = 1; columnIndex <= columnCount; columnIndex++) {
          row.add(resultSet.getBigDecimal(columnIndex));
        }
        actualRows.add(row);
      }
    }

    assertEquals(expectedRows.size(), actualRows.size(), "Unexpected row count for " + NUMBER_TYPE);
    for (int rowIndex = 0; rowIndex < expectedRows.size(); rowIndex++) {
      assertEquals(
          expectedRows.get(rowIndex),
          actualRows.get(rowIndex),
          "Row mismatch for " + NUMBER_TYPE + " at index " + rowIndex);
    }
  }

  private static void assertDecimalColumn(
      ResultSet resultSet, int columnIndex, BigDecimal expected, String message) throws Exception {
    BigDecimal actual = resultSet.getBigDecimal(columnIndex);
    assertEquals(expected, actual, message + " (getBigDecimal)");
    assertFalse(resultSet.wasNull(), message + " (getBigDecimal should not be NULL)");

    Object objectValue = resultSet.getObject(columnIndex);
    // This intentionally validates current JDBC behavior:
    // scale=0 values in long range come back as Long, otherwise as BigDecimal.
    Class<?> expectedObjectType = getExpectedObjectType(expected);
    assertInstanceOf(
        expectedObjectType,
        objectValue,
        message + " (getObject expected type " + expectedObjectType.getSimpleName() + ")");
    BigDecimal objectAsDecimal = toBigDecimal(objectValue, message + " (getObject)");
    assertEquals(0, expected.compareTo(objectAsDecimal), message + " (getObject numeric value)");
    assertFalse(resultSet.wasNull(), message + " (getObject should not be NULL)");

    String stringValue = resultSet.getString(columnIndex);
    assertNotNull(stringValue, message + " (getString should not be NULL)");
    assertFalse(stringValue.isEmpty(), message + " (getString should not be empty)");
    assertDoesNotThrow(
        () -> parseLocaleDecimal(stringValue, message),
        message + " (getString should be parseable numeric)");
    assertFalse(resultSet.wasNull(), message + " (getString should not be NULL)");
  }

  // It's crucial to have proper local in big decimal comparison
  private static BigDecimal parseLocaleDecimal(String value, String message) {
    DecimalFormat df = (DecimalFormat) DecimalFormat.getInstance();
    df.setParseBigDecimal(true);
    try {
      return (BigDecimal) df.parse(value);
    } catch (ParseException e) {
      throw new AssertionError(
          message + " (getString value '" + value + "' is not a parseable number)", e);
    }
  }

  private static BigDecimal toBigDecimal(Object value, String message) {
    assertInstanceOf(Number.class, value, message + " (expected numeric type)");
    if (value instanceof BigDecimal) {
      return (BigDecimal) value;
    }
    return new BigDecimal(value.toString());
  }

  private static Class<?> getExpectedObjectType(BigDecimal expected) {
    if (expected.scale() == 0
        && expected.compareTo(LONG_MIN_DECIMAL) >= 0
        && expected.compareTo(LONG_MAX_DECIMAL) <= 0) {
      return Long.class;
    }
    return BigDecimal.class;
  }
}
