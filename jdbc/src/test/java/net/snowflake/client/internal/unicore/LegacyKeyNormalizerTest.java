package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class LegacyKeyNormalizerTest {

  @Test
  void shouldMapLegacyPrivateKey() {
    assertEquals("private_key", LegacyKeyNormalizer.normalize("privateKey"));
  }

  @Test
  void shouldMapLegacyOktaUsername() {
    assertEquals("okta_username", LegacyKeyNormalizer.normalize("oktausername"));
  }

  @Test
  void shouldMapLegacyLoginTimeout() {
    assertEquals("authentication_timeout", LegacyKeyNormalizer.normalize("loginTimeout"));
  }

  @Test
  void shouldMapLegacyDisableSamlUrlCheck() {
    assertEquals("disable_saml_url_check", LegacyKeyNormalizer.normalize("disableSamlURLCheck"));
  }

  @Test
  void shouldNormalizeCaseInsensitively() {
    assertEquals("authentication_timeout", LegacyKeyNormalizer.normalize("LOGINTIMEOUT"));
    assertEquals("okta_username", LegacyKeyNormalizer.normalize("OktaUserName"));
  }

  @Test
  void shouldLeaveUnknownKeysUnchanged() {
    assertEquals("authenticator", LegacyKeyNormalizer.normalize("authenticator"));
  }
}
