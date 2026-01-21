package net.snowflake.client.internal.core.arrow;

public class ArrowResultUtil {
  private static final int[] POWERS_OF_10 = {
    1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000
  };

  public static final int MAX_SCALE_POWERS_OF_10 = 9;

  public static long powerOfTen(int pow) {
    long val = 1;
    while (pow > MAX_SCALE_POWERS_OF_10) {
      val *= POWERS_OF_10[MAX_SCALE_POWERS_OF_10];
      pow -= MAX_SCALE_POWERS_OF_10;
    }
    return val * POWERS_OF_10[pow];
  }

  public static String getStringFormat(int scale) {
    return new StringBuilder().append("%.").append(scale).append('f').toString();
  }

  private ArrowResultUtil() {}
}
