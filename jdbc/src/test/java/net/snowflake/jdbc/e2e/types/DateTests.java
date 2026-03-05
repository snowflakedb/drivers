package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.Date;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.time.LocalDate;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class DateTests extends SnowflakeIntegrationTestBase {
  private static final int LARGE_RESULT_SET_SIZE = 50_000;

  @Test
  public void shouldCastDateValuesToAppropriateType() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    // Then All values should be returned as DATE type
    // And No precision loss should occur
    Connection connection = getDefaultConnection();
    String sql = "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
      assertDateGetters(resultSet, 2, LocalDate.of(1970, 1, 1));
      assertDateGetters(resultSet, 3, LocalDate.of(1999, 12, 31));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectDateLiterals() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
    Connection connection = getDefaultConnection();
    String sql = "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
      assertDateGetters(resultSet, 2, LocalDate.of(1970, 1, 1));
      assertDateGetters(resultSet, 3, LocalDate.of(1999, 12, 31));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectEpochAndPreEpochDates() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
    // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
    Connection connection = getDefaultConnection();
    String sql = "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1970, 1, 1));
      assertDateGetters(resultSet, 2, LocalDate.of(1969, 12, 31));
      assertDateGetters(resultSet, 3, LocalDate.of(1900, 1, 1));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectHistoricalDates() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE" is executed
    // Then Result should contain dates [0001-01-01, 1582-10-15]
    Connection connection = getDefaultConnection();
    String sql = "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1, 1, 1));
      assertDateGetters(resultSet, 2, LocalDate.of(1582, 10, 15));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldHandleNULLValuesForDate() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
    // Then Result should contain [NULL, 2024-01-15, NULL]
    Connection connection = getDefaultConnection();
    String sql = "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      assertTrue(resultSet.next());

      assertNull(resultSet.getDate(1));
      assertTrue(resultSet.wasNull());
      assertNull(resultSet.getObject(1));
      assertTrue(resultSet.wasNull());

      assertDateGetters(resultSet, 2, LocalDate.of(2024, 1, 15));

      assertNull(resultSet.getDate(3));
      assertTrue(resultSet.wasNull());

      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldDownloadLargeResultSetForDate() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT DATEADD(day, seq4(), '1970-01-01'::DATE) as d FROM
    // TABLE(GENERATOR(ROWCOUNT
    // => 50000)) v ORDER BY d" is executed
    // Then Result should contain 50000 rows with sequential dates starting from 1970-01-01
    Connection connection = getDefaultConnection();
    String sql =
        "SELECT DATEADD(day, seq4(), '1970-01-01'::DATE) as d"
            + " FROM TABLE(GENERATOR(ROWCOUNT => "
            + LARGE_RESULT_SET_SIZE
            + ")) v ORDER BY d";
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(sql)) {
      int rowCount = 0;
      LocalDate expected = LocalDate.of(1970, 1, 1);
      while (resultSet.next()) {
        assertEquals(
            Date.valueOf(expected), resultSet.getDate(1), "Date mismatch at row " + rowCount);
        assertFalse(resultSet.wasNull());
        expected = expected.plusDays(1);
        rowCount++;
      }
      assertEquals(LARGE_RESULT_SET_SIZE, rowCount);
    }
  }

  @Test
  public void shouldSelectDatesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_date_", "col DATE");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES ('2024-01-15'), ('1970-01-01'), ('1999-12-31')");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1970, 1, 1));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1999, 12, 31));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectDatesWithNullFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain [1999-12-31, 2024-01-15, NULL]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_date_", "col DATE");
    execute(
        connection, "INSERT INTO " + tableName + " VALUES ('2024-01-15'), (NULL), ('1999-12-31')");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1999, 12, 31));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
      assertTrue(resultSet.next());
      assertNull(resultSet.getDate(1));
      assertTrue(resultSet.wasNull());
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectHistoricalDatesFromTable() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15']
    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_date_hist_", "col DATE");
    execute(
        connection,
        "INSERT INTO " + tableName + " VALUES ('0001-01-01'), ('0100-03-01'), ('1582-10-15')");

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1, 1, 1));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(100, 3, 1));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1582, 10, 15));
      assertFalse(resultSet.next());
    }
  }

  @Test
  public void shouldSelectDateUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed with bound date values
    // [2024-01-15, 1970-01-01, 1999-12-31]
    // Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
    // When Query "SELECT ?::DATE" is executed with bound NULL value
    // Then Result should contain [NULL]
    Connection connection = getDefaultConnection();

    try (PreparedStatement ps = connection.prepareStatement("SELECT ?::DATE, ?::DATE, ?::DATE")) {
      ps.setDate(1, Date.valueOf("2024-01-15"));
      ps.setDate(2, Date.valueOf("1970-01-01"));
      ps.setDate(3, Date.valueOf("1999-12-31"));
      try (ResultSet resultSet = ps.executeQuery()) {
        assertTrue(resultSet.next());
        assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
        assertDateGetters(resultSet, 2, LocalDate.of(1970, 1, 1));
        assertDateGetters(resultSet, 3, LocalDate.of(1999, 12, 31));
        assertFalse(resultSet.next());
      }
    }

    try (PreparedStatement ps = connection.prepareStatement("SELECT ?::DATE")) {
      ps.setNull(1, java.sql.Types.DATE);
      try (ResultSet resultSet = ps.executeQuery()) {
        assertTrue(resultSet.next());
        assertNull(resultSet.getDate(1));
        assertTrue(resultSet.wasNull());
        assertFalse(resultSet.next());
      }
    }
  }

  @Test
  public void shouldInsertDateUsingParameterBinding() throws Exception {
    // Given Snowflake client is logged in
    // And Table with DATE column exists
    // When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using setDate binding
    // And Query "SELECT * FROM <table> ORDER BY col" is executed
    // Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_date_bind_", "col DATE");

    try (PreparedStatement ps =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?)")) {
      ps.setDate(1, Date.valueOf("2024-01-15"));
      ps.execute();
      ps.setDate(1, Date.valueOf("1970-01-01"));
      ps.execute();
      ps.setDate(1, Date.valueOf("1999-12-31"));
      ps.execute();
    }

    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery("SELECT * FROM " + tableName + " ORDER BY col")) {
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1970, 1, 1));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(1999, 12, 31));
      assertTrue(resultSet.next());
      assertDateGetters(resultSet, 1, LocalDate.of(2024, 1, 15));
      assertFalse(resultSet.next());
    }
  }

  private static void assertDateGetters(ResultSet rs, int col, LocalDate expected)
      throws Exception {
    Date expectedDate = Date.valueOf(expected);
    int expectedEpochDays = (int) expected.toEpochDay();

    assertEquals(expectedDate, rs.getDate(col), "getDate mismatch");
    assertFalse(rs.wasNull());

    assertEquals(expected.toString(), rs.getString(col), "getString mismatch");
    assertFalse(rs.wasNull());

    Object obj = rs.getObject(col);
    assertEquals(expectedDate, obj, "getObject mismatch");
    assertFalse(rs.wasNull());

    assertEquals(expectedEpochDays, rs.getInt(col), "getInt (epoch days) mismatch");
    assertFalse(rs.wasNull());

    assertEquals((long) expectedEpochDays, rs.getLong(col), "getLong (epoch days) mismatch");
    assertFalse(rs.wasNull());
  }
}
