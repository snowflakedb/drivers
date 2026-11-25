package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.sql.Time;
import java.sql.Timestamp;
import java.text.DateFormat;
import java.text.SimpleDateFormat;
import java.time.Duration;
import java.time.Period;
import java.util.Date;
import java.util.TimeZone;
import net.snowflake.client.internal.util.SnowflakeTypeHelper;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class CsvValuesTest {

  private DateFormat dateFormat;
  private DateFormat timeFormat;
  private DateFormat timestampFormat;
  private DateFormat timestampTzFormat;

  @BeforeEach
  void setUpUtcFormats() {
    dateFormat = utc(SnowflakeTypeHelper.DATE_OR_TIME_FORMAT_PATTERN);
    timeFormat = utc(SnowflakeTypeHelper.TIME_FORMAT_PATTERN);
    timestampFormat = utc(SnowflakeTypeHelper.TIMESTAMP_FORMAT_PATTERN);
    timestampTzFormat = utc(SnowflakeTypeHelper.TIMESTAMP_FORMAT_TZ_PATTERN);
  }

  @Test
  void shouldReturnNullLexicalValueForNull() {
    assertNull(lexical(null));
  }

  @Test
  void shouldFormatUtilDateAndSqlDateWithDateFormat() {
    assertEquals("1970-01-02T00:00:01.500Z", lexical(new Date(86_401_500L)));
    assertEquals("1970-01-02T00:00:01.500Z", lexical(new java.sql.Date(86_401_500L)));
  }

  @Test
  void shouldFormatTimeWithTimeFormat() {
    assertEquals("01:02:03.000", lexical(new Time(3_723_000L)));
  }

  @Test
  void shouldFormatTimestampWithFullNanosecondPrecision() {
    Timestamp ts = new Timestamp(0L);
    ts.setNanos(123_456_789);

    assertEquals("1970-01-01T00:00:00.123456789Z", lexical(ts));
  }

  @Test
  void shouldFormatFloatingPointAsHexString() {
    assertEquals("0x1.044ccp4", lexical(0x4.11_33p2));
    assertEquals("0x1.8p1", lexical(3.0f));
  }

  @Test
  void shouldFormatNumericAndIntervalValuesWithToString() {
    assertEquals("42", lexical(42));
    assertEquals("42", lexical(42L));
    assertEquals("12345678901234567890.123", lexical(new BigDecimal("12345678901234567890.123")));
    assertEquals("P1Y2M3D", lexical(Period.of(1, 2, 3)));
    assertEquals("PT1H", lexical(Duration.ofHours(1)));
  }

  @Test
  void shouldFormatBinaryAsUppercaseHex() {
    assertEquals("00ABFF", lexical(new byte[] {0x00, (byte) 0xAB, (byte) 0xFF}));
    assertEquals("", lexical(new byte[0]));
  }

  @Test
  void shouldFormatOtherObjectsWithStringValueOf() {
    assertEquals("true", lexical(Boolean.TRUE));
    assertEquals("{\"a\":1}", lexical("{\"a\":1}"));
  }

  @Test
  void shouldEscapeNullAsUnquotedEmptyField() {
    assertEquals("", CsvValues.escapeForCsv(null));
  }

  @Test
  void shouldEscapeEmptyStringAsQuotedEmptyField() {
    assertEquals("\"\"", CsvValues.escapeForCsv(""));
  }

  @ParameterizedTest
  @CsvSource(
      delimiter = '|',
      quoteCharacter = '\'',
      value = {
        "plain|plain",
        "a,b|'\"a,b\"'",
        "a\\b|'\"a\\b\"'",
        "say \"hi\"|'\"say \"\"hi\"\"\"'",
      })
  void shouldQuoteOnlyCellsThatNeedIt(String value, String expected) {
    assertEquals(expected, CsvValues.escapeForCsv(value));
  }

  @Test
  void shouldQuoteCellContainingLineFeed() {
    assertEquals("\"a\nb\"", CsvValues.escapeForCsv("a\nb"));
  }

  @Test
  void shouldLeaveCarriageReturnUnquoted() {
    assertEquals("a\rb", CsvValues.escapeForCsv("a\rb"));
  }

  private String lexical(Object value) {
    return CsvValues.lexicalValue(
        value, dateFormat, timeFormat, timestampFormat, timestampTzFormat);
  }

  private static DateFormat utc(String pattern) {
    SimpleDateFormat format = new SimpleDateFormat(pattern);
    format.setTimeZone(TimeZone.getTimeZone("UTC"));
    return format;
  }
}
