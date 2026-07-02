package net.snowflake.jdbc.utils;

/**
 * Utility for detecting whether tests are running against the new (universal) driver or the old
 * (legacy) JDBC driver. Used to conditionally skip or branch test logic for known behavioral
 * differences between the two drivers.
 *
 * <p>Detection is based on the presence of a class unique to the universal driver
 * (net.snowflake.client.internal.unicore.CoreDriverApi). When running reference tests against the
 * old driver JAR, this class will not be on the classpath.
 */
public final class DriverCompatibility {

  private static final boolean IS_UNIVERSAL_DRIVER = detectUniversalDriver();

  private DriverCompatibility() {}

  public static boolean isNewDriver() {
    return IS_UNIVERSAL_DRIVER;
  }

  public static boolean isOldDriver() {
    return !IS_UNIVERSAL_DRIVER;
  }

  private static boolean detectUniversalDriver() {
    try {
      Class.forName("net.snowflake.client.internal.unicore.CoreDriverApi");
      return true;
    } catch (ClassNotFoundException e) {
      return false;
    }
  }
}
