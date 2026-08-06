package net.snowflake.client.internal.api.implementation.parameters;

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
public final class ParameterKeyNormalizer {

  /** Keys are stored lowercased; lookups are case-insensitive (JDBC accepts props in any case). */
  private static final Map<String, String> LEGACY_KEY_ALIASES;

  static {
    LEGACY_KEY_ALIASES = new HashMap<>();
    LEGACY_KEY_ALIASES.put("db", "database");
    LEGACY_KEY_ALIASES.put("privatekey", "private_key");
    LEGACY_KEY_ALIASES.put("oktausername", "okta_username");
    LEGACY_KEY_ALIASES.put("disablesamlurlcheck", "disable_saml_url_check");
    LEGACY_KEY_ALIASES.put("oauthclientid", "oauth_client_id");
    LEGACY_KEY_ALIASES.put("oauthclientsecret", "oauth_client_secret");
    LEGACY_KEY_ALIASES.put("oauthauthorizationurl", "oauth_authorization_url");
    LEGACY_KEY_ALIASES.put("oauthtokenrequesturl", "oauth_token_request_url");
    LEGACY_KEY_ALIASES.put("oauthredirecturi", "oauth_redirect_uri");
    LEGACY_KEY_ALIASES.put("oauthscope", "oauth_scope");
    LEGACY_KEY_ALIASES.put("logintimeout", "login_timeout");
    LEGACY_KEY_ALIASES.put("allowunderscoresinhost", "preserve_underscores_in_hostname");
    LEGACY_KEY_ALIASES.put("querytimeoutseconds", "query_timeout");
    LEGACY_KEY_ALIASES.put("querytimeout", "query_timeout");
    LEGACY_KEY_ALIASES.put("maxhttpretries", "retry_max_attempts");
    LEGACY_KEY_ALIASES.put("putgetmaxretries", "put_get_max_attempts");
    LEGACY_KEY_ALIASES.put("proxyhost", "proxy_host");
    LEGACY_KEY_ALIASES.put("proxyport", "proxy_port");
    LEGACY_KEY_ALIASES.put("proxyuser", "proxy_user");
    LEGACY_KEY_ALIASES.put("proxypassword", "proxy_password");
    LEGACY_KEY_ALIASES.put("nonproxyhosts", "no_proxy");
    LEGACY_KEY_ALIASES.put("enablediagnostics", "enable_connection_diag");
    LEGACY_KEY_ALIASES.put("diagnosticsallowlistfile", "connection_diag_allowlist_path");
    LEGACY_KEY_ALIASES.put("browser_response_timeout", "authentication_timeout");
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
