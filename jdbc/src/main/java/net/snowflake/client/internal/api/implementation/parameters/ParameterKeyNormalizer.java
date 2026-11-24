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
    LEGACY_KEY_ALIASES.put("workloadidentityprovider", "workload_identity_provider");
    LEGACY_KEY_ALIASES.put("workloadidentityentraresource", "workload_identity_entra_resource");
    LEGACY_KEY_ALIASES.put(
        "workloadidentityimpersonationpath", "workload_identity_impersonation_path");
    LEGACY_KEY_ALIASES.put(
        "workloadidentityawsuseoutboundtoken", "workload_identity_aws_use_outbound_token");
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
    LEGACY_KEY_ALIASES.put("platformdetectiontimeoutms", "platform_detection_timeout_ms");
    LEGACY_KEY_ALIASES.put("disableplatformdetection", "disable_platform_detection");
    LEGACY_KEY_ALIASES.put("enablediagnostics", "enable_connection_diag");
    LEGACY_KEY_ALIASES.put("enable_diagnostics", "enable_connection_diag");
    LEGACY_KEY_ALIASES.put("diagnosticsallowlistfile", "connection_diag_allowlist_path");
    LEGACY_KEY_ALIASES.put("diagnostics_allowlist_file", "connection_diag_allowlist_path");
    LEGACY_KEY_ALIASES.put("browser_response_timeout", "authentication_timeout");
    // sf_core reads crl_*. In 4.x only CERT_REVOCATION_CHECK_MODE and
    // ALLOW_CERTIFICATES_WITHOUT_CRL_URL were connection properties; the other four were JVM -D
    // flags. The camelCase spellings are extra aliases, not 4.x names.
    LEGACY_KEY_ALIASES.put("cert_revocation_check_mode", "crl_check_mode");
    LEGACY_KEY_ALIASES.put("certrevocationcheckmode", "crl_check_mode");
    LEGACY_KEY_ALIASES.put(
        "allow_certificates_without_crl_url", "crl_allow_certificates_without_crl_url");
    LEGACY_KEY_ALIASES.put(
        "allowcertificateswithoutcrlurl", "crl_allow_certificates_without_crl_url");
    LEGACY_KEY_ALIASES.put("enable_crl_in_memory_caching", "crl_enable_memory_caching");
    LEGACY_KEY_ALIASES.put("enablecrlinmemorycaching", "crl_enable_memory_caching");
    LEGACY_KEY_ALIASES.put("enable_crl_disk_caching", "crl_enable_disk_caching");
    LEGACY_KEY_ALIASES.put("enablecrldiskcaching", "crl_enable_disk_caching");
    LEGACY_KEY_ALIASES.put("crl_cache_validity_time", "crl_validity_time");
    LEGACY_KEY_ALIASES.put("crlcachevaliditytime", "crl_validity_time");
    LEGACY_KEY_ALIASES.put("crl_response_cache_dir", "crl_cache_dir");
    LEGACY_KEY_ALIASES.put("crlresponsecachedir", "crl_cache_dir");
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
