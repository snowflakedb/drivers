package net.snowflake.client.internal.api.implementation.parameters;

import lombok.AccessLevel;
import lombok.Getter;
import lombok.RequiredArgsConstructor;

@Getter
@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
public enum SessionProperty implements Property {
  // Keys mirror legacy snowflake-jdbc DataSource / connection-property names for connect-time
  // normalization via ParameterKeyNormalizer. camelCase entries (e.g. proxyHost, loginTimeout)
  // match legacy DataSource bean setters; snake_case entries (e.g. disable_saml_url_check,
  // browser_response_timeout) match legacy connection-property names that were already stored
  // that way on the reference driver.

  // Connection endpoint and session context
  URL("url"),
  USER("user"),
  PASSWORD("password"),
  ACCOUNT("account"),
  DATABASE("database"),
  SCHEMA("schema"),
  ROLE("role"),
  WAREHOUSE("warehouse"),

  // Transport
  SSL("ssl"),

  // Authentication
  AUTHENTICATOR("authenticator"),
  TOKEN("token"),
  PRIVATE_KEY_FILE("private_key_file"),
  PRIVATE_KEY("private_key"),
  PRIVATE_KEY_PASSWORD("private_key_password"),
  PASSCODE("passcode"),
  PASSCODE_IN_PASSWORD("passcodeInPassword"),
  OKTA_USERNAME("okta_username"),
  DISABLE_SAML_URL_CHECK("disable_saml_url_check"),
  CLIENT_STORE_TEMPORARY_CREDENTIAL("clientStoreTemporaryCredential"),

  // OAuth configuration
  OAUTH_CLIENT_ID("oauth_client_id"),
  OAUTH_CLIENT_SECRET("oauth_client_secret"),
  OAUTH_AUTHORIZATION_URL("oauth_authorization_url"),
  OAUTH_TOKEN_REQUEST_URL("oauth_token_request_url"),
  OAUTH_REDIRECT_URI("oauth_redirect_uri"),
  OAUTH_SCOPE("oauth_scope"),
  OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS("oauth_enable_single_use_refresh_tokens"),

  // Application identity
  APPLICATION("application"),

  // Timeouts and retries
  LOGIN_TIMEOUT("loginTimeout"),
  QUERY_TIMEOUT_SECONDS("queryTimeoutSeconds"),
  MAX_HTTP_RETRIES("maxHttpRetries"),
  PUT_GET_MAX_RETRIES("putGetMaxRetries"),
  BROWSER_RESPONSE_TIMEOUT("browser_response_timeout"),

  // Hostname handling
  ALLOW_UNDERSCORES_IN_HOST("allowUnderscoresInHost"),

  // Proxy
  PROXY_HOST("proxyHost"),
  PROXY_PORT("proxyPort"),
  PROXY_USER("proxyUser"),
  PROXY_PASSWORD("proxyPassword"),
  NON_PROXY_HOSTS("nonProxyHosts"),

  // Connection diagnostics
  ENABLE_DIAGNOSTICS("enableDiagnostics"),
  DIAGNOSTICS_ALLOWLIST_FILE("diagnosticsAllowlistFile");

  private final String key;
}
