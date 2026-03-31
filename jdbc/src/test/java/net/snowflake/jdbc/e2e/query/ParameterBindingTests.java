package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.List;
import java.util.StringJoiner;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class ParameterBindingTests extends SnowflakeIntegrationTestBase {
  @Test
  public void shouldBindBasicTypesWithPositionalParameters() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?, ?, ?, ?, ?" is executed with positional parameters
    // [42, 3.14, "hello", True, None]
    String sql = "SELECT ?, ?, ?, ?, ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 42);
      preparedStatement.setDouble(2, 3.14d);
      preparedStatement.setString(3, "hello");
      preparedStatement.setBoolean(4, true);
      preparedStatement.setNull(5, Types.NULL);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain values matching the bound parameters
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(42, resultSet.getInt(1), "Unexpected first bound value");
        assertFalse(resultSet.wasNull(), "First bound value should not be NULL");
        assertEquals(3.14d, resultSet.getDouble(2), 0.0001d, "Unexpected second bound value");
        assertFalse(resultSet.wasNull(), "Second bound value should not be NULL");
        assertEquals("hello", resultSet.getString(3), "Unexpected third bound value");
        assertFalse(resultSet.wasNull(), "Third bound value should not be NULL");
        assertTrue(resultSet.getBoolean(4), "Unexpected fourth bound value");
        assertFalse(resultSet.wasNull(), "Fourth bound value should not be NULL");
        assertNull(resultSet.getObject(5), "Expected NULL fifth bound value");
        assertTrue(resultSet.wasNull(), "Fifth bound value should set wasNull()");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldBindPositionalParametersWithNumericPlaceholders() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT :1, :2, :3" is executed with positional parameters
    // [100, "test", True]
    String sql = "SELECT :1, :2, :3";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 100);
      preparedStatement.setString(2, "test");
      preparedStatement.setBoolean(3, true);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain values in order [100, "test", True]
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(100, resultSet.getInt(1), "Unexpected first numeric placeholder value");
        assertFalse(resultSet.wasNull(), "First numeric placeholder value should not be NULL");
        assertEquals("test", resultSet.getString(2), "Unexpected second numeric placeholder value");
        assertFalse(resultSet.wasNull(), "Second numeric placeholder value should not be NULL");
        assertTrue(resultSet.getBoolean(3), "Unexpected third numeric placeholder value");
        assertFalse(resultSet.wasNull(), "Third numeric placeholder value should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldInsertSingleRowWithParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR, active BOOLEAN) exists
    String tableName =
        createTempTable(
            connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR, active BOOLEAN");

    // When Row with values [1, "Alice", True] is inserted using parameter binding
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      preparedStatement.setInt(1, 1);
      preparedStatement.setString(2, "Alice");
      preparedStatement.setBoolean(3, true);
      preparedStatement.executeUpdate();
    }

    // And Query "SELECT * FROM table" is executed
    String selectSql = "SELECT * FROM " + tableName;
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      // Then Result should contain the inserted row [1, "Alice", True]
      assertTrue(resultSet.next(), "Expected inserted row");
      assertEquals(1, resultSet.getInt(1), "Unexpected inserted id");
      assertFalse(resultSet.wasNull(), "Inserted id should not be NULL");
      assertEquals("Alice", resultSet.getString(2), "Unexpected inserted name");
      assertFalse(resultSet.wasNull(), "Inserted name should not be NULL");
      assertTrue(resultSet.getBoolean(3), "Unexpected inserted active flag");
      assertFalse(resultSet.wasNull(), "Inserted active flag should not be NULL");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldInsertMultipleRowsSequentiallyWithParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName =
        createTempTable(connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR");

    // When Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"] are inserted sequentially using
    // parameter binding
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      preparedStatement.setInt(1, 1);
      preparedStatement.setString(2, "Alice");
      preparedStatement.executeUpdate();

      preparedStatement.setInt(1, 2);
      preparedStatement.setString(2, "Bob");
      preparedStatement.executeUpdate();

      preparedStatement.setInt(1, 3);
      preparedStatement.setString(2, "Charlie");
      preparedStatement.executeUpdate();
    }

    // And Query "SELECT * FROM table ORDER BY id" is executed
    String selectSql = "SELECT * FROM " + tableName + " ORDER BY id";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      // Then Result should contain 3 rows with correct values
      assertTrue(resultSet.next(), "Expected first row");
      assertEquals(1, resultSet.getInt(1), "Unexpected first row id");
      assertEquals("Alice", resultSet.getString(2), "Unexpected first row name");

      assertTrue(resultSet.next(), "Expected second row");
      assertEquals(2, resultSet.getInt(1), "Unexpected second row id");
      assertEquals("Bob", resultSet.getString(2), "Unexpected second row name");

      assertTrue(resultSet.next(), "Expected third row");
      assertEquals(3, resultSet.getInt(1), "Unexpected third row id");
      assertEquals("Charlie", resultSet.getString(2), "Unexpected third row name");

      assertFalse(resultSet.next(), "Expected exactly three rows");
    }
  }

  @Test
  public void shouldUpdateRowWithParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName =
        createTempTable(connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR");

    // And Row [1, "Alice"] is inserted
    execute(connection, "INSERT INTO " + tableName + " VALUES (1, 'Alice')");

    // When Query "UPDATE table SET name = ? WHERE id = ?" is executed with parameters
    // ["Alice Updated", 1]
    String updateSql = "UPDATE " + tableName + " SET name = ? WHERE id = ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(updateSql)) {
      preparedStatement.setString(1, "Alice Updated");
      preparedStatement.setInt(2, 1);
      preparedStatement.executeUpdate();
    }

    // And Query "SELECT * FROM table" is executed
    String selectSql = "SELECT * FROM " + tableName;
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      // Then Result should contain [1, "Alice Updated"]
      assertTrue(resultSet.next(), "Expected updated row");
      assertEquals(1, resultSet.getInt(1), "Unexpected updated row id");
      assertFalse(resultSet.wasNull(), "Updated row id should not be NULL");
      assertEquals("Alice Updated", resultSet.getString(2), "Unexpected updated row name");
      assertFalse(resultSet.wasNull(), "Updated row name should not be NULL");
      assertFalse(resultSet.next(), "Expected exactly one row");
    }
  }

  @Test
  public void shouldDeleteRowWithParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName =
        createTempTable(connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR");

    // And Rows [1, "Alice"] and [2, "Bob"] are inserted
    execute(connection, "INSERT INTO " + tableName + " VALUES (1, 'Alice'), (2, 'Bob')");

    // When Query "DELETE FROM table WHERE id = ?" is executed with parameter [1]
    String deleteSql = "DELETE FROM " + tableName + " WHERE id = ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(deleteSql)) {
      preparedStatement.setInt(1, 1);
      preparedStatement.executeUpdate();
    }

    // And Query "SELECT * FROM table" is executed
    String selectSql = "SELECT * FROM " + tableName;
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      // Then Result should contain only [2, "Bob"]
      assertTrue(resultSet.next(), "Expected remaining row");
      assertEquals(2, resultSet.getInt(1), "Unexpected remaining row id");
      assertFalse(resultSet.wasNull(), "Remaining row id should not be NULL");
      assertEquals("Bob", resultSet.getString(2), "Unexpected remaining row name");
      assertFalse(resultSet.wasNull(), "Remaining row name should not be NULL");
      assertFalse(resultSet.next(), "Expected exactly one remaining row");
    }
  }

  @Test
  public void shouldSelectWithWhereClauseParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR, age NUMBER) exists
    String tableName =
        createTempTable(connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR, age NUMBER");

    // And Rows [1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35] are inserted
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35)");

    // When Query "SELECT * FROM table WHERE age > ?" is executed with parameter [28]
    String sql = "SELECT * FROM " + tableName + " WHERE age > ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 28);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain rows for "Alice" and "Charlie"
        List<String> names = new ArrayList<>();
        while (resultSet.next()) {
          names.add(resultSet.getString(2));
        }
        assertEquals(2, names.size(), "Expected two matching rows");
        assertTrue(names.contains("Alice"), "Expected Alice in filtered results");
        assertTrue(names.contains("Charlie"), "Expected Charlie in filtered results");
      }
    }
  }

  @Test
  public void shouldHandleNullValuesInParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?, ?, ?" is executed with parameters [None, 42, None]
    String sql = "SELECT ?, ?, ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setNull(1, Types.NULL);
      preparedStatement.setInt(2, 42);
      preparedStatement.setNull(3, Types.NULL);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain [NULL, 42, NULL]
        assertTrue(resultSet.next(), "Expected one row");
        assertNull(resultSet.getObject(1), "Expected NULL in first column");
        assertTrue(resultSet.wasNull(), "First column should set wasNull()");
        assertEquals(42, resultSet.getInt(2), "Unexpected second column value");
        assertFalse(resultSet.wasNull(), "Second column should not be NULL");
        assertNull(resultSet.getObject(3), "Expected NULL in third column");
        assertTrue(resultSet.wasNull(), "Third column should set wasNull()");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldHandleSpecialCharactersInStringBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::VARCHAR" is executed with parameter containing special characters
    String sql = "SELECT ?::VARCHAR";
    String[] specialStrings = {
      "'; DROP TABLE test; --",
      "<script>alert('xss')</script>",
      "Line1\nLine2\nLine3",
      "Tab\t\tSeparated\t\tValues",
      "Quote'Within\"String",
      "\\n\\t\\r\\\\"
    };
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      // Then Result should contain the exact special character string
      for (String specialString : specialStrings) {
        preparedStatement.setString(1, specialString);
        try (ResultSet resultSet = preparedStatement.executeQuery()) {
          assertTrue(resultSet.next(), "Expected one row for special string");
          assertEquals(
              specialString, resultSet.getString(1), "Unexpected special character round-trip");
          assertFalse(resultSet.wasNull(), "Special character result should not be NULL");
          assertFalse(resultSet.next(), "Expected exactly one row for special string");
        }
      }
    }
  }

  @Test
  public void shouldHandleUnicodeCharactersInParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::VARCHAR, ?::VARCHAR" is executed with parameters ["日本語", "⛄"]
    String sql = "SELECT ?::VARCHAR, ?::VARCHAR";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setString(1, "日本語");
      preparedStatement.setString(2, "⛄");
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain Unicode strings ["日本語", "⛄"]
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals("日本語", resultSet.getString(1), "Unexpected first Unicode string");
        assertFalse(resultSet.wasNull(), "First Unicode string should not be NULL");
        assertEquals("⛄", resultSet.getString(2), "Unexpected second Unicode string");
        assertFalse(resultSet.wasNull(), "Second Unicode string should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldBindZeroValues() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?, ?::FLOAT, ?::VARCHAR" is executed with parameters [0, 0.0, ""]
    String sql = "SELECT ?, ?::FLOAT, ?::VARCHAR";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 0);
      preparedStatement.setDouble(2, 0.0d);
      preparedStatement.setString(3, "");
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain zero and empty values [0, 0.0, ""]
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(0, resultSet.getInt(1), "Unexpected zero integer value");
        assertFalse(resultSet.wasNull(), "Zero integer value should not be NULL");
        assertEquals(0.0d, resultSet.getDouble(2), 0.0001d, "Unexpected zero float value");
        assertFalse(resultSet.wasNull(), "Zero float value should not be NULL");
        assertEquals("", resultSet.getString(3), "Unexpected empty string value");
        assertFalse(resultSet.wasNull(), "Empty string value should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldHandleMixedTypeCastingWithParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN" is executed with parameters
    // [42, "hello", True]
    String sql = "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 42);
      preparedStatement.setString(2, "hello");
      preparedStatement.setBoolean(3, true);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should match the type-casted parameters [42, "hello", True]
        assertTrue(resultSet.next(), "Expected one row");
        assertEquals(42, resultSet.getInt(1), "Unexpected casted number value");
        assertFalse(resultSet.wasNull(), "Casted number value should not be NULL");
        assertEquals("hello", resultSet.getString(2), "Unexpected casted string value");
        assertFalse(resultSet.wasNull(), "Casted string value should not be NULL");
        assertTrue(resultSet.getBoolean(3), "Unexpected casted boolean value");
        assertFalse(resultSet.wasNull(), "Casted boolean value should not be NULL");
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldBindManyParameters() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query with 20 positional parameters is executed with values [0..19]
    StringJoiner placeholderJoiner = new StringJoiner(", ");
    for (int i = 0; i < 20; i++) {
      placeholderJoiner.add("?");
    }
    String sql = "SELECT " + placeholderJoiner;
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      for (int i = 0; i < 20; i++) {
        preparedStatement.setInt(i + 1, i);
      }
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain all 20 values in order
        assertTrue(resultSet.next(), "Expected one row");
        for (int i = 0; i < 20; i++) {
          assertEquals(i, resultSet.getInt(i + 1), "Unexpected value at parameter position " + i);
          assertFalse(resultSet.wasNull(), "Parameter value should not be NULL at position " + i);
        }
        assertFalse(resultSet.next(), "Expected exactly one row");
      }
    }
  }

  @Test
  public void shouldBindParametersWithOrClauseForMultipleValueMatching() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // And A temporary table with columns (id NUMBER, name VARCHAR) exists
    String tableName =
        createTempTable(connection, "ud_parameter_binding_", "id NUMBER, name VARCHAR");

    // And Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"], [4, "David"], [5, "Eve"] are inserted
    execute(
        connection,
        "INSERT INTO "
            + tableName
            + " VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie'), (4, 'David'), (5, 'Eve')");

    // When Query "SELECT FROM {table_name} WHERE id = ? OR id = ? OR id = ? ORDER BY id" is
    // executed with parameters [1, 3, 5]
    String sql = "SELECT * FROM " + tableName + " WHERE id = ? OR id = ? OR id = ? ORDER BY id";
    try (PreparedStatement preparedStatement = connection.prepareStatement(sql)) {
      preparedStatement.setInt(1, 1);
      preparedStatement.setInt(2, 3);
      preparedStatement.setInt(3, 5);
      try (ResultSet resultSet = preparedStatement.executeQuery()) {
        // Then Result should contain [("Alice"), ("Charlie"), ("Eve")]
        assertTrue(resultSet.next(), "Expected first matching row");
        assertEquals("Alice", resultSet.getString(2), "Unexpected first matching name");
        assertFalse(resultSet.wasNull(), "First matching name should not be NULL");

        assertTrue(resultSet.next(), "Expected second matching row");
        assertEquals("Charlie", resultSet.getString(2), "Unexpected second matching name");
        assertFalse(resultSet.wasNull(), "Second matching name should not be NULL");

        assertTrue(resultSet.next(), "Expected third matching row");
        assertEquals("Eve", resultSet.getString(2), "Unexpected third matching name");
        assertFalse(resultSet.wasNull(), "Third matching name should not be NULL");

        assertFalse(resultSet.next(), "Expected exactly three matching rows");
      }
    }
  }

  @Test
  public void shouldRaiseErrorWhenPlaceholderCountMismatchesArgumentCount() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query with 2 placeholders is executed with 3 arguments
    String extraParameterSql = "SELECT ?, ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(extraParameterSql)) {
      preparedStatement.setInt(1, 1);
      preparedStatement.setInt(2, 2);
      preparedStatement.setInt(3, 3);
      boolean hasResultSet = preparedStatement.execute();

      // Then Query should successfully execute
      assertTrue(hasResultSet, "Expected query with extra arguments to execute successfully");
    }

    // When Query with 3 placeholders is executed with 1 argument
    String tooFewArgumentsSql = "SELECT ?, ?, ?";
    try (PreparedStatement preparedStatement = connection.prepareStatement(tooFewArgumentsSql)) {
      preparedStatement.setInt(1, 1);
      SQLException tooFewArgumentsException =
          assertThrows(SQLException.class, preparedStatement::executeQuery);

      // Then Error should be raised for too few arguments
      assertTrue(
          tooFewArgumentsException.getMessage() != null
              && !tooFewArgumentsException.getMessage().isEmpty(),
          "Expected message for too few arguments");
    }
  }
}
