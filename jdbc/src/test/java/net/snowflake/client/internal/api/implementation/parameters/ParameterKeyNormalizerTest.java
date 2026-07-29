package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class ParameterKeyNormalizerTest {

  @Test
  void shouldMapLegacyDatabase() {
    assertEquals("database", ParameterKeyNormalizer.normalize("db"));
  }

  @Test
  void shouldMapLegacyPrivateKey() {
    assertEquals("private_key", ParameterKeyNormalizer.normalize("privateKey"));
  }

  @Test
  void shouldMapLegacyOktaUsername() {
    assertEquals("okta_username", ParameterKeyNormalizer.normalize("oktausername"));
  }

  @Test
  void shouldMapLegacyLoginTimeout() {
    assertEquals("login_timeout", ParameterKeyNormalizer.normalize("loginTimeout"));
  }

  @Test
  void shouldMapLegacyBrowserResponseTimeout() {
    assertEquals(
        "authentication_timeout", ParameterKeyNormalizer.normalize("BROWSER_RESPONSE_TIMEOUT"));
  }

  @Test
  void shouldMapLegacyDisableSamlUrlCheck() {
    assertEquals("disable_saml_url_check", ParameterKeyNormalizer.normalize("disableSamlURLCheck"));
  }

  @Test
  void shouldNormalizeCaseInsensitively() {
    assertEquals("login_timeout", ParameterKeyNormalizer.normalize("LOGINTIMEOUT"));
    assertEquals("okta_username", ParameterKeyNormalizer.normalize("OktaUserName"));
  }

  @Test
  void shouldMapLegacyOauthProperties() {
    assertEquals("oauth_client_id", ParameterKeyNormalizer.normalize("oauthClientId"));
    assertEquals("oauth_client_secret", ParameterKeyNormalizer.normalize("oauthClientSecret"));
    assertEquals(
        "oauth_authorization_url", ParameterKeyNormalizer.normalize("oauthAuthorizationUrl"));
    assertEquals(
        "oauth_token_request_url", ParameterKeyNormalizer.normalize("oauthTokenRequestUrl"));
    assertEquals("oauth_redirect_uri", ParameterKeyNormalizer.normalize("oauthRedirectUri"));
    assertEquals("oauth_scope", ParameterKeyNormalizer.normalize("oauthScope"));
  }

  @Test
  void shouldLeaveUnknownKeysUnchanged() {
    assertEquals("authenticator", ParameterKeyNormalizer.normalize("authenticator"));
  }

  @Test
  void shouldNotAliasClientStoreTemporaryCredential() {
    // sf_core already aliases this key, so the normalizer must leave it untouched (see the
    // MFA/credential-cache note in ParameterKeyNormalizer).
    assertEquals(
        "clientStoreTemporaryCredential",
        ParameterKeyNormalizer.normalize("clientStoreTemporaryCredential"));
  }

  @Test
  void shouldMapLegacyDataSourcePropertyKeys() {
    assertAll(
        () ->
            assertEquals(
                "preserve_underscores_in_hostname",
                ParameterKeyNormalizer.normalize("allowUnderscoresInHost")),
        () ->
            assertEquals("query_timeout", ParameterKeyNormalizer.normalize("queryTimeoutSeconds")),
        () -> assertEquals("query_timeout", ParameterKeyNormalizer.normalize("queryTimeout")),
        () ->
            assertEquals("retry_max_attempts", ParameterKeyNormalizer.normalize("maxHttpRetries")),
        () ->
            assertEquals(
                "put_get_max_attempts", ParameterKeyNormalizer.normalize("putGetMaxRetries")),
        () -> assertEquals("proxy_host", ParameterKeyNormalizer.normalize("proxyHost")),
        () -> assertEquals("proxy_port", ParameterKeyNormalizer.normalize("proxyPort")),
        () -> assertEquals("proxy_user", ParameterKeyNormalizer.normalize("proxyUser")),
        () -> assertEquals("proxy_password", ParameterKeyNormalizer.normalize("proxyPassword")),
        () -> assertEquals("no_proxy", ParameterKeyNormalizer.normalize("nonProxyHosts")),
        () ->
            assertEquals(
                "enable_connection_diag", ParameterKeyNormalizer.normalize("enableDiagnostics")),
        () ->
            assertEquals(
                "connection_diag_allowlist_path",
                ParameterKeyNormalizer.normalize("diagnosticsAllowlistFile")));
  }
}
