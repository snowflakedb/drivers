package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import org.junit.jupiter.api.Test;

class ParameterValueNormalizerTest {

  @Test
  void shouldTranslatePipeDelimitedNoProxyToComma() {
    assertEquals(
        "host1,host2,.example.com",
        ParameterValueNormalizer.normalize("no_proxy", "host1|host2|.example.com"));
  }

  @Test
  void shouldLeaveCommaDelimitedNoProxyUnchanged() {
    assertEquals("host1,host2", ParameterValueNormalizer.normalize("no_proxy", "host1,host2"));
  }

  @Test
  void shouldLeaveSingleNoProxyHostUnchanged() {
    assertEquals("host1", ParameterValueNormalizer.normalize("no_proxy", "host1"));
  }

  @Test
  void shouldTranslateLegacySubdomainWildcardToLeadingDot() {
    assertEquals(
        ".foo.com,.bar.com,localhost",
        ParameterValueNormalizer.normalize("no_proxy", "*.foo.com|*.bar.com|localhost"));
  }

  @Test
  void shouldPreserveBareWildcardEntry() {
    assertEquals("*", ParameterValueNormalizer.normalize("no_proxy", "*"));
  }

  @Test
  void shouldNotTranslateValuesForOtherKeys() {
    assertEquals("a|b", ParameterValueNormalizer.normalize("proxy_host", "a|b"));
  }

  @Test
  void shouldTranslateLegacyTlsProtocolNamesToCoreForm() {
    assertEquals("tls12", ParameterValueNormalizer.normalize("min_tls_version", "TLSv1.2"));
    assertEquals("tls13", ParameterValueNormalizer.normalize("max_tls_version", "TLSv1.3"));
    assertEquals("tls12", ParameterValueNormalizer.normalize("MIN_TLS_VERSION", "TLSv1.2"));
  }

  @Test
  void shouldLeaveCoreTlsVersionValuesUnchanged() {
    assertEquals("tls12", ParameterValueNormalizer.normalize("min_tls_version", "tls12"));
    assertEquals("tls13", ParameterValueNormalizer.normalize("max_tls_version", "tls13"));
  }

  @Test
  void shouldRoundLegacyHttpClientTimeoutsUpToWholeSeconds() {
    assertAll(
        () ->
            assertEquals(
                60L, ParameterValueNormalizer.normalize("HTTP_CLIENT_CONNECTION_TIMEOUT", "60000")),
        () ->
            assertEquals(
                300L, ParameterValueNormalizer.normalize("HTTP_CLIENT_SOCKET_TIMEOUT", 300000)),
        () ->
            assertEquals(
                1L, ParameterValueNormalizer.normalize("HTTP_CLIENT_SOCKET_TIMEOUT", "10")),
        () ->
            assertEquals(
                2L, ParameterValueNormalizer.normalize("HTTP_CLIENT_CONNECTION_TIMEOUT", "1001")),
        () ->
            assertEquals(
                0L, ParameterValueNormalizer.normalize("HTTP_CLIENT_CONNECTION_TIMEOUT", 0)),
        () ->
            assertEquals(
                "abc",
                ParameterValueNormalizer.normalize("HTTP_CLIENT_CONNECTION_TIMEOUT", "abc")));
  }

  @Test
  void shouldRoundLegacyNetworkTimeoutUpToWholeSeconds() {
    assertAll(
        () -> assertEquals(1L, ParameterValueNormalizer.normalize("networkTimeout", "500")),
        () -> assertEquals(1L, ParameterValueNormalizer.normalize("networkTimeout", 1000)),
        () -> assertEquals(2L, ParameterValueNormalizer.normalize("networkTimeout", "1999")),
        () -> assertEquals(0L, ParameterValueNormalizer.normalize("networkTimeout", "0")),
        () -> assertEquals("abc", ParameterValueNormalizer.normalize("networkTimeout", "abc")));
  }

  @Test
  void shouldReturnNonStringValuesUnchanged() {
    Integer value = 42;
    assertSame(value, ParameterValueNormalizer.normalize("no_proxy", value));
  }
}
