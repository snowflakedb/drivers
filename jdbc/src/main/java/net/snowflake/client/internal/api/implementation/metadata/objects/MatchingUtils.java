package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.util.regex.Pattern;
import lombok.experimental.UtilityClass;

@UtilityClass
class MatchingUtils {

  static boolean matches(Pattern nullablePattern, String input) {
    return nullablePattern == null || (input != null && nullablePattern.matcher(input).matches());
  }

  static boolean isPatternMatchingAll(String pattern) {
    return pattern == null
        || pattern.isEmpty()
        || pattern.trim().equals("%")
        || pattern.trim().equals(".*");
  }
}
