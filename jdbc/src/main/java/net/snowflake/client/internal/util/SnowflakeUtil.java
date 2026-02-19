package net.snowflake.client.internal.util;

public class SnowflakeUtil {
  public static final String BIG_DECIMAL_STR = "big decimal";
  public static final String FLOAT_STR = "float";
  public static final String DOUBLE_STR = "double";
  public static final String BOOLEAN_STR = "boolean";
  public static final String SHORT_STR = "short";
  public static final String INT_STR = "int";
  public static final String LONG_STR = "long";
  public static final String TIME_STR = "time";
  public static final String TIMESTAMP_STR = "timestamp";
  public static final String DATE_STR = "date";
  public static final String BYTE_STR = "byte";
  public static final String BYTES_STR = "byte array";

  private SnowflakeUtil() {}

  public static String bytesToHex(byte[] value) {
    if (value == null) {
      return null;
    }
    char[] hexArray = "0123456789ABCDEF".toCharArray();
    char[] hexChars = new char[value.length * 2];
    for (int j = 0; j < value.length; j++) {
      int v = value[j] & 0xFF;
      hexChars[j * 2] = hexArray[v >>> 4];
      hexChars[j * 2 + 1] = hexArray[v & 0x0F];
    }
    return new String(hexChars);
  }
}
