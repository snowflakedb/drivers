package net.snowflake.client.internal.common.core;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.sql.Date;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;

class SnowflakeDateTimeFormatTest {

  @Test
  void shouldFormatDateFromSqlPattern() {
    SnowflakeDateTimeFormat formatter = SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD");
    Date date = Date.valueOf(LocalDate.of(2024, 3, 15));
    TimeZone zone = TimeZone.getDefault();
    assertEquals("2024-03-15", formatter.format(date, zone));
  }

  @Test
  void shouldFormatTimestampWithTimePattern() {
    SnowflakeDateTimeFormat formatter =
        SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS");
    Timestamp timestamp = Timestamp.valueOf("2024-03-15 10:30:45");
    TimeZone zone = TimeZone.getDefault();
    assertEquals("2024-03-15 10:30:45", formatter.format(timestamp, zone, 0));
  }

  @Test
  void shouldFormatTimestampWithFractions() {
    SnowflakeDateTimeFormat formatter =
        SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FF3");
    Timestamp timestamp = Timestamp.valueOf("2024-03-15 10:30:45.123");
    TimeZone zone = TimeZone.getDefault();
    assertEquals("2024-03-15 10:30:45.123", formatter.format(timestamp, zone, 3));
  }

  @Test
  void shouldAcceptEmptySqlFormat() {
    SnowflakeDateTimeFormat formatter = SnowflakeDateTimeFormat.fromSqlFormat("");
    assertEquals("", formatter.getSqlFormat());
  }
}
