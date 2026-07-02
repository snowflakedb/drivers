package net.snowflake.jdbc.e2e.parity;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * A named overlay of session parameters applied via ALTER SESSION on top of (tz, output-format).
 * Drives full Cartesian coverage of the boolean date/time flags on snowflake-jdbc.
 *
 * <p>Stores entries in a deterministic order so the generated ALTER SESSION clauses are stable
 * (helps memoization in {@link ParitySession}).
 */
final class Profile {

  /** Default-everything profile. Equivalent to "no overrides". */
  static final Profile DEFAULT = new Profile("default", Collections.emptyMap());

  private final String name;
  private final Map<String, String> overlay;

  private Profile(String name, Map<String, String> overlay) {
    this.name = name;
    this.overlay = Collections.unmodifiableMap(new TreeMap<>(overlay));
  }

  String name() {
    return name;
  }

  Map<String, String> overlay() {
    return overlay;
  }

  /**
   * Build the full Cartesian product of true/false values for the given boolean parameter names.
   * Each combination becomes a Profile whose name encodes the flag values (e.g. {@code
   * "USE_SESSION_TZ=t,TREAT_NTZ_UTC=f"}).
   */
  static List<Profile> booleanCartesian(List<String> paramNames) {
    if (paramNames.isEmpty()) {
      return Collections.singletonList(DEFAULT);
    }
    List<Profile> out = new ArrayList<>();
    int total = 1 << paramNames.size();
    for (int mask = 0; mask < total; mask++) {
      Map<String, String> entries = new LinkedHashMap<>();
      StringBuilder name = new StringBuilder();
      for (int bit = 0; bit < paramNames.size(); bit++) {
        boolean value = ((mask >> bit) & 1) == 1;
        String param = paramNames.get(bit);
        entries.put(param, Boolean.toString(value));
        if (name.length() > 0) {
          name.append(',');
        }
        name.append(shortName(param)).append('=').append(value ? 't' : 'f');
      }
      out.add(new Profile(name.toString(), entries));
    }
    return out;
  }

  private static String shortName(String fullName) {
    // Compact display name. Keep the suffix that disambiguates between flags so it stays
    // readable in @ParameterizedTest names.
    return fullName
        .replace("CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ", "HONOR_CLIENT_TZ")
        .replace("JDBC_USE_SESSION_TIMEZONE", "USE_SESSION_TZ")
        .replace("JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC", "TREAT_NTZ_UTC")
        .replace("JDBC_FORMAT_DATE_WITH_TIMEZONE", "DATE_WITH_TZ")
        .replace("JDBC_GET_DATE_USE_NULL_TIMEZONE", "DATE_NULL_TZ");
  }

  @Override
  public String toString() {
    return name;
  }
}
