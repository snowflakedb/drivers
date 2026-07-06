package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Timestamp;
import java.util.TimeZone;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for the timestamp math added to {@link ArrowResultUtil} in the timestamp migration.
 */
public class ArrowResultUtilTest {
  private static final TimeZone UTC = TimeZone.getTimeZone("UTC");

  @Test
  public void shouldChainPowerOfTenBeyondScaleNine() {
    assertEquals(1L, ArrowResultUtil.powerOfTen(0));
    assertEquals(1_000_000_000L, ArrowResultUtil.powerOfTen(9));
    // scale > 9 must chain (10^9 * 10^(pow-9)) rather than index out of POWERS_OF_10.
    assertEquals(10_000_000_000L, ArrowResultUtil.powerOfTen(10));
    assertEquals(1_000_000_000_000_000_000L, ArrowResultUtil.powerOfTen(18));
  }

  @Test
  public void shouldDecomposeScaledEpochIntoSecondsAndNanos() {
    // scale 0: whole seconds, no fraction.
    Timestamp scale0 = ArrowResultUtil.toJavaTimestamp(1546391837L, 0);
    assertEquals(1546391837000L, scale0.getTime());
    assertEquals(0, scale0.getNanos());

    // scale 9: 1546391837.000000001 → seconds=1546391837, fraction=1ns.
    Timestamp scale9 = ArrowResultUtil.toJavaTimestamp(1546391837000000001L, 9);
    assertEquals(1546391837000L, scale9.getTime());
    assertEquals(1, scale9.getNanos());
  }

  @Test
  public void shouldNormalizeNegativeEpochFraction() {
    // -0.000000001s at scale 9 → seconds=-1, fraction=999999999 (fraction kept in [0,10^9)).
    // Timestamp.getTime() folds the 999ms component of the nanos back in: -1000 + 999 = -1.
    Timestamp tiny = ArrowResultUtil.toJavaTimestamp(-1L, 9);
    assertEquals(-1L, tiny.getTime());
    assertEquals(999_999_999, tiny.getNanos());

    // -1232.234s at scale 3 → seconds=-1233, fraction=766000000 (plan's worked example).
    // getTime() = -1233000 + 766 = -1232234.
    Timestamp neg = ArrowResultUtil.toJavaTimestamp(-1232234L, 3);
    assertEquals(-1232234L, neg.getTime());
    assertEquals(766_000_000, neg.getNanos());
  }

  @Test
  public void shouldCreatePlainTimestampWhenNotUsingSessionTimezone() {
    Timestamp ts = ArrowResultUtil.createTimestamp(1546391837L, 123_000_000, UTC, false);
    assertEquals(Timestamp.class, ts.getClass());
    // getTime() folds the 123ms from the nanos back in: 1546391837000 + 123.
    assertEquals(1546391837123L, ts.getTime());
    assertEquals(123_000_000, ts.getNanos());
  }

  @Test
  public void shouldCreateTimezoneCarryingTimestampWhenUsingSessionTimezone() {
    TimeZone ny = TimeZone.getTimeZone("America/New_York");
    Timestamp ts = ArrowResultUtil.createTimestamp(1546391837L, 123_000_000, ny, true);
    assertInstanceOf(SnowflakeTimestampWithTimezone.class, ts);
    assertEquals(ny, ((SnowflakeTimestampWithTimezone) ts).getTimezone());
    assertEquals(1546391837123L, ts.getTime());
    assertEquals(123_000_000, ts.getNanos());
  }

  @Test
  public void shouldReturnSameTimestampWhenMovingBetweenZonesWithSameRules() {
    Timestamp ts = new Timestamp(1546391837000L);
    ts.setNanos(123_456_789);
    assertSame(ts, ArrowResultUtil.moveToTimeZone(ts, UTC, TimeZone.getTimeZone("UTC")));
  }

  @Test
  public void shouldShiftByOffsetDifferenceAndPreserveNanosWhenMovingZones() {
    Timestamp ts = new Timestamp(1546391837000L);
    ts.setNanos(123_456_789);
    // ts.getTime() == 1546391837123 (123ms from nanos). UTC (offset 0) → GMT+05:00 (+5h): shift
    // = 0 - 18000000 = -18000000 ms → 1546373837123.
    Timestamp moved = ArrowResultUtil.moveToTimeZone(ts, UTC, TimeZone.getTimeZone("GMT+05:00"));
    assertEquals(1546373837123L, moved.getTime());
    assertEquals(123_456_789, moved.getNanos());
  }

  @Test
  public void shouldDetectTimestampOverflow() {
    assertFalse(ArrowResultUtil.isTimestampOverflow(0L));
    assertFalse(ArrowResultUtil.isTimestampOverflow(1546391837L));
    assertTrue(ArrowResultUtil.isTimestampOverflow(Long.MAX_VALUE));
    assertTrue(ArrowResultUtil.isTimestampOverflow(Long.MIN_VALUE));
  }
}
