package net.snowflake.client.internal.unicore;

import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;

/**
 * Maps legacy JDBC connection property names onto the canonical names that sf_core understands.
 *
 * <p>These aliases preserve compatibility with the old Snowflake JDBC driver, which exposed several
 * properties under camelCase names that differ from sf_core's snake_case canonical names (e.g.
 * {@code oktausername} → {@code okta_username}).
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class LegacyKeyNormalizer {

  /** Keys are stored lowercased; lookups are case-insensitive (JDBC accepts props in any case). */
  private static final Map<String, String> LEGACY_KEY_ALIASES;

  static {
    LEGACY_KEY_ALIASES = new HashMap<>();
    LEGACY_KEY_ALIASES.put("db", "database");
    LEGACY_KEY_ALIASES.put("privatekey", "private_key");
    LEGACY_KEY_ALIASES.put("oktausername", "okta_username");
    LEGACY_KEY_ALIASES.put("logintimeout", "authentication_timeout");
    LEGACY_KEY_ALIASES.put("disablesamlurlcheck", "disable_saml_url_check");
    LEGACY_KEY_ALIASES.put("oauthclientid", "oauth_client_id");
    LEGACY_KEY_ALIASES.put("oauthclientsecret", "oauth_client_secret");
    LEGACY_KEY_ALIASES.put("oauthauthorizationurl", "oauth_authorization_url");
    LEGACY_KEY_ALIASES.put("oauthtokenrequesturl", "oauth_token_request_url");
    LEGACY_KEY_ALIASES.put("oauthredirecturi", "oauth_redirect_uri");
    LEGACY_KEY_ALIASES.put("oauthscope", "oauth_scope");
  }

  /**
   * Normalizes a JDBC connection property key before sending to sf_core, mapping legacy camelCase
   * property names the old driver exposed onto sf_core's canonical names. Unknown keys are returned
   * unchanged.
   */
  public static String normalize(String key) {
    String alias = LEGACY_KEY_ALIASES.get(key.toLowerCase(Locale.ROOT));
    if (alias != null) {
      return alias;
    }
    return key;
  }
}
