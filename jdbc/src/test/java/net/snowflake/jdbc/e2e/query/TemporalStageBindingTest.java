package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.Date;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.TimeZone;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.parallel.ResourceLock;
import org.junit.jupiter.api.parallel.Resources;

@ResourceLock(Resources.TIME_ZONE)
public class TemporalStageBindingTest extends SnowflakeIntegrationTestBase {

  private static final int DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD = 65280;

  @AfterEach
  public void restoreSessionParameters() throws Exception {
    setStageArrayBindingThreshold(getDefaultConnection(), DEFAULT_STAGE_ARRAY_BINDING_THRESHOLD);
    execute(getDefaultConnection(), "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = TIMESTAMP_LTZ");
    execute(
        getDefaultConnection(), "ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = TIMESTAMP_LTZ");
  }

  @Test
  public void shouldPreserveTemporalValuesBetweenInlineAndStageBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    String[] temporalValues = {
      "0001-01-01 00:00:00",
      "0100-03-01 00:00:00",
      "0400-02-29 00:00:00",
      "0400-03-01 00:00:00",
      "1400-03-01 00:00:00",
      "1582-10-15 00:00:00",
      "1899-12-31 23:59:59.900",
      "1900-02-28 00:00:00",
      "1900-03-01 00:00:00",
      "1969-12-31 00:00:00",
      "1969-12-31 23:59:59",
      "1970-01-01 00:00:00",
      "1997-04-27 00:00:00",
      "2000-02-28 00:00:00",
      "2000-02-29 00:00:00",
      "2000-03-01 00:00:00",
      "2024-02-29 00:00:00",
      "3000-01-01 00:00:00"
    };
    Date[] dates = new Date[temporalValues.length + 1];
    Timestamp[] timestamps = new Timestamp[temporalValues.length + 1];
    for (int index = 0; index < temporalValues.length; index++) {
      dates[index] = Date.valueOf(temporalValues[index].substring(0, 10));
      timestamps[index] = Timestamp.valueOf(temporalValues[index]);
    }
    dates[dates.length - 1] = Date.valueOf("1970-01-01");
    timestamps[timestamps.length - 1] = null;

    // When DATE, TIMESTAMP_LTZ, and TIMESTAMP_NTZ values spanning the Unix epoch are inserted once
    // inline and once through stage binding
    for (String timestampType : new String[] {"TIMESTAMP_LTZ", "TIMESTAMP_NTZ"}) {
      String tableName =
          createTempTable(
              connection,
              "ud_temporal_stage_binding_",
              "bind_path VARCHAR, id INTEGER, d DATE, ts " + timestampType);
      execute(connection, "ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = " + timestampType);
      execute(connection, "ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = " + timestampType);
      insertTemporalRows(connection, tableName, "inline", 0, dates, timestamps);
      insertTemporalRows(connection, tableName, "stage", 1, dates, timestamps);

      // Then every stage-bound temporal value should equal its inline-bound value
      assertTemporalRowsMatch(connection, tableName, timestampType, dates, timestamps);
    }
  }

  @Test
  public void shouldPreserveTimeValuesBetweenInlineAndStageBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    Time[] times = {
      new Time(-100L),
      new Time(-1L),
      Time.valueOf("00:00:00"),
      new Time(1L),
      Time.valueOf("00:00:01"),
      new Time(55_555L),
      new Time(123_456L),
      Time.valueOf("13:00:00"),
      Time.valueOf("14:00:00"),
      new Time(Integer.MAX_VALUE),
      new Time(Time.valueOf("23:59:59").getTime() + 999L),
      null
    };
    String tableName =
        createTempTable(
            connection, "ud_time_stage_binding_", "bind_path VARCHAR, id INTEGER, t TIME");

    // When TIME values spanning the whole day are inserted once inline and once through stage
    // binding
    insertTimeRows(connection, tableName, "inline", 0, times);
    insertTimeRows(connection, tableName, "stage", 1, times);

    // Then every stage-bound TIME value should equal its inline-bound value
    Time[] inlineTimes = new Time[times.length];
    try (Statement statement = connection.createStatement();
        ResultSet resultSet =
            statement.executeQuery(
                "SELECT bind_path, id, t FROM " + tableName + " ORDER BY bind_path, id")) {
      for (String bindPath : new String[] {"inline", "stage"}) {
        for (int id = 0; id < times.length; id++) {
          assertTrue(resultSet.next(), "Expected " + bindPath + " TIME row " + id);
          assertEquals(bindPath, resultSet.getString(1), "Unexpected TIME bind path");
          assertFalse(resultSet.wasNull(), "TIME bind path should not be NULL");
          assertEquals(id, resultSet.getInt(2), "Unexpected TIME row id");
          assertFalse(resultSet.wasNull(), "TIME row id should not be NULL");
          Time actualTime = resultSet.getTime(3);
          boolean wasNull = resultSet.wasNull();
          if ("inline".equals(bindPath)) {
            inlineTimes[id] = actualTime;
            assertEquals(times[id] == null, wasNull, "Unexpected inline TIME nullability");
          } else {
            assertEquals(times[id] == null, wasNull, "Unexpected stage TIME nullability");
            if (times[id] == null) {
              assertNull(actualTime, "Stage TIME should be NULL for id " + id);
            } else {
              assertEquals(
                  inlineTimes[id].getTime(),
                  actualTime.getTime(),
                  "Stage TIME should match inline TIME for id " + id);
            }
          }
        }
      }
      assertFalse(resultSet.next(), "Expected exactly " + (times.length * 2) + " TIME rows");
    }
  }

  @Test
  public void shouldPreserveExplicitNtzAndLtzSetObjectValuesBetweenBindPaths() throws Exception {
    // Given Snowflake client is logged in
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET TIMEZONE = 'UTC'");
      String tableName =
          createTempTable(
              connection,
              "ud_temporal_stage_binding_",
              "bind_path VARCHAR, id INTEGER, ntz TIMESTAMP_NTZ, ltz TIMESTAMP_LTZ");
      Timestamp[] values = {
        Timestamp.valueOf("2014-01-01 16:00:00"), Timestamp.valueOf("1945-11-12 05:25:00")
      };

      // When explicit TIMESTAMP_NTZ and TIMESTAMP_LTZ values are inserted inline and through stage
      // binding
      insertExplicitTimestampRows(connection, tableName, "inline", 0, values);
      insertExplicitTimestampRows(connection, tableName, "stage", 1, values);

      // Then every stage-bound value should equal its inline-bound value
      Calendar utc = Calendar.getInstance(TimeZone.getTimeZone("UTC"));
      Timestamp[][] inline = new Timestamp[values.length][2];
      try (Statement statement = connection.createStatement();
          ResultSet resultSet =
              statement.executeQuery(
                  "SELECT bind_path, id, ntz, ltz FROM " + tableName + " ORDER BY bind_path, id")) {
        for (String bindPath : new String[] {"inline", "stage"}) {
          for (int id = 0; id < values.length; id++) {
            assertTrue(resultSet.next(), "Expected " + bindPath + " explicit timestamp row " + id);
            assertEquals(bindPath, resultSet.getString(1), "Unexpected bind path");
            assertFalse(resultSet.wasNull(), "Bind path should not be NULL");
            assertEquals(id, resultSet.getInt(2), "Unexpected row id");
            assertFalse(resultSet.wasNull(), "Row id should not be NULL");
            Timestamp ntz = resultSet.getTimestamp(3, utc);
            assertFalse(resultSet.wasNull(), "TIMESTAMP_NTZ should not be NULL");
            Timestamp ltz = resultSet.getTimestamp(4, utc);
            assertFalse(resultSet.wasNull(), "TIMESTAMP_LTZ should not be NULL");
            if ("inline".equals(bindPath)) {
              inline[id][0] = ntz;
              inline[id][1] = ltz;
            } else {
              assertEquals(inline[id][0], ntz, "Stage NTZ should match inline for id " + id);
              assertEquals(inline[id][1], ltz, "Stage LTZ should match inline for id " + id);
            }
          }
        }
        assertFalse(resultSet.next(), "Expected exactly " + values.length * 2 + " rows");
      }
    }
  }

  @Test
  public void shouldPreserveWallClockTimesViaStageBinding() throws Exception {
    // Given Snowflake client is logged in with wall-clock TIME binding enabled
    TimeZone originalTimeZone = TimeZone.getDefault();
    TimeZone.setDefault(TimeZone.getTimeZone("Pacific/Honolulu"));
    try (Connection connection =
        openConnection("CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME", Boolean.TRUE.toString())) {
      execute(connection, "ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'");
      setStageArrayBindingThreshold(connection, 1);
      String tableName =
          createTempTable(connection, "ud_time_stage_binding_", "id INTEGER, value TIME");
      Time[] times = {
        Time.valueOf("00:00:00"),
        Time.valueOf("11:59:59"),
        Time.valueOf("12:00:00"),
        Time.valueOf("12:34:56"),
        Time.valueOf("13:01:01"),
        Time.valueOf("15:30:00"),
        Time.valueOf("23:59:59")
      };

      // When TIME values are inserted through stage binding with different JVM and session zones
      try (PreparedStatement preparedStatement =
          connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?, ?)")) {
        for (int id = 0; id < times.length; id++) {
          preparedStatement.setInt(1, id);
          preparedStatement.setTime(2, times[id]);
          preparedStatement.addBatch();
        }
        preparedStatement.executeBatch();
      }

      // Then every value should retain its original wall-clock fields
      try (Statement statement = connection.createStatement();
          ResultSet resultSet =
              statement.executeQuery(
                  "SELECT id, value, TO_VARCHAR(value, 'HH24:MI:SS') FROM "
                      + tableName
                      + " ORDER BY id")) {
        for (int id = 0; id < times.length; id++) {
          assertTrue(resultSet.next(), "Expected wall-clock TIME row " + id);
          assertEquals(id, resultSet.getInt(1), "Unexpected wall-clock TIME row id");
          assertFalse(resultSet.wasNull(), "Wall-clock TIME row id should not be NULL");
          assertEquals(
              times[id].toLocalTime(),
              resultSet.getTime(2).toLocalTime(),
              "Unexpected wall-clock TIME value for id " + id);
          assertFalse(resultSet.wasNull(), "Wall-clock TIME should not be NULL");
          assertEquals(
              times[id].toString(),
              resultSet.getString(3),
              "Unexpected rendered wall-clock TIME for id " + id);
          assertFalse(resultSet.wasNull(), "Rendered wall-clock TIME should not be NULL");
        }
        assertFalse(resultSet.next(), "Expected exactly " + times.length + " wall-clock rows");
      }
    } finally {
      TimeZone.setDefault(originalTimeZone);
    }
  }

  @Test
  public void shouldPreserveLtzMappedDstValuesBetweenBindPaths() throws Exception {
    // Given Snowflake client is logged in with TIMESTAMP_LTZ client mapping
    TimeZone originalTimeZone = TimeZone.getDefault();
    TimeZone australiaTimeZone = TimeZone.getTimeZone("Australia/Sydney");
    TimeZone.setDefault(australiaTimeZone);
    try (Connection connection = openConnection()) {
      execute(connection, "ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = TIMESTAMP_LTZ");
      execute(connection, "ALTER SESSION SET TIMEZONE = 'UTC'");
      String tableName =
          createTempTable(
              connection,
              "ud_temporal_stage_binding_",
              "bind_path VARCHAR, id INTEGER, ltz TIMESTAMP_LTZ, tz TIMESTAMP_TZ,"
                  + " ntz TIMESTAMP_NTZ");
      Timestamp[] values = {new Timestamp(1_403_049_600_000L), new Timestamp(1_388_016_000_000L)};
      Calendar australia = Calendar.getInstance(australiaTimeZone);

      // When values on opposite sides of daylight-saving time are inserted inline and through
      // stage binding
      insertDstTimestampRows(connection, tableName, "inline", 0, values, australia, true);
      insertDstTimestampRows(connection, tableName, "stage", 1, values, australia, false);

      // Then every staged timestamp should equal its inline counterpart
      Timestamp[][] inline = new Timestamp[values.length][3];
      try (Statement statement = connection.createStatement();
          ResultSet resultSet =
              statement.executeQuery(
                  "SELECT bind_path, id, ltz, tz, ntz FROM "
                      + tableName
                      + " ORDER BY bind_path, id")) {
        for (String bindPath : new String[] {"inline", "stage"}) {
          for (int id = 0; id < values.length; id++) {
            assertTrue(resultSet.next(), "Expected " + bindPath + " DST row " + id);
            assertEquals(bindPath, resultSet.getString(1), "Unexpected DST bind path");
            assertFalse(resultSet.wasNull(), "DST bind path should not be NULL");
            assertEquals(id, resultSet.getInt(2), "Unexpected DST row id");
            assertFalse(resultSet.wasNull(), "DST row id should not be NULL");
            for (int column = 0; column < 3; column++) {
              Timestamp actual = resultSet.getTimestamp(column + 3);
              assertFalse(resultSet.wasNull(), "DST timestamp should not be NULL");
              if ("inline".equals(bindPath)) {
                inline[id][column] = actual;
              } else {
                assertEquals(
                    inline[id][column],
                    actual,
                    "Stage timestamp column " + column + " should match inline for id " + id);
              }
            }
          }
        }
        assertFalse(resultSet.next(), "Expected exactly " + values.length * 2 + " DST rows");
      }
    } finally {
      TimeZone.setDefault(originalTimeZone);
    }
  }

  private void setStageArrayBindingThreshold(Connection connection, int threshold)
      throws Exception {
    execute(connection, "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = " + threshold);
  }

  private void insertTimeRows(
      Connection connection,
      String tableName,
      String bindPath,
      int stageBindingThreshold,
      Time[] times)
      throws Exception {
    setStageArrayBindingThreshold(connection, stageBindingThreshold);
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?, ?, ?)")) {
      for (int id = 0; id < times.length; id++) {
        preparedStatement.setString(1, bindPath);
        preparedStatement.setInt(2, id);
        preparedStatement.setTime(3, times[id]);
        preparedStatement.addBatch();
      }
      preparedStatement.executeBatch();
    }
  }

  private void insertTemporalRows(
      Connection connection,
      String tableName,
      String bindPath,
      int stageBindingThreshold,
      Date[] dates,
      Timestamp[] timestamps)
      throws Exception {
    setStageArrayBindingThreshold(connection, stageBindingThreshold);
    String insertSql = "INSERT INTO " + tableName + " VALUES (?, ?, ?, ?)";
    try (PreparedStatement preparedStatement = connection.prepareStatement(insertSql)) {
      for (int id = 0; id < timestamps.length; id++) {
        preparedStatement.setString(1, bindPath);
        preparedStatement.setInt(2, id);
        preparedStatement.setDate(3, dates[id]);
        preparedStatement.setTimestamp(4, timestamps[id]);
        preparedStatement.addBatch();
      }
      preparedStatement.executeBatch();
    }
  }

  private void assertTemporalRowsMatch(
      Connection connection,
      String tableName,
      String timestampType,
      Date[] expectedDates,
      Timestamp[] expectedTimestamps)
      throws Exception {
    String selectSql = "SELECT bind_path, id, d, ts FROM " + tableName + " ORDER BY bind_path, id";
    int expectedRowCount = expectedTimestamps.length;
    Date[] inlineDates = new Date[expectedRowCount];
    Timestamp[] inlineTimestamps = new Timestamp[expectedRowCount];
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(selectSql)) {
      for (String bindPath : new String[] {"inline", "stage"}) {
        for (int id = 0; id < expectedRowCount; id++) {
          assertTrue(resultSet.next(), "Expected " + bindPath + " temporal row " + id);
          assertEquals(bindPath, resultSet.getString(1), "Unexpected temporal bind path");
          assertFalse(resultSet.wasNull(), "Temporal bind path should not be NULL");
          assertEquals(id, resultSet.getInt(2), "Unexpected temporal row id");
          assertFalse(resultSet.wasNull(), "Temporal row id should not be NULL");
          Date actualDate = resultSet.getDate(3);
          assertFalse(resultSet.wasNull(), bindPath + " DATE should not be NULL");
          Timestamp actualTimestamp = resultSet.getTimestamp(4);
          boolean timestampWasNull = resultSet.wasNull();
          if ("inline".equals(bindPath)) {
            inlineDates[id] = actualDate;
            inlineTimestamps[id] = actualTimestamp;
            assertEquals(
                expectedDates[id].toLocalDate(),
                actualDate.toLocalDate(),
                "Unexpected inline DATE for id " + id);
            assertEquals(
                expectedTimestamps[id] == null,
                timestampWasNull,
                "Unexpected inline " + timestampType + " nullability for id " + id);
          } else {
            assertEquals(
                inlineDates[id].toLocalDate(),
                actualDate.toLocalDate(),
                "Stage DATE should match inline DATE for id " + id);
            assertEquals(
                expectedTimestamps[id] == null,
                timestampWasNull,
                "Unexpected stage " + timestampType + " nullability for id " + id);
            if (expectedTimestamps[id] == null) {
              assertNull(actualTimestamp, "Stage " + timestampType + " should be NULL");
            } else {
              assertEquals(
                  inlineTimestamps[id],
                  actualTimestamp,
                  "Stage " + timestampType + " should match inline value for id " + id);
            }
          }
        }
      }
      assertFalse(
          resultSet.next(),
          "Expected exactly " + (expectedRowCount * 2) + " " + timestampType + " rows");
    }
  }

  private void insertExplicitTimestampRows(
      Connection connection,
      String tableName,
      String bindPath,
      int stageBindingThreshold,
      Timestamp[] values)
      throws Exception {
    setStageArrayBindingThreshold(connection, stageBindingThreshold);
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?, ?, ?, ?)")) {
      for (int id = 0; id < values.length; id++) {
        preparedStatement.setString(1, bindPath);
        preparedStatement.setInt(2, id);
        preparedStatement.setObject(3, values[id], SnowflakeType.EXTRA_TYPES_TIMESTAMP_NTZ);
        preparedStatement.setObject(4, values[id], SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ);
        preparedStatement.addBatch();
      }
      preparedStatement.executeBatch();
    }
  }

  private void insertDstTimestampRows(
      Connection connection,
      String tableName,
      String bindPath,
      int stageBindingThreshold,
      Timestamp[] values,
      Calendar ntzCalendar,
      boolean useNtzCalendar)
      throws Exception {
    setStageArrayBindingThreshold(connection, stageBindingThreshold);
    try (PreparedStatement preparedStatement =
        connection.prepareStatement("INSERT INTO " + tableName + " VALUES (?, ?, ?, ?, ?)")) {
      for (int id = 0; id < values.length; id++) {
        preparedStatement.setString(1, bindPath);
        preparedStatement.setInt(2, id);
        preparedStatement.setTimestamp(3, values[id]);
        preparedStatement.setTimestamp(4, values[id]);
        if (useNtzCalendar) {
          preparedStatement.setTimestamp(5, values[id], ntzCalendar);
        } else {
          preparedStatement.setTimestamp(5, values[id]);
        }
        preparedStatement.addBatch();
      }
      preparedStatement.executeBatch();
    }
  }
}
