package net.snowflake.client.internal.jdbc;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.sql.Date;
import java.time.Instant;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;

/** Unit tests for {@link SnowflakeDateWithTimezone#toString()}. */
public class SnowflakeDateWithTimezoneTest {

  @Test
  public void shouldFormatInCarriedZoneWhenUseSessionTimezone() {
    long millis = Instant.parse("2024-01-15T02:00:00Z").toEpochMilli();
    // In UTC the wall-clock date is 2024-01-15.
    assertEquals(
        "2024-01-15",
        new SnowflakeDateWithTimezone(millis, TimeZone.getTimeZone("UTC"), true).toString());
    // In America/New_York (UTC-5) the same instant is still 2024-01-14 21:00 → 2024-01-14.
    assertEquals(
        "2024-01-14",
        new SnowflakeDateWithTimezone(millis, TimeZone.getTimeZone("America/New_York"), true)
            .toString());
  }

  @Test
  public void shouldDelegateToSuperWhenNotUseSessionTimezone() {
    long millis = Instant.parse("2024-01-15T02:00:00Z").toEpochMilli();
    // useSessionTimezone=false → identical to plain java.sql.Date (JVM-default rendering), so the
    // carried zone is irrelevant.
    assertEquals(
        new Date(millis).toString(),
        new SnowflakeDateWithTimezone(millis, TimeZone.getTimeZone("Asia/Tokyo"), false)
            .toString());
  }
}
