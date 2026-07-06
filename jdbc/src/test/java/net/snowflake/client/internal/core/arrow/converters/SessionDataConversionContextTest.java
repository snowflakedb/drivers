package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Date;
import java.sql.Timestamp;
import java.time.Instant;
import java.time.LocalTime;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.ArrowDateUtil;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import org.junit.jupiter.api.Test;

public class SessionDataConversionContextTest {
  // 12:34:56.123456789 since midnight.
  private static final LocalTime SAMPLE =
      LocalTime.ofNanoOfDay((12 * 3600L + 34 * 60L + 56L) * 1_000_000_000L + 123_456_789L);

  // 2024-01-15 12:34:56 UTC, scale 0.
  private static final TimeZone UTC = TimeZone.getTimeZone("UTC");
  private static final Timestamp SAMPLE_TS =
      new Timestamp(Instant.parse("2024-01-15T12:34:56Z").toEpochMilli());

  private static String format(String snowflakeFormat, int scale) {
    return SessionDataConversionContext.buildTimeFormatter(snowflakeFormat).format(SAMPLE, scale);
  }

  /** Build a context from a server-parameter map, mirroring the connection-init path. */
  private static DataConversionContext contextFrom(Map<String, String> params) throws Exception {
    CoreDriverApi api = mock(CoreDriverApi.class);
    when(api.connectionGetAllParameters(any()))
        .thenReturn(
            ConnectionGetAllParametersResponse.newBuilder().putAllParameters(params).build());
    return SessionDataConversionContext.fromConnection(
        api, ConnectionHandle.getDefaultInstance(), null);
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

  @Test
  public void shouldFallBackToGenericFormatWhenSpecializedTimestampFormatIsEmpty() {
    String generic = "YYYY-MM-DD HH24:MI:SS";
    assertEquals(
        "2024-01-15 12:34:56",
        SessionDataConversionContext.buildTimestampFormatter(null, generic)
            .format(SAMPLE_TS, UTC, 0));
    assertEquals(
        "2024-01-15 12:34:56",
        SessionDataConversionContext.buildTimestampFormatter("", generic)
            .format(SAMPLE_TS, UTC, 0));
  }

  @Test
  public void shouldUseSpecializedTimestampFormatWhenSet() {
    assertEquals(
        "2024/01/15",
        SessionDataConversionContext.buildTimestampFormatter("YYYY/MM/DD", "YYYY-MM-DD HH24:MI:SS")
            .format(SAMPLE_TS, UTC, 0));
  }

  @Test
  public void shouldDefaultAllTimestampFormattersToGenericDefaultWhenParamsAbsent()
      throws Exception {
    DataConversionContext ctx = contextFrom(Collections.emptyMap());
    SnowflakeDateTimeFormat expected =
        SnowflakeDateTimeFormat.fromSqlFormat(
            DataConversionContext.DEFAULT_TIMESTAMP_OUTPUT_FORMAT);
    String expectedRendering = expected.format(SAMPLE_TS, UTC, 0);
    assertEquals(expectedRendering, ctx.getTimestampNTZFormatter().format(SAMPLE_TS, UTC, 0));
    assertEquals(expectedRendering, ctx.getTimestampLTZFormatter().format(SAMPLE_TS, UTC, 0));
    assertEquals(expectedRendering, ctx.getTimestampTZFormatter().format(SAMPLE_TS, UTC, 0));
  }

  @Test
  public void shouldResolvePerTypeTimestampFormatsWithGenericFallback() throws Exception {
    Map<String, String> params = new HashMap<>();
    params.put("TIMESTAMP_OUTPUT_FORMAT", "YYYY-MM-DD HH24:MI:SS");
    params.put("TIMESTAMP_NTZ_OUTPUT_FORMAT", "YYYY/MM/DD");
    DataConversionContext ctx = contextFrom(params);
    // NTZ uses its specialized format; LTZ and TZ fall back to the generic TIMESTAMP_OUTPUT_FORMAT.
    assertEquals("2024/01/15", ctx.getTimestampNTZFormatter().format(SAMPLE_TS, UTC, 0));
    assertEquals("2024-01-15 12:34:56", ctx.getTimestampLTZFormatter().format(SAMPLE_TS, UTC, 0));
    assertEquals("2024-01-15 12:34:56", ctx.getTimestampTZFormatter().format(SAMPLE_TS, UTC, 0));
  }

  @Test
  public void shouldDefaultTimestampFlagsWhenParamsAbsent() throws Exception {
    DataConversionContext ctx = contextFrom(Collections.emptyMap());
    assertFalse(ctx.isTreatNTZAsUTC());
    assertTrue(ctx.isHonorClientTZForTimestampNTZ());
    assertEquals("TIMESTAMP_LTZ", ctx.getTimestampMappedType());
  }

  @Test
  public void shouldParseTimestampFlagsFromParams() throws Exception {
    Map<String, String> params = new HashMap<>();
    params.put("CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ", "false");
    params.put("CLIENT_TIMESTAMP_TYPE_MAPPING", "TIMESTAMP_NTZ");
    DataConversionContext ctx = contextFrom(params);
    assertFalse(ctx.isHonorClientTZForTimestampNTZ());
    assertEquals("TIMESTAMP_NTZ", ctx.getTimestampMappedType());
  }
}
