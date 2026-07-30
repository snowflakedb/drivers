package net.snowflake.client.internal.api.implementation.parameters;

import lombok.AccessLevel;
import lombok.NoArgsConstructor;

/**
 * Adjusts legacy JDBC connection property <em>values</em> to the representation sf_core expects,
 * after {@link ParameterKeyNormalizer} has mapped the property key to its canonical name.
 *
 * <p>Most legacy values pass through unchanged; only the few whose encoding differs between the old
 * driver and sf_core are translated here (e.g. legacy pipe-delimited {@code nonProxyHosts}).
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class ParameterValueNormalizer {

  private static final String NO_PROXY = "no_proxy";

  /**
   * Normalizes a property value for the given canonical (already key-normalized) property. Returns
   * the value unchanged when no translation applies.
   *
   * @param canonicalKey the sf_core canonical key (output of {@link ParameterKeyNormalizer})
   * @param value the property value as supplied by the caller
   */
  public static Object normalize(String canonicalKey, Object value) {
    if (NO_PROXY.equals(canonicalKey) && value instanceof String) {
      return normalizeNoProxy((String) value);
    }
    return value;
  }

  /**
   * Translates a legacy {@code nonProxyHosts} value to the form sf_core's reqwest-based {@code
   * NoProxy::from_string} understands.
   *
   * <p>Two legacy conventions are converted: (1) the Java {@code http.nonProxyHosts} pipe delimiter
   * ({@code host1|host2}) becomes the comma separation reqwest expects; (2) the Java {@code *.host}
   * subdomain glob becomes the leading-dot {@code .host} form reqwest recognizes. reqwest only
   * honors a bare {@code *} as a wildcard, so a standalone {@code *} entry is preserved. Note
   * reqwest matches {@code .host} against the apex host as well as its subdomains, a minor
   * over-match versus the Java glob's subdomain-only semantics (reqwest has no subdomain-only
   * form).
   */
  private static String normalizeNoProxy(String value) {
    String[] entries = value.replace('|', ',').split(",", -1);
    StringBuilder result = new StringBuilder(value.length());
    for (int i = 0; i < entries.length; i++) {
      if (i > 0) {
        result.append(',');
      }
      String entry = entries[i].trim();
      if (entry.startsWith("*.")) {
        entry = entry.substring(1); // "*.foo.com" -> ".foo.com"
      }
      result.append(entry);
    }
    return result.toString();
  }
}
