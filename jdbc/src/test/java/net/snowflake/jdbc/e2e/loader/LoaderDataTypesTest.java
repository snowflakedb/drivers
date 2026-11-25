package net.snowflake.jdbc.e2e.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Time;
import java.sql.Timestamp;
import java.text.SimpleDateFormat;
import java.util.Arrays;
import java.util.Collections;
import java.util.Date;
import java.util.Map;
import java.util.TimeZone;
import java.util.stream.Stream;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoaderProperty;
import net.snowflake.client.api.loader.Operation;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

public class LoaderDataTypesTest extends LoaderTestBase {

  private static final TimeZone UTC = TimeZone.getTimeZone("UTC");

  private static final TimeZone LOS_ANGELES = TimeZone.getTimeZone("America/Los_Angeles");

  @Test
  public void shouldLoadTimeAndDateValuesInUtc() throws Exception {
    // Given a table with TIME and DATE columns
    String table = uniqueName("LOADER_TIME_TEST");
    createTable(table, "ID int, C1 time, C2 date");
    try {
      Map<LoaderProperty, Object> properties =
          loaderProperties(
              table,
              Operation.INSERT,
              Arrays.asList("ID", "C1", "C2"),
              Collections.singletonList("ID"));
      properties.put(LoaderProperty.startTransaction, true);
      properties.put(LoaderProperty.truncateTable, true);
      RecordingLoadResultListener listener = new RecordingLoadResultListener();
      try (Loader loader = createLoader(properties, listener)) {
        loader.start();

        // When a java.sql.Time and a java.util.Date are loaded
        for (int i = 0; i < 10; i++) {
          loader.submitRow(new Object[] {i, new Time(3_723_000L), new Date(1_700_000_000_000L)});
        }
        loader.finish();
      }

      // Then the TIME and DATE columns hold the UTC wall clock and calendar day
      assertEquals(0, listener.getErrorCount(), "error count");
      try (Statement statement = getDefaultConnection().createStatement();
          ResultSet rs =
              statement.executeQuery(
                  "SELECT TO_VARCHAR(C1, 'HH24:MI:SS.FF3'), TO_VARCHAR(C2, 'YYYY-MM-DD') FROM \""
                      + table
                      + "\" WHERE ID = 0")) {
        assertTrue(rs.next(), "row with ID 0 should exist");
        assertEquals("01:02:03.000", rs.getString(1), "TIME column");
        assertFalse(rs.wasNull(), "TIME column should not be NULL");
        assertEquals("2023-11-14", rs.getString(2), "DATE column");
        assertFalse(rs.wasNull(), "DATE column should not be NULL");
      }
    } finally {
      dropTable(table);
    }
  }

  @Test
  public void shouldLoadNullAsNullAndEmptyStringAsEmptyByDefault() throws Exception {
    // Given a string table and a loader with the default copyEmptyFieldAsEmpty=false
    String table = uniqueName("LOADER_EMPTY_FIELD");
    createTable(table, "ID int, C1 string, C2 string");
    try {
      // When a NULL and an empty string are loaded
      loadNullAndEmptyString(table, false);

      // Then the NULL stays NULL and the empty string stays empty
      try (Statement statement = getDefaultConnection().createStatement();
          ResultSet rs = statement.executeQuery("SELECT C1, C2 FROM \"" + table + "\"")) {
        assertTrue(rs.next(), "the loaded row should exist");
        assertNull(rs.getString(1), "C1 loaded from NULL");
        assertTrue(rs.wasNull(), "C1 should be NULL");
        assertEquals("", rs.getString(2), "C2 loaded from an empty string");
        assertFalse(rs.wasNull(), "C2 should not be NULL");
      }
    } finally {
      dropTable(table);
    }
  }

  @Test
  public void shouldLoadNullAsEmptyStringWhenCopyEmptyFieldAsEmptyIsSet() throws Exception {
    // Given a string table and a loader with copyEmptyFieldAsEmpty=true
    String table = uniqueName("LOADER_EMPTY_FIELD");
    createTable(table, "ID int, C1 string, C2 string");
    try {
      // When a NULL and an empty string are loaded
      loadNullAndEmptyString(table, true);

      // Then both are stored as empty strings
      try (Statement statement = getDefaultConnection().createStatement();
          ResultSet rs = statement.executeQuery("SELECT C1, C2 FROM \"" + table + "\"")) {
        assertTrue(rs.next(), "the loaded row should exist");
        assertEquals("", rs.getString(1), "C1 loaded from NULL");
        assertFalse(rs.wasNull(), "C1 should not be NULL");
        assertEquals("", rs.getString(2), "C2 loaded from an empty string");
        assertFalse(rs.wasNull(), "C2 should not be NULL");
      }
    } finally {
      dropTable(table);
    }
  }

  @Test
  public void shouldPreserveNanosecondTimestampsReadFromResultSet() throws Exception {
    // Given a source table with TIMESTAMP_NTZ(9) values and an empty copy of it
    String srcTable = uniqueName("LOAD_TIMESTAMP_MS_SRC");
    String dstTable = uniqueName("LOAD_TIMESTAMP_MS_DST");
    createTable(srcTable, "C1 int, C2 timestamp_ntz(9)");
    createTable(dstTable, "C1 int, C2 timestamp_ntz(9)");
    TimeZone jvmDefault = TimeZone.getDefault();
    try (Statement statement = getDefaultConnection().createStatement()) {
      statement.execute(
          "INSERT INTO \""
              + srcTable
              + "\"(C1, C2) VALUES"
              + "(1, '2018-05-12 12:34:56.123456789'),"
              + "(2, '2018-05-13 03:45:27.988'),"
              + "(3, '2018-05-14 07:12:34'),"
              + "(4, '2018-12-15 10:43:45.000000876')");
      TimeZone.setDefault(UTC);
      Map<LoaderProperty, Object> properties =
          loaderProperties(dstTable, Operation.INSERT, Arrays.asList("C1", "C2"), null);
      properties.put(LoaderProperty.onError, "ABORT_STATEMENT");
      try (Loader loader = createLoader(properties, new RecordingLoadResultListener(true));
          Connection reader = openConnection();
          Statement select = reader.createStatement();
          ResultSet rs = select.executeQuery("SELECT C1, C2 FROM \"" + srcTable + "\"")) {
        loader.start();

        // When the rows read through getObject are loaded into the copy
        while (rs.next()) {
          loader.submitRow(new Object[] {rs.getObject(1), rs.getObject(2)});
        }
        loader.finish();
      }

      // Then both tables hold the same values to the nanosecond
      try (ResultSet diff =
          statement.executeQuery(
              "SELECT * FROM \"" + dstTable + "\" MINUS SELECT * FROM \"" + srcTable + "\"")) {
        assertFalse(diff.next(), "the copy should match the source");
      }
      assertEquals(4, countRows(dstTable), "copied rows");
    } finally {
      TimeZone.setDefault(jvmDefault);
      dropTable(srcTable);
      dropTable(dstTable);
    }
  }

  static Stream<Arguments> timestampValues() {
    return Stream.of(
        Arguments.of(new Date(1_502_931_205_123L), false),
        Arguments.of(new Date(1_502_931_205_123L), true),
        Arguments.of(Timestamp.valueOf("0001-01-01 08:00:00"), false),
        Arguments.of(Timestamp.valueOf("0001-01-01 08:00:00"), true),
        Arguments.of(java.sql.Date.valueOf("2001-01-02"), false),
        Arguments.of(java.sql.Date.valueOf("2001-01-02"), true));
  }

  @ParameterizedTest
  @MethodSource("timestampValues")
  public void shouldLoadDateAndTimestampValuesIntoTimestampNtz(
      Object value, boolean useLocalTimezone) throws Exception {
    // Given a TIMESTAMP_NTZ table and a loader with the given useLocalTimezone setting
    String table = uniqueName("LOADER_TEST_TIMESTAMP");
    createTable(table, "ID int, C1 varchar(255), C2 timestamp_ntz");
    try {
      // When the value is loaded and read back
      String retrieved = loadAndReadTimestamp(table, value, useLocalTimezone, false);

      // Then the stored wall clock matches the value in the zone the loader formatted it in
      assertEquals(formatIn(value, useLocalTimezone ? LOS_ANGELES : UTC), retrieved);
    } finally {
      dropTable(table);
    }
  }

  static Stream<Arguments> timeValues() {
    return Stream.of(
        Arguments.of(new Time(1_502_931_205_000L), false),
        Arguments.of(new Time(1_502_931_205_000L), true),
        Arguments.of(Time.valueOf("12:34:56"), false),
        Arguments.of(Time.valueOf("12:34:56"), true));
  }

  @ParameterizedTest
  @MethodSource("timeValues")
  public void shouldLoadTimeValuesIntoTimestampNtzWhenMapTimeToTimestampIsSet(
      Time value, boolean useLocalTimezone) throws Exception {
    // Given a TIMESTAMP_NTZ table and a loader with mapTimeToTimestamp=true
    String table = uniqueName("LOADER_TEST_TIMESTAMP_V1");
    createTable(table, "ID int, C1 varchar(255), C2 timestamp_ntz");
    try {
      // When the java.sql.Time value is loaded and read back
      String retrieved = loadAndReadTimestamp(table, value, useLocalTimezone, true);

      // Then the full date and time of the value are stored
      assertEquals(formatIn(value, useLocalTimezone ? LOS_ANGELES : UTC), retrieved);
    } finally {
      dropTable(table);
    }
  }

  private void loadNullAndEmptyString(String table, boolean copyEmptyFieldAsEmpty)
      throws Exception {
    Map<LoaderProperty, Object> properties =
        loaderProperties(table, Operation.INSERT, Arrays.asList("ID", "C1", "C2"), null);
    properties.put(LoaderProperty.startTransaction, true);
    properties.put(LoaderProperty.truncateTable, true);
    properties.put(LoaderProperty.copyEmptyFieldAsEmpty, copyEmptyFieldAsEmpty);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    try (Loader loader = createLoader(properties, listener)) {
      loader.start();
      loader.submitRow(new Object[] {1, null, ""});
      loader.finish();
    }
    assertEquals(1, listener.getSubmittedRowCount(), "submitted rows");
  }

  /**
   * Loads five rows holding {@code value} with the JVM default timezone set to Los Angeles when
   * {@code useLocalTimezone} is set, otherwise UTC, and returns the stored wall clock.
   */
  private String loadAndReadTimestamp(
      String table, Object value, boolean useLocalTimezone, boolean mapTimeToTimestamp)
      throws Exception {
    Map<LoaderProperty, Object> properties =
        loaderProperties(table, Operation.INSERT, Arrays.asList("ID", "C1", "C2"), null);
    properties.put(LoaderProperty.startTransaction, true);
    properties.put(LoaderProperty.truncateTable, true);
    properties.put(LoaderProperty.useLocalTimezone, useLocalTimezone);
    properties.put(LoaderProperty.mapTimeToTimestamp, mapTimeToTimestamp);
    RecordingLoadResultListener listener = new RecordingLoadResultListener();
    TimeZone jvmDefault = TimeZone.getDefault();
    try (Loader loader = createLoader(properties, listener)) {
      TimeZone.setDefault(useLocalTimezone ? LOS_ANGELES : UTC);
      loader.start();
      for (int i = 0; i < 5; ++i) {
        loader.submitRow(new Object[] {i, "foo_" + i, value});
      }
      loader.finish();
    } finally {
      TimeZone.setDefault(jvmDefault);
    }
    assertEquals(0, listener.getErrorCount(), "error count");
    try (Statement statement = getDefaultConnection().createStatement();
        ResultSet rs =
            statement.executeQuery(
                "SELECT TO_VARCHAR(C2, 'YYYY-MM-DD\"T\"HH24:MI:SS.FF3') FROM \""
                    + table
                    + "\" WHERE ID = 0")) {
      assertTrue(rs.next(), "row with ID 0 should exist");
      String stored = rs.getString(1);
      assertFalse(rs.wasNull(), "C2 should not be NULL");
      return stored;
    }
  }

  private static String formatIn(Object value, TimeZone zone) {
    SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS");
    format.setTimeZone(zone);
    return format.format(value);
  }
}
