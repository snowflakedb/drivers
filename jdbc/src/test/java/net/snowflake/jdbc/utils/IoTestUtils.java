package net.snowflake.jdbc.utils;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;

public final class IoTestUtils {
  private IoTestUtils() {}

  /** Java 8 stand-in for {@code InputStream.readAllBytes()}. */
  public static byte[] readAllBytes(InputStream in) throws IOException {
    ByteArrayOutputStream buf = new ByteArrayOutputStream();
    byte[] chunk = new byte[4096];
    int n;
    while ((n = in.read(chunk)) != -1) {
      buf.write(chunk, 0, n);
    }
    return buf.toByteArray();
  }
}
