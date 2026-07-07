package net.snowflake.client.internal.util;

public class StringUtil {
  public static boolean isNullOrEmpty(String value) {
    return value == null || value.isEmpty();
  }

  public static String nullIfEmpty(String value) {
    return isNullOrEmpty(value) ? null : value;
  }
}
