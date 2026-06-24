package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.time.LocalTime;
import org.junit.jupiter.api.Test;

public class SessionDataConversionContextTest {
  // 12:34:56.123456789 since midnight.
  private static final LocalTime SAMPLE =
      LocalTime.ofNanoOfDay((12 * 3600L + 34 * 60L + 56L) * 1_000_000_000L + 123_456_789L);

  private static String format(String snowflakeFormat, int scale) {
    return SessionDataConversionContext.buildTimeFormatter(snowflakeFormat).format(SAMPLE, scale);
  }

  @Test
  public void shouldBuildDefaultTimeFormatter() {
    assertEquals("12:34:56", format(null, 9));
    assertEquals("12:34:56", format("", 9));
    assertEquals("12:34:56", format("HH24:MI:SS", 9));
  }

  @Test
  public void shouldBuildTimeFormatterWithFractionalSeconds() {
    assertEquals("12:34:56.123", format("HH24:MI:SS.FF3", 9));
    assertEquals("12:34:56.123456789", format("HH24:MI:SS.FF9", 9));
    // Bare FF uses the column scale passed to format().
    assertEquals("12:34:56.123", format("HH24:MI:SS.FF", 3));
    assertEquals("12:34:56.123456789", format("HH24:MI:SS.FF", 9));
  }

  @Test
  public void shouldBuildTimeFormatterWithTwelveHourAmPm() {
    assertEquals("12:34:56 PM", format("HH12:MI:SS AM", 9));
  }

  @Test
  public void shouldTranslateDefaultDateFormat() {
    assertEquals("yyyy-MM-dd", SessionDataConversionContext.translateDateFormat(null));
    assertEquals("yyyy-MM-dd", SessionDataConversionContext.translateDateFormat("YYYY-MM-DD"));
  }
}
