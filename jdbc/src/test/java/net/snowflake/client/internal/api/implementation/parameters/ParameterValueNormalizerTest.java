package net.snowflake.client.internal.api.implementation.parameters;

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
  void shouldReturnNonStringValuesUnchanged() {
    Integer value = 42;
    assertSame(value, ParameterValueNormalizer.normalize("no_proxy", value));
  }
}
