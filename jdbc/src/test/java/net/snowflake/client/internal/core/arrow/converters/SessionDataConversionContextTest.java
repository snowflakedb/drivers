package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Date;
import java.time.LocalTime;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import org.junit.jupiter.api.Test;

public class SessionDataConversionContextTest {
  // 12:34:56.123456789 since midnight.
  private static final LocalTime SAMPLE =
      LocalTime.ofNanoOfDay((12 * 3600L + 34 * 60L + 56L) * 1_000_000_000L + 123_456_789L);

  private static String format(String snowflakeFormat, int scale) {
    return SessionDataConversionContext.buildTimeFormatter(snowflakeFormat).format(SAMPLE, scale);
  }

  /** Build a context from a server-parameter map, mirroring the connection-init path. */
  private static DataConversionContext contextFrom(Map<String, String> params) throws Exception {
    CoreDriverApi api = mock(CoreDriverApi.class);
    when(api.connectionGetAllParameters(any()))
        .thenReturn(
            ConnectionGetAllParametersResponse.newBuilder().putAllParameters(params).build());
    return SessionDataConversionContext.fromConnection(api, ConnectionHandle.getDefaultInstance());
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
  public void shouldBuildDefaultDateFormatter() {
    // Default DATE_OUTPUT_FORMAT (absent or "YYYY-MM-DD") renders dates in ISO yyyy-MM-dd, matching
    // snowflake-jdbc's ResultUtil default. buildDateFormatter replaced the former
    // translateDateFormat
    // helper, so assert on the rendered output rather than the intermediate Java pattern.
    Date date = Date.valueOf("2024-01-15");
    assertEquals(
        "2024-01-15",
        ArrowDateUtil.getDateAsString(date, SessionDataConversionContext.buildDateFormatter(null)));
    assertEquals(
        "2024-01-15",
        ArrowDateUtil.getDateAsString(
            date, SessionDataConversionContext.buildDateFormatter("YYYY-MM-DD")));
  }

  @Test
  public void shouldDefaultGetDateUseNullTimezoneToTrueWhenParamAbsent() throws Exception {
    // JDBC_GET_DATE_USE_NULL_TIMEZONE is a client-only property, normally absent from the server
    // parameter map; snowflake-jdbc's SFBaseSession defaults it to true.
    assertTrue(contextFrom(Collections.emptyMap()).isGetDateUseNullTimezone());
  }

  @Test
  public void shouldParseGetDateUseNullTimezoneFromParams() throws Exception {
    Map<String, String> falseParam = new HashMap<>();
    falseParam.put("JDBC_GET_DATE_USE_NULL_TIMEZONE", "false");
    assertFalse(contextFrom(falseParam).isGetDateUseNullTimezone());

    Map<String, String> trueParam = new HashMap<>();
    trueParam.put("JDBC_GET_DATE_USE_NULL_TIMEZONE", "true");
    assertTrue(contextFrom(trueParam).isGetDateUseNullTimezone());
  }
}
