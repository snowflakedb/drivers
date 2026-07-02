package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import java.math.RoundingMode;
import java.time.Duration;
import lombok.experimental.UtilityClass;

@UtilityClass
public class ArrowResultUtil {
  private static final int[] POWERS_OF_10 = {
    1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000
  };

  public static final int MAX_SCALE_POWERS_OF_10 = 9;

  private static final BigDecimal NANO_IN_SECOND = BigDecimal.valueOf(1_000_000_000);

  public static long powerOfTen(int pow) {
    long val = 1;
    while (pow > MAX_SCALE_POWERS_OF_10) {
      val *= POWERS_OF_10[MAX_SCALE_POWERS_OF_10];
      pow -= MAX_SCALE_POWERS_OF_10;
    }
    return val * POWERS_OF_10[pow];
  }

  public static String getStringFormat(int scale) {
    return "%." + scale + 'f';
  }

  public static Duration getDurationFromNanos(BigDecimal numNanos) {
    int sign = numNanos.signum();
    numNanos = numNanos.abs();
    // Duration.ofSeconds overflows on negative second values, so convert the magnitude and
    // re-apply the sign via negated().
    Duration duration =
        Duration.ofSeconds(
            numNanos.divide(NANO_IN_SECOND, RoundingMode.FLOOR).longValueExact(),
            numNanos.remainder(NANO_IN_SECOND).longValueExact());
    return sign >= 0 ? duration : duration.negated();
  }
}
