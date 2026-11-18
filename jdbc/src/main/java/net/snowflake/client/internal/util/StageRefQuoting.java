package net.snowflake.client.internal.util;

/** Quotes a stage reference for PUT SQL when Snowflake file-operation syntax requires it. */
public class StageRefQuoting {
  private static final String UNQUOTED_CHARS = "_$./@~%\"-:";

  private StageRefQuoting() {}

  /**
   * Wraps {@code stageRef} in single quotes when it contains a character outside ASCII
   * alphanumerics and {@code _$./@~%"-:}. Embedded single quotes are doubled. A reference already
   * wrapped in single quotes or dollar quotes is returned unchanged. {@code null} stays {@code
   * null}.
   */
  public static String quoteIfNeeded(String stageRef) {
    if (stageRef == null || !needsQuoting(stageRef)) {
      return stageRef;
    }
    return "'" + stageRef.replace("'", "''") + "'";
  }

  /**
   * Prefixes {@code @} to a stage reference that lacks one, keeping any outer single-quote or
   * dollar-quote wrapper in place. A reference that already starts with {@code @} is returned
   * unchanged, and {@code null} stays {@code null}.
   */
  public static String withAtPrefix(String stageRef) {
    if (stageRef == null) {
      return null;
    }
    String inner = unwrapOuterQuotes(stageRef);
    if (inner.startsWith("@")) {
      return stageRef;
    }
    String prefixed = "@" + inner;
    if (isDollarQuoted(stageRef)) {
      return "$$" + prefixed + "$$";
    }
    if (isSingleQuoted(stageRef)) {
      return "'" + prefixed.replace("'", "''") + "'";
    }
    return prefixed;
  }

  /**
   * Removes an outer single-quote or dollar-quote wrapper so a path suffix can be appended inside
   * the reference. Doubled single quotes inside a single-quoted wrapper become one quote. A
   * reference that is not wrapped, including a malformed quote such as {@code 'a'b'c'}, is returned
   * unchanged. {@code null} stays {@code null}.
   */
  public static String unwrapOuterQuotes(String stageRef) {
    if (stageRef == null) {
      return null;
    }
    if (isDollarQuoted(stageRef)) {
      return stageRef.substring(2, stageRef.length() - 2);
    }
    if (isSingleQuoted(stageRef)) {
      return stageRef.substring(1, stageRef.length() - 1).replace("''", "'");
    }
    return stageRef;
  }

  private static boolean needsQuoting(String stageRef) {
    if (isDollarQuoted(stageRef) || isSingleQuoted(stageRef)) {
      return false;
    }
    for (int i = 0; i < stageRef.length(); i++) {
      if (!isUnquotedChar(stageRef.charAt(i))) {
        return true;
      }
    }
    return false;
  }

  private static boolean isUnquotedChar(char c) {
    return (c >= 'A' && c <= 'Z')
        || (c >= 'a' && c <= 'z')
        || (c >= '0' && c <= '9')
        || UNQUOTED_CHARS.indexOf(c) >= 0;
  }

  private static boolean isDollarQuoted(String stageRef) {
    return stageRef.length() >= 4 && stageRef.startsWith("$$") && stageRef.endsWith("$$");
  }

  private static boolean isSingleQuoted(String stageRef) {
    int length = stageRef.length();
    if (length < 2 || stageRef.charAt(0) != '\'' || stageRef.charAt(length - 1) != '\'') {
      return false;
    }
    String content = stageRef.substring(1, length - 1);
    for (int i = 0; i < content.length(); i++) {
      if (content.charAt(i) != '\'') {
        continue;
      }
      if (i + 1 >= content.length() || content.charAt(i + 1) != '\'') {
        return false;
      }
      i++;
    }
    return true;
  }
}
