package net.snowflake.client.internal.util;

import static java.net.URI.create;
import static net.snowflake.client.internal.util.UrlUtils.sanitize;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

class UrlUtilsTest {

  private static final String HOST = "account.snowflakecomputing.com";
  private static final String HOST_PATH = HOST + "/warehouse/db/schema";
  private static final String JDBC_PREFIX = "jdbc:snowflake://" + HOST_PATH;

  @Test
  void shouldReturnNullForNullUrl() {
    assertNull(sanitize(null));
  }

  @Test
  void shouldReturnHostAndPathWhenNoSensitiveParts() {
    assertEquals(HOST_PATH, sanitize(JDBC_PREFIX));
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://" + HOST + "/"));
  }

  @Test
  void shouldStripQueryStringCarryingSecrets() {
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://" + HOST + "/?token=secret&passcode=123"));
    assertEquals(HOST_PATH, sanitize(JDBC_PREFIX + "?private_key=abc&password=def"));
  }

  @Test
  void shouldStripFragment() {
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://" + HOST + "/#fragment"));
    assertEquals(HOST_PATH, sanitize(JDBC_PREFIX + "#session"));
  }

  @Test
  void shouldStripQueryAndFragment() {
    assertEquals(HOST + "/path", sanitize("jdbc:snowflake://" + HOST + "/path?token=secret#frag"));
  }

  @Test
  void shouldStripUserInfoFromAuthority() {
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://user:password@" + HOST + "/"));
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://user@" + HOST + "/"));
  }

  @Test
  void shouldStripUserInfoQueryAndFragmentTogether() {
    assertEquals(
        HOST + "/", sanitize("jdbc:snowflake://user:password@" + HOST + "/?token=secret#frag"));
  }

  @Test
  void shouldUseManualFallbackForMalformedUrls() {
    String malformed = "not valid uri";
    assertThrows(IllegalArgumentException.class, () -> create(malformed));
    assertEquals(malformed, sanitize(malformed));
  }

  @Test
  void shouldStripQueryFromMalformedUrlViaFallback() {
    assertEquals("not valid ", sanitize("not valid ?token=secret&passcode=123"));
  }

  @Test
  void shouldNeverEmitSensitiveQueryValues() {
    String sanitized =
        sanitize("jdbc:snowflake://user:password@" + HOST + "/?token=secret&passcode=123#frag");
    assertEquals(HOST + "/", sanitized);
  }

  @Test
  void shouldHandleEmptyString() {
    assertEquals("", sanitize(""));
  }

  @Test
  void shouldHandleBareQueryMarker() {
    assertEquals(HOST + "/", sanitize("jdbc:snowflake://" + HOST + "/?"));
  }

  @Test
  void shouldNotTreatAtSignInPathAsUserInfo() {
    assertEquals(
        HOST + "/stage@~/files",
        sanitize("jdbc:snowflake://" + HOST + "/stage@~/files?token=secret"));
  }

  private static void assertUriCreateRejected(String url) {
    assertThrows(IllegalArgumentException.class, () -> create(url));
  }

  @Nested
  class Rfc2396InvalidUriFallback {

    @Test
    void shouldFallbackWhenUriHasSpaceInPath() {
      String invalid = "jdbc:snowflake://host/path with space?token=secret&passcode=123";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/path with space", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasSpaceInHost() {
      String invalid = "jdbc:snowflake://host name/path?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host name/path", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasUnescapedBraceInPath() {
      String invalid = "jdbc:snowflake://host/{bad}/path?token=secret#frag";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/{bad}/path", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasMalformedPercentEscape() {
      String invalid = "jdbc:snowflake://host/%ZZ/path?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/%ZZ/path", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasIllegalCharacterInPath() {
      String invalid = "not valid ^ uri?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("not valid ^ uri", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasIllegalCharacterInHost() {
      String invalid = "jdbc:snowflake://host|bad?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host|bad", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasBackslashInPath() {
      String invalid = "jdbc:snowflake://host\\path?token=secret#frag";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host\\path", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasUnescapedQuoteInPath() {
      String invalid = "jdbc:snowflake://host/\"path\"?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/\"path\"", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasLeadingWhitespace() {
      String invalid = " notvalid?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals(" notvalid", sanitize(invalid));
    }

    @Test
    void shouldFallbackWhenUriHasControlCharacterInPath() {
      String invalid = "jdbc:snowflake://host/\u0007?token=secret";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/\u0007", sanitize(invalid));
    }

    @Test
    void shouldFallbackAndStripUserInfoQueryAndFragmentTogether() {
      String invalid = "jdbc:snowflake://user:password@host/path with space?token=secret#frag";
      assertUriCreateRejected(invalid);
      assertEquals("jdbc:snowflake://host/path with space", sanitize(invalid));
    }

    @Test
    void shouldFallBackWhenUriHasNoHost() {
      String valid = "jdbc:snowflake:///path?token=secret";
      assertEquals("jdbc:snowflake:///path", sanitize(valid));
    }
  }
}
