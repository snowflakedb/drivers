package net.snowflake.client.internal.api.implementation.parameters;

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
    assertEquals("authentication_timeout", ParameterKeyNormalizer.normalize("loginTimeout"));
  }

  @Test
  void shouldMapLegacyDisableSamlUrlCheck() {
    assertEquals("disable_saml_url_check", ParameterKeyNormalizer.normalize("disableSamlURLCheck"));
  }

  @Test
  void shouldNormalizeCaseInsensitively() {
    assertEquals("authentication_timeout", ParameterKeyNormalizer.normalize("LOGINTIMEOUT"));
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
}
