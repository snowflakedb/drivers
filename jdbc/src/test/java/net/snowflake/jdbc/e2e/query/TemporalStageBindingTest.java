package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.Date;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

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
    Date[] dates = {new Date(0L), Date.valueOf("0001-01-01"), Date.valueOf("3000-01-01")};
    Timestamp[] timestamps = {
      new Timestamp(-1000L),
      Timestamp.valueOf("0001-01-01 00:00:00"),
      Timestamp.valueOf("3000-01-01 00:00:00")
    };

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
      assertTemporalRowsMatch(connection, tableName, timestampType, dates.length);
    }
  }

  @Test
  public void shouldPreserveTimeValuesBetweenInlineAndStageBinding() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();
    Time[] times = {
      Time.valueOf("00:00:00"),
      Time.valueOf("00:00:01"),
      Time.valueOf("13:00:00"),
      Time.valueOf("14:00:00"),
      new Time(Time.valueOf("23:59:59").getTime() + 999L)
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
          assertFalse(resultSet.wasNull(), bindPath + " TIME should not be NULL");
          if ("inline".equals(bindPath)) {
            inlineTimes[id] = actualTime;
          } else {
            assertEquals(
                inlineTimes[id].getTime(),
                actualTime.getTime(),
                "Stage TIME should match inline TIME for id " + id);
          }
        }
      }
      assertFalse(resultSet.next(), "Expected exactly " + (times.length * 2) + " TIME rows");
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
      for (int id = 0; id < dates.length; id++) {
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
      Connection connection, String tableName, String timestampType, int expectedRowCount)
      throws Exception {
    String selectSql = "SELECT bind_path, id, d, ts FROM " + tableName + " ORDER BY bind_path, id";
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
          assertFalse(resultSet.wasNull(), bindPath + " " + timestampType + " should not be NULL");
          if ("inline".equals(bindPath)) {
            inlineDates[id] = actualDate;
            inlineTimestamps[id] = actualTimestamp;
          } else {
            assertEquals(
                inlineDates[id].toLocalDate(),
                actualDate.toLocalDate(),
                "Stage DATE should match inline DATE for id " + id);
            assertEquals(
                inlineTimestamps[id].toLocalDateTime(),
                actualTimestamp.toLocalDateTime(),
                "Stage " + timestampType + " should match inline value for id " + id);
          }
        }
      }
      assertFalse(
          resultSet.next(),
          "Expected exactly " + (expectedRowCount * 2) + " " + timestampType + " rows");
    }
  }
}
