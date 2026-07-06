package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.sql.Timestamp;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link ArrowDateUtil#adjustTimestamp} (the Julian→Gregorian timestamp shift) and
 * the {@code setTimestamp} bind encoders ({@link ArrowDateUtil#timestampToBindString}, {@link
 * ArrowDateUtil#timestampWithCalendarToBindString}, {@link ArrowDateUtil#timestampTzToBindString}).
 */
public class ArrowDateUtilTest {

  /** A whole-second epoch-millis for 2024-01-15T12:34:56Z; nanos are set separately per test. */
  private static final long TS_2024_SECONDS_MILLIS = 1_705_323_296_000L;

  private static final int NANOS_789_012_345 = 789_012_345;

  @Test
  public void shouldNotAdjustTimestampOnOrAfterGregorianCutover() {
    Timestamp modern = new Timestamp(1546391837000L); // 2019-01-01, well after 1582-10-05
    modern.setNanos(123_456_789);
    Timestamp adjusted = ArrowDateUtil.adjustTimestamp(modern);
    // No shift needed → same instance returned, nanos untouched.
    assertSame(modern, adjusted);
    assertEquals(123_456_789, adjusted.getNanos());
  }

  @Test
  public void shouldAdjustPre1582TimestampByJulianDiffPreservingNanos() {
    // ~year 1494, comfortably before the 1582-10-05 Gregorian cutover.
    Timestamp ancient = new Timestamp(-15_000_000_000_000L);
    ancient.setNanos(123_456_789);

    long diff = ArrowDateUtil.msDiffJulianToGregorian(ancient);
    // Sanity: a pre-cutover date must require a non-zero correction.
    assertNotEquals(0L, diff);

    Timestamp adjusted = ArrowDateUtil.adjustTimestamp(ancient);
    assertEquals(ancient.getTime() + diff, adjusted.getTime());
    assertEquals(123_456_789, adjusted.getNanos());
  }

  @Test
  public void shouldEncodePlainTimestampAsEpochNanosDecimalStringWithFullNanoFidelity() {
    Timestamp ts = new Timestamp(TS_2024_SECONDS_MILLIS);
    ts.setNanos(NANOS_789_012_345);
    // 1705323296 seconds * 1e9 + 789012345 nanos, all nine fractional digits preserved.
    assertEquals("1705323296789012345", ArrowDateUtil.timestampToBindString(ts));
  }

  @Test
  public void shouldEncodePreEpochTimestampAsNegativeSecondsWithPositiveNanos() {
    // 1969-12-31 23:59:58 UTC + 1 nano: the legacy /1000 truncation yields negative seconds with a
    // positive nano remainder, reassembled server-side. After the Gregorian cutover → no Julian
    // shift. Constructed from millis (not Timestamp.valueOf) to stay timezone-independent.
    Timestamp ts = new Timestamp(-2_000L);
    ts.setNanos(1);
    assertEquals("-1999999999", ArrowDateUtil.timestampToBindString(ts));
  }

  @Test
  public void shouldApplyJulianCorrectionOnlyOnPlainTimestampSetterNotCalendarOverload() {
    // ~year 1494, before the 1582-10-05 Gregorian cutover.
    Timestamp ancient = new Timestamp(-15_000_000_000_000L);
    ancient.setNanos(123_456_789);
    long diff = ArrowDateUtil.msDiffJulianToGregorian(ancient);
    assertNotEquals(0L, diff, "pre-cutover instant must require a non-zero Julian correction");

    TimeZone utc = TimeZone.getTimeZone("UTC");
    // Plain setter backs the Julian diff out of the seconds; the Calendar overload (UTC → zero
    // offset) does not, so the two encodings must differ for a pre-1582 instant.
    String plainSeconds = expectedEpochNanos(ancient.getTime() - diff, ancient.getNanos());
    assertEquals(plainSeconds, ArrowDateUtil.timestampToBindString(ancient));
    assertEquals(
        expectedEpochNanos(ancient.getTime(), ancient.getNanos()),
        ArrowDateUtil.timestampWithCalendarToBindString(ancient, utc));
    assertNotEquals(
        ArrowDateUtil.timestampToBindString(ancient),
        ArrowDateUtil.timestampWithCalendarToBindString(ancient, utc));
  }

  @Test
  public void shouldShiftInstantByCalendarOffsetForLtzNtzCalendarOverload() {
    Timestamp ts = new Timestamp(TS_2024_SECONDS_MILLIS);
    ts.setNanos(NANOS_789_012_345);
    // GMT+02:00 → +7_200_000 ms added to the instant before encoding (no Julian correction).
    TimeZone plus2 = TimeZone.getTimeZone("GMT+02:00");
    assertEquals("1705330496789012345", ArrowDateUtil.timestampWithCalendarToBindString(ts, plus2));
  }

  @Test
  public void shouldEncodeTimestampTzAsEpochNanosPlusBiasedOffsetCode() {
    Timestamp ts = new Timestamp(TS_2024_SECONDS_MILLIS);
    ts.setNanos(NANOS_789_012_345);
    // The instant is NOT shifted; the offset is stored separately as minutes-from-UTC + 1440.
    // These codes must round-trip with the read-side decode (index - 1440): 1440→UTC, 1560→+2h,
    // 960→-8h (see ArrowResultUtil timezone-index handling).
    assertEquals(
        "1705323296789012345 1440",
        ArrowDateUtil.timestampTzToBindString(ts, TimeZone.getTimeZone("UTC")));
    assertEquals(
        "1705323296789012345 1560",
        ArrowDateUtil.timestampTzToBindString(ts, TimeZone.getTimeZone("GMT+02:00")));
    assertEquals(
        "1705323296789012345 960",
        ArrowDateUtil.timestampTzToBindString(ts, TimeZone.getTimeZone("GMT-08:00")));
  }

  /** Recomputes the legacy epoch-nanoseconds string for a given millis/nanos pair. */
  private static String expectedEpochNanos(long millis, int nanos) {
    return java.math.BigDecimal.valueOf(millis / 1000)
        .scaleByPowerOfTen(9)
        .add(java.math.BigDecimal.valueOf(nanos))
        .toString();
  }
}
