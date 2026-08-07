package net.snowflake.client.internal.api.implementation.datasource;

import static java.lang.Integer.parseInt;
import static net.snowflake.client.internal.util.StringUtil.isNullOrEmpty;
import static net.snowflake.client.internal.util.UrlUtils.sanitize;

import java.io.PrintWriter;
import java.io.Serializable;
import java.security.PrivateKey;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Base64;
import java.util.Properties;
import java.util.function.Supplier;
import java.util.logging.Logger;
import net.snowflake.client.api.datasource.SnowflakeDataSource;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.Property;
import net.snowflake.client.internal.api.implementation.parameters.SessionProperty;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.util.DelegatingWrapper;

/**
 * Basic implementation of {@link SnowflakeDataSource} for Snowflake JDBC connections.
 *
 * <p>This class provides a simple, non-pooled DataSource implementation that creates new Snowflake
 * connections on demand. It is suitable for applications that do not require connection pooling or
 * for use with external connection pool managers.
 *
 * <p><b>Note:</b> This class is not intended for direct instantiation. Use {@link
 * net.snowflake.client.api.datasource.SnowflakeDataSourceFactory#createDataSource()} instead.
 *
 * <p><b>Security note:</b> like the reference driver, this class is {@link java.io.Serializable} so
 * it can be bound in JNDI. Credentials configured via setters (password, token, private key and its
 * passphrase, proxy password) are held in the serialized {@code properties} and are therefore
 * written in clear text when the instance is serialized; protect any serialized form accordingly.
 */
@JdbcBoundary
public class SnowflakeBasicDataSource
    implements SnowflakeDataSource, DelegatingWrapper, Serializable {

  private static final long serialVersionUID = 1L;

  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeBasicDataSource.class);

  static {
    try {
      Class.forName("net.snowflake.client.api.driver.SnowflakeDriver");
    } catch (ClassNotFoundException e) {
      throw new IllegalStateException(
          "Unable to load "
              + "net.snowflake.client.api.driver.SnowflakeDriver. "
              + "Please check if you have proper Snowflake JDBC "
              + "Driver jar on the classpath",
          e);
    }
  }

  private final Properties properties = new Properties();
  private String url;
  private String user;
  private String password;
  private String serverName;
  private int portNumber;

  // DataSource methods ----------------------------------------------------------------------------

  @Override
  public Connection getConnection() {
    return getConnection(user, password);
  }

  @Override
  public Connection getConnection(String username, String password) {
    String resolvedUrl = resolveUrl();
    String effectiveUser = username != null ? username : user;
    try {
      Properties properties = getProperties(username, password);
      Connection con = openConnection(resolvedUrl, properties);
      logger.debug(
          "Created a connection for {} at {}",
          effectiveUser,
          (Supplier<String>) () -> sanitize(getUrl()));
      return con;
    } catch (Exception e) {
      logger.error(
          "Failed to create a connection for {} at {}: {}",
          effectiveUser,
          sanitize(resolvedUrl),
          e.getClass().getName());
      logger.debug("Connection failure detail", e);
      throw e;
    }
  }

  private String resolveUrl() {
    String resolved = getUrl();
    if (resolved == null || resolved.trim().isEmpty()) {
      throw new IllegalStateException("URL is not set.");
    }
    return resolved;
  }

  private Properties getProperties(String username, String password) {
    Properties properties = getProperties();
    if (username != null) {
      properties.setProperty(SessionProperty.USER.getKey(), username);
    }
    if (password != null) {
      properties.setProperty(SessionProperty.PASSWORD.getKey(), password);
    }
    return properties;
  }

  protected Connection openConnection(String url, Properties properties) {
    try {
      return DriverManager.getConnection(url, properties);
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  // CommonDataSource methods ----------------------------------------------------------------------

  @Override
  public PrintWriter getLogWriter() {
    throw new SFSQLFeatureNotSupportedException("getLogWriter not supported");
  }

  @Override
  public void setLogWriter(PrintWriter out) {
    throw new SFSQLFeatureNotSupportedException("setLogWriter not supported");
  }

  @Override
  public int getLoginTimeout() {
    try {
      return parseInt(properties.getProperty(SessionProperty.LOGIN_TIMEOUT.getKey()));
    } catch (NumberFormatException e) {
      logger.warn(
          "Could not parse loginTimeout property value '{}', returning default of 0",
          properties.getProperty(SessionProperty.LOGIN_TIMEOUT.getKey()));
      return 0;
    }
  }

  @Override
  public void setLoginTimeout(int seconds) {
    setProperty(SessionProperty.LOGIN_TIMEOUT, seconds);
  }

  @Override
  public Logger getParentLogger() throws SQLFeatureNotSupportedException {
    throw new SQLFeatureNotSupportedException();
  }

  // SnowflakeDataSource methods -------------------------------------------------------------------

  @Override
  public void setUrl(String url) {
    this.url = url;
  }

  @Override
  public void setUser(String user) {
    this.user = user;
  }

  @Override
  public void setPassword(String password) {
    this.password = password;
  }

  @Override
  public void setAccount(String account) {
    setProperty(SessionProperty.ACCOUNT, account);
  }

  @Override
  public void setDatabase(String database) {
    setProperty(SessionProperty.DATABASE, database);
  }

  @Override
  public void setSchema(String schema) {
    setProperty(SessionProperty.SCHEMA, schema);
  }

  @Override
  public void setRole(String role) {
    setProperty(SessionProperty.ROLE, role);
  }

  @Override
  public void setWarehouse(String warehouse) {
    setProperty(SessionProperty.WAREHOUSE, warehouse);
  }

  @Override
  public void setDatabaseName(String databaseName) {
    setDatabase(databaseName);
  }

  @Override
  public void setPortNumber(int portNumber) {
    this.portNumber = portNumber;
  }

  @Override
  public void setServerName(String serverName) {
    this.serverName = serverName;
  }

  @Override
  public void setSsl(boolean ssl) {
    setProperty(SessionProperty.SSL, ssl);
  }

  @Override
  public void setAuthenticator(String authenticator) {
    setProperty(SessionProperty.AUTHENTICATOR, authenticator);
  }

  private void setAuthenticator(DataSourceAuthenticator authenticator) {
    setProperty(SessionProperty.AUTHENTICATOR, authenticator.getWireValue());
  }

  @Override
  public void setToken(String token) {
    // can be used with PROGRAMMATIC_ACCESS_TOKEN, OAUTH
    setProperty(SessionProperty.TOKEN, token);
  }

  @Override
  public void setOauthToken(String oauthToken) {
    setAuthenticator(DataSourceAuthenticator.OAUTH_ACCESS_TOKEN);
    setProperty(SessionProperty.TOKEN, oauthToken);
  }

  @Override
  public void setPat(String pat) {
    setAuthenticator(DataSourceAuthenticator.PAT);
    setProperty(SessionProperty.TOKEN, pat);
  }

  @Override
  public void setPrivateKey(PrivateKey privateKey) {
    setAuthenticator(DataSourceAuthenticator.JWT);
    setProperty(
        SessionProperty.PRIVATE_KEY, Base64.getEncoder().encodeToString(privateKey.getEncoded()));
  }

  @Override
  public void setPrivateKeyFile(String location, String password) {
    setAuthenticator(DataSourceAuthenticator.JWT);
    setProperty(SessionProperty.PRIVATE_KEY_FILE, location);
    if (isNullOrEmpty(password)) {
      clearProperty(SessionProperty.PRIVATE_KEY_PASSWORD);
    } else {
      setProperty(SessionProperty.PRIVATE_KEY_PASSWORD, password);
    }
  }

  @Override
  public void setPrivateKeyBase64(String privateKeyBase64, String password) {
    setAuthenticator(DataSourceAuthenticator.JWT);
    setProperty(SessionProperty.PRIVATE_KEY, privateKeyBase64);
    if (isNullOrEmpty(password)) {
      clearProperty(SessionProperty.PRIVATE_KEY_PASSWORD);
    } else {
      setProperty(SessionProperty.PRIVATE_KEY_PASSWORD, password);
    }
  }

  @Override
  public void setPasscode(String passcode) {
    setAuthenticator(DataSourceAuthenticator.MFA);
    setProperty(SessionProperty.PASSCODE, passcode);
  }

  @Override
  public void setPasscodeInPassword(boolean isPasscodeInPassword) {
    setProperty(SessionProperty.PASSCODE_IN_PASSWORD, isPasscodeInPassword);
    if (isPasscodeInPassword) {
      setAuthenticator(DataSourceAuthenticator.MFA);
    }
  }

  @Override
  public void setOktaUsername(String oktaUsername) {
    // companion to native Okta (authenticator=<vanity URL>)
    setProperty(SessionProperty.OKTA_USERNAME, oktaUsername);
  }

  @Override
  public void setDisableSamlURLCheck(boolean disableSamlURLCheck) {
    setProperty(SessionProperty.DISABLE_SAML_URL_CHECK, disableSamlURLCheck);
  }

  @Override
  public void setClientStoreTemporaryCredential(boolean clientStoreTemporaryCredential) {
    setAuthenticator(DataSourceAuthenticator.EXTERNAL_BROWSER);
    setProperty(SessionProperty.CLIENT_STORE_TEMPORARY_CREDENTIAL, clientStoreTemporaryCredential);
  }

  @Override
  public void setOauthClientId(String oauthClientId) {
    // can be used with OAUTH_AUTHORIZATION_CODE and OAUTH_CLIENT_CREDENTIALS
    setProperty(SessionProperty.OAUTH_CLIENT_ID, oauthClientId);
  }

  @Override
  public void setOauthClientSecret(String oauthClientSecret) {
    // can be used with OAUTH_CLIENT_CREDENTIALS and OAUTH_AUTHORIZATION_CODE
    setProperty(SessionProperty.OAUTH_CLIENT_SECRET, oauthClientSecret);
  }

  @Override
  public void setOauthAuthorizationUrl(String oauthAuthorizationUrl) {
    setAuthenticator(DataSourceAuthenticator.OAUTH_AUTHORIZATION_CODE);
    setProperty(SessionProperty.OAUTH_AUTHORIZATION_URL, oauthAuthorizationUrl);
  }

  @Override
  public void setOauthTokenRequestUrl(String oauthTokenRequestUrl) {
    // can be used with OAUTH_CLIENT_CREDENTIALS and OAUTH_AUTHORIZATION_CODE
    setProperty(SessionProperty.OAUTH_TOKEN_REQUEST_URL, oauthTokenRequestUrl);
  }

  @Override
  public void setOauthRedirectUri(String oauthRedirectUri) {
    setAuthenticator(DataSourceAuthenticator.OAUTH_AUTHORIZATION_CODE);
    setProperty(SessionProperty.OAUTH_REDIRECT_URI, oauthRedirectUri);
  }

  @Override
  public void setOauthScope(String oauthScope) {
    // can be optional for OAUTH_AUTHORIZATION_CODE and OAUTH_CLIENT_CREDENTIALS
    setProperty(SessionProperty.OAUTH_SCOPE, oauthScope);
  }

  @Override
  public void setOauthEnableSingleUseRefreshTokens(boolean oauthEnableSingleUseRefreshTokens) {
    setAuthenticator(DataSourceAuthenticator.OAUTH_AUTHORIZATION_CODE);
    setProperty(
        SessionProperty.OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS, oauthEnableSingleUseRefreshTokens);
  }

  @Override
  public void setApplication(String application) {
    setProperty(SessionProperty.APPLICATION, application);
  }

  @Override
  public void setAllowUnderscoresInHost(boolean allowUnderscoresInHost) {
    setProperty(SessionProperty.ALLOW_UNDERSCORES_IN_HOST, allowUnderscoresInHost);
  }

  @Override
  public void setQueryTimeout(int queryTimeoutSeconds) {
    setProperty(SessionProperty.QUERY_TIMEOUT_SECONDS, queryTimeoutSeconds);
  }

  @Override
  public void setMaxHttpRetries(int maxHttpRetries) {
    setProperty(SessionProperty.MAX_HTTP_RETRIES, maxHttpRetries);
  }

  @Override
  public void setPutGetMaxRetries(int putGetMaxRetries) {
    setProperty(SessionProperty.PUT_GET_MAX_RETRIES, putGetMaxRetries);
  }

  @Override
  public void setProxyHost(String proxyHost) {
    setProperty(SessionProperty.PROXY_HOST, proxyHost);
  }

  @Override
  public void setProxyPort(int proxyPort) {
    setProperty(SessionProperty.PROXY_PORT, proxyPort);
  }

  @Override
  public void setProxyUser(String proxyUser) {
    setProperty(SessionProperty.PROXY_USER, proxyUser);
  }

  @Override
  public void setProxyPassword(String proxyPassword) {
    setProperty(SessionProperty.PROXY_PASSWORD, proxyPassword);
  }

  @Override
  public void setNonProxyHosts(String nonProxyHosts) {
    setProperty(SessionProperty.NON_PROXY_HOSTS, nonProxyHosts);
  }

  @Override
  public void setEnableDiagnostics(boolean enableDiagnostics) {
    setProperty(SessionProperty.ENABLE_DIAGNOSTICS, enableDiagnostics);
  }

  @Override
  public void setDiagnosticsAllowlistFile(String diagnosticsAllowlistFile) {
    setProperty(SessionProperty.DIAGNOSTICS_ALLOWLIST_FILE, diagnosticsAllowlistFile);
  }

  @Override
  public void setBrowserResponseTimeout(int browserResponseTimeoutSeconds) {
    setAuthenticator(DataSourceAuthenticator.EXTERNAL_BROWSER);
    setProperty(SessionProperty.BROWSER_RESPONSE_TIMEOUT, browserResponseTimeoutSeconds);
  }

  @Override
  public void setTracing(String tracing) {
    setProperty(Parameter.TRACING, tracing);
  }

  @Override
  public void setEnablePatternSearch(boolean enablePatternSearch) {
    setProperty(Parameter.ENABLE_PATTERN_SEARCH, enablePatternSearch);
  }

  @Override
  public void setArrowTreatDecimalAsInt(boolean treatDecimalAsInt) {
    setProperty(Parameter.JDBC_TREAT_DECIMAL_AS_INT, treatDecimalAsInt);
  }

  @Override
  public void setJDBCDefaultFormatDateWithTimezone(Boolean jdbcDefaultFormatDateWithTimezone) {
    setProperty(
        Parameter.JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE, jdbcDefaultFormatDateWithTimezone);
  }

  @Override
  public void setGetDateUseNullTimezone(Boolean getDateUseNullTimezone) {
    setProperty(Parameter.JDBC_GET_DATE_USE_NULL_TIMEZONE, getDateUseNullTimezone);
  }

  // Legacy snowflake-jdbc stores string setters via Properties.put, which rejects null values
  // with NullPointerException; match that contract rather than treating null as "clear".
  private void setProperty(Property property, String value) {
    this.properties.setProperty(property.getKey(), value);
  }

  private void setProperty(Property property, boolean value) {
    this.properties.setProperty(property.getKey(), Boolean.toString(value));
  }

  private void setProperty(Property property, Boolean value) {
    // Unbox so a null Boolean NPEs like legacy Properties.put(null) / Boolean unboxing.
    setProperty(property, value.booleanValue());
  }

  private void setProperty(Property property, int value) {
    this.properties.setProperty(property.getKey(), Integer.toString(value));
  }

  private void clearProperty(Property property) {
    this.properties.remove(property.getKey());
  }

  @Override
  public String getUrl() {
    if (url != null) {
      return url;
    }
    if (serverName == null) {
      return null;
    }
    StringBuilder sb = new StringBuilder("jdbc:snowflake://");
    sb.append(serverName);
    if (portNumber != 0) {
      sb.append(":").append(portNumber);
    }
    return sb.toString();
  }

  @Override
  public Properties getProperties() {
    Properties properties = new Properties();
    properties.putAll(this.properties);
    return properties;
  }
}
