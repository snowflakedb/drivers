package net.snowflake.client.internal.util;

public class HexUtil {
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
