package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.sql.Timestamp;
import org.junit.jupiter.api.Test;

/** Unit tests for {@link ArrowDateUtil#adjustTimestamp} (the Julian→Gregorian timestamp shift). */
public class ArrowDateUtilTest {

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
}
