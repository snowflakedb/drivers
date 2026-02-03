package net.snowflake.client.internal.core.arrow;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Random;
import net.snowflake.client.api.exception.SFException;
import org.junit.jupiter.api.function.Executable;

public final class TestHelper {
  private static final char[] ALPHANUM =
      "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".toCharArray();

  private TestHelper() {}

  public static void assertSFException(int expectedErrorCode, Executable executable) {
    SFException ex = assertThrows(SFException.class, executable);
    assertEquals(expectedErrorCode, ex.getErrorCode().getMessageCode());
  }

  public static String randomString(Random random, int length) {
    char[] chars = new char[length];
    for (int i = 0; i < length; i++) {
      chars[i] = ALPHANUM[random.nextInt(ALPHANUM.length)];
    }
    return new String(chars);
  }
}
