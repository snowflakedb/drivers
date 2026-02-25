package net.snowflake.client.utils;

import java.util.Random;

public class RandomStringUtils {
  public static String randomAlphaNumeric(int length) {
    String symbols = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    Random random = new Random();
    StringBuilder sb = new StringBuilder(length);
    for (int i = 0; i < length; i++) {
      sb.append(symbols.charAt(random.nextInt(symbols.length())));
    }
    return sb.toString();
  }
}
