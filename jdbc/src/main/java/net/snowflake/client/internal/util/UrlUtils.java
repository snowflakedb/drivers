package net.snowflake.client.internal.util;

import static java.net.URI.create;
import static lombok.AccessLevel.PRIVATE;

import java.net.URI;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.datasource.SnowflakeBasicDataSource;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Reduces a JDBC URL to host + path for safe logging. Per logging guidelines only the host and path
 * are emitted; userinfo, query, and fragment (which may carry credentials) are dropped.
 */
@NoArgsConstructor(access = PRIVATE)
public class UrlUtils {

  private static final String JDBC_PREFIX = "jdbc:";
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeBasicDataSource.class);

  public static String sanitize(String url) {
    if (url == null) {
      return null;
    }
    try {
      URI uri = create(stripJdbcPrefix(url));
      String host = uri.getHost();
      if (host != null) {
        String path = uri.getPath();
        return host + path;
      }
    } catch (IllegalArgumentException parseException) {
      // URI cannot parse this url (e.g. RFC 2396 violations); fall back to manual stripping below.
      logger.debug("Sanitize failed - URI cannot parse url", parseException.getMessage());
    }
    return stripSensitiveParts(url);
  }

  private static String stripJdbcPrefix(String url) {
    return url.startsWith(JDBC_PREFIX) ? url.substring(JDBC_PREFIX.length()) : url;
  }

  private static String stripSensitiveParts(String url) {
    return removeUserInfo(removeFragment(removeQuery(url)));
  }

  private static String removeQuery(String url) {
    int query = url.indexOf('?');
    return query < 0 ? url : url.substring(0, query);
  }

  private static String removeFragment(String url) {
    int fragment = url.indexOf('#');
    return fragment < 0 ? url : url.substring(0, fragment);
  }

  private static String removeUserInfo(String url) {
    int schemeSep = url.indexOf("://");
    if (schemeSep < 0) {
      return url;
    }
    int authorityStart = schemeSep + 3;
    int at = url.indexOf('@', authorityStart);
    if (at < 0) {
      return url;
    }
    int slash = url.indexOf('/', authorityStart);
    if (slash >= 0 && at > slash) {
      return url;
    }
    return url.substring(0, authorityStart) + url.substring(at + 1);
  }
}
