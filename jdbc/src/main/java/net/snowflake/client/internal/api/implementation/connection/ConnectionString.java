package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.client.internal.util.StringUtil.isNullOrEmpty;

import java.io.UnsupportedEncodingException;
import java.net.URI;
import java.net.URISyntaxException;
import java.net.URLDecoder;
import java.util.Collections;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

public class ConnectionString {
  private static final SFLogger logger = SFLoggerFactory.getLogger(ConnectionString.class);
  private static final String PREFIX = "jdbc:snowflake://";
  private static final String ALLOW_UNDERSCORES_IN_HOST = "ALLOWUNDERSCORESINHOST";

  private static final ConnectionString INVALID_CONNECT_STRING =
      new ConnectionString("", "", -1, Collections.emptyMap(), "");

  private final String scheme;
  private final String host;
  private final int port;
  private final Map<String, Object> parameters;
  private final String account;

  private ConnectionString(
      String scheme, String host, int port, Map<String, Object> parameters, String account) {
    this.scheme = scheme;
    this.host = host;
    this.port = port;
    this.parameters = parameters;
    this.account = account;
  }

  public static boolean hasUnsupportedPrefix(String url) {
    return url == null || !url.startsWith(PREFIX);
  }

  public static ConnectionString parse(String url) {
    return parse(url, new Properties());
  }

  public static ConnectionString parse(String url, Properties info) {
    if (hasUnsupportedPrefix(url)) {
      logger.debug("Connect strings must start with jdbc:snowflake://");
      return INVALID_CONNECT_STRING;
    }

    try {
      return new ConnectionStringBuilder(url, info)
          .parseUri()
          .extractHostAndPort()
          .ensureSupportedSchemeHostPathAndDefaultPort()
          .applyQueryParameters()
          .normalizeScheme()
          .applyInfoOverrides()
          .deriveAccountFromHostIfMissing()
          .ensureAccountPresent()
          .normalizeHostForUnderscoreAccount()
          .applyDefaultPortForEffectiveScheme()
          .build();
    } catch (URISyntaxException uriEx) {
      logger.warn(
          "Exception thrown while parsing Snowflake connect string. Illegal character in url.",
          uriEx);
      return INVALID_CONNECT_STRING;
    } catch (Exception ex) {
      logger.warn("Exception thrown while parsing Snowflake connect string", ex);
      return INVALID_CONNECT_STRING;
    }
  }

  public boolean isValid() {
    return !isNullOrEmpty(host);
  }

  public String getScheme() {
    return scheme;
  }

  public String getHost() {
    return host;
  }

  public int getPort() {
    return port;
  }

  public Map<String, Object> getParameters() {
    return Collections.unmodifiableMap(parameters);
  }

  public String getAccount() {
    return account;
  }

  private static boolean isSslDisabled(Object value) {
    if (value instanceof Boolean) {
      return !((Boolean) value);
    }
    String vs = String.valueOf(value);
    return "off".equalsIgnoreCase(vs) || "false".equalsIgnoreCase(vs);
  }

  private static final class ConnectionStringBuilder {
    private final String url;
    private final Properties info;
    private URI uri;
    String scheme;
    String host;
    int port = -1;
    String account;
    Map<String, Object> parameters = new HashMap<>();

    private ConnectionStringBuilder(String url, Properties info) {
      this.url = url;
      this.info = info == null ? new Properties() : info;
    }

    ConnectionStringBuilder parseUri() throws Exception {
      String afterPrefix = url.substring(PREFIX.length());
      if (!afterPrefix.startsWith("http://") && !afterPrefix.startsWith("https://")) {
        afterPrefix = url.substring(url.indexOf("snowflake:"));
      }
      this.uri = new URI(afterPrefix);
      this.scheme = uri.getScheme();
      return this;
    }

    ConnectionStringBuilder extractHostAndPort() {
      String authority = uri.getRawAuthority();
      if (authority == null) {
        throw new IllegalArgumentException("Missing URI authority");
      }
      String[] hostAndPort = authority.split(":");
      if (hostAndPort.length == 2) {
        host = hostAndPort[0];
        port = Integer.parseInt(hostAndPort[1]);
      } else if (hostAndPort.length == 1) {
        host = hostAndPort[0];
      }
      return this;
    }

    ConnectionStringBuilder ensureSupportedSchemeHostPathAndDefaultPort() {
      if (!"snowflake".equals(scheme) && !"http".equals(scheme) && !"https".equals(scheme)) {
        logger.debug("Connect strings must have a valid scheme: 'snowflake' or 'http' or 'https'");
        throw new IllegalArgumentException("Unsupported scheme");
      }
      if (isNullOrEmpty(host)) {
        logger.debug("Connect strings must have a valid host: found null or empty host");
        throw new IllegalArgumentException("Missing host");
      }
      String path = uri.getPath();
      if (!isNullOrEmpty(path) && !"/".equals(path)) {
        logger.debug("Connect strings must have no path: expecting empty or null or '/'");
        throw new IllegalArgumentException("Invalid path");
      }
      return this;
    }

    ConnectionStringBuilder applyQueryParameters() {
      String queryData = uri.getRawQuery();
      if (isNullOrEmpty(queryData)) {
        return this;
      }
      String[] params = queryData.split("&");
      for (String param : params) {
        String[] keyVals = param.split("=", 2);
        if (keyVals.length != 2 || keyVals[1].isEmpty()) {
          continue;
        }
        try {
          String key = URLDecoder.decode(keyVals[0], "UTF-8");
          String value = URLDecoder.decode(keyVals[1], "UTF-8");
          applyParameter(key, value);
        } catch (UnsupportedEncodingException ex) {
          logger.warn("Failed to decode a parameter {}. Ignored.", param);
        }
      }
      return this;
    }

    ConnectionStringBuilder normalizeScheme() {
      if ("snowflake".equals(scheme)) {
        scheme = "https";
      }
      return this;
    }

    ConnectionStringBuilder applyInfoOverrides() {
      if (info.isEmpty()) {
        return this;
      }
      for (Map.Entry<Object, Object> entry : info.entrySet()) {
        String key = entry.getKey().toString();
        Object value = entry.getValue();
        if ("ssl".equalsIgnoreCase(key) && isSslDisabled(value)) {
          scheme = "http";
        } else if ("account".equalsIgnoreCase(key)) {
          account = value.toString();
        }
        parameters.put(key.toUpperCase(Locale.US), value);
      }
      return this;
    }

    ConnectionStringBuilder deriveAccountFromHostIfMissing() {
      if (parameters.get("ACCOUNT") != null || account != null || host.indexOf('.') <= 0) {
        return this;
      }
      account = host.substring(0, host.indexOf('.'));
      if (host.contains(".global.")) {
        int idx = account.lastIndexOf('-');
        if (idx > 0) {
          account = account.substring(0, idx);
        }
      }
      parameters.put("ACCOUNT", account);
      return this;
    }

    ConnectionStringBuilder ensureAccountPresent() {
      if (isNullOrEmpty(account)) {
        logger.debug("Connect strings must contain account identifier");
        throw new IllegalArgumentException("Missing account");
      }
      return this;
    }

    ConnectionStringBuilder normalizeHostForUnderscoreAccount() {
      boolean allowUnderscoresInHost =
          Boolean.parseBoolean(String.valueOf(parameters.get(ALLOW_UNDERSCORES_IN_HOST)));
      if (account.contains("_") && !allowUnderscoresInHost && host.startsWith(account)) {
        host = host.replaceFirst(account, account.replace("_", "-"));
      }
      return this;
    }

    ConnectionStringBuilder applyDefaultPortForEffectiveScheme() {
      if (port != -1) {
        return this;
      }
      port = "http".equals(scheme) ? 80 : 443;
      return this;
    }

    ConnectionString build() {
      return new ConnectionString(scheme, host, port, parameters, account);
    }

    private void applyParameter(String key, String value) {
      if ("ssl".equalsIgnoreCase(key) && isSslDisabled(value)) {
        scheme = "http";
      } else if ("account".equalsIgnoreCase(key)) {
        account = value;
      }
      parameters.put(key.toUpperCase(Locale.US), value);
    }
  }
}
