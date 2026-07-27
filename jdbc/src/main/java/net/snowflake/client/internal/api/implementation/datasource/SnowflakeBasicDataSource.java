package net.snowflake.client.internal.api.implementation.datasource;

import static java.lang.Integer.parseInt;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.CLIENT_STORE_TEMPORARY_CREDENTIAL;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.DISABLE_SAML_URL_CHECK;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.LOGIN_TIMEOUT;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_AUTHORIZATION_URL;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_CLIENT_ID;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_CLIENT_SECRET;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_REDIRECT_URI;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_SCOPE;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OAUTH_TOKEN_REQUEST_URL;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.OKTA_USERNAME;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.PASSCODE_IN_PASSWORD;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.PASSWORD;
import static net.snowflake.client.internal.api.implementation.parameters.SessionProperty.USER;
import static net.snowflake.client.internal.util.UrlUtils.sanitize;

import java.io.PrintWriter;
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
import net.snowflake.client.internal.api.implementation.parameters.SessionProperty;
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
 */
public class SnowflakeBasicDataSource implements SnowflakeDataSource, DelegatingWrapper {

  // TODO: [SNOW-3595091] align authenticator-promotion behavior across drivers.
  //  The legacy JDBC driver auto-set the authenticator to USERNAME_PASSWORD_MFA
  //  from setPasscode / setPasscodeInPassword (and analogous setters for other auth methods),
  //  and analogous behavior exists in the Python and ODBC connectors.
  //  The new universal driver intentionally drops this side-effect for now so
  //  that each setter does only what its name says.
  //  Once the cross-driver decision is finalized: either reinstate the auto-promotion uniformly,
  //  or document a hard "callers must set the authenticator explicitly" contract everywhere.

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
  public Connection getConnection() throws SQLException {
    return getConnection(user, password);
  }

  @Override
  public Connection getConnection(String username, String password) throws SQLException {
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
    } catch (SQLException e) {
      logger.error(
          "Failed to create a connection for {} at {}: {}",
          effectiveUser,
          sanitize(resolvedUrl),
          e.getClass().getName());
      logger.debug("Connection failure detail", e);
      throw e;
    }
  }

  private String resolveUrl() throws SQLException {
    try {
      String resolved = getUrl();
      if (resolved == null || resolved.trim().isEmpty()) {
        throw new IllegalStateException("URL is not set.");
      }
      return resolved;
    } catch (IllegalStateException e) {
      throw new SQLException(e.getMessage(), e);
    }
  }

  private Properties getProperties(String username, String password) {
    Properties properties = getProperties();
    if (username != null) {
      properties.setProperty(USER.getKey(), username);
    }
    if (password != null) {
      properties.setProperty(PASSWORD.getKey(), password);
    }
    return properties;
  }

  protected Connection openConnection(String url, Properties properties) throws SQLException {
    return DriverManager.getConnection(url, properties);
  }

  // CommonDataSource methods ----------------------------------------------------------------------

  @Override
  public PrintWriter getLogWriter() throws SQLException {
    throw new SQLFeatureNotSupportedException();
  }

  @Override
  public void setLogWriter(PrintWriter out) throws SQLException {
    throw new SQLFeatureNotSupportedException();
  }

  @Override
  public int getLoginTimeout() {
    try {
      return parseInt(properties.getProperty(LOGIN_TIMEOUT.getKey()));
    } catch (NumberFormatException e) {
      logger.warn(
          "Could not parse loginTimeout property value '{}', returning default of 0",
          properties.getProperty(LOGIN_TIMEOUT.getKey()));
      return 0;
    }
  }

  @Override
  public void setLoginTimeout(int seconds) {
    properties.put(LOGIN_TIMEOUT.getKey(), Integer.toString(seconds));
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
    this.properties.setProperty(SessionProperty.ACCOUNT.getKey(), account);
  }

  @Override
  public void setDatabase(String database) {
    this.properties.setProperty(SessionProperty.DATABASE.getKey(), database);
  }

  @Override
  public void setSchema(String schema) {
    this.properties.setProperty(SessionProperty.SCHEMA.getKey(), schema);
  }

  @Override
  public void setRole(String role) {
    this.properties.setProperty(SessionProperty.ROLE.getKey(), role);
  }

  @Override
  public void setWarehouse(String warehouse) {
    this.properties.setProperty(SessionProperty.WAREHOUSE.getKey(), warehouse);
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
    this.properties.setProperty("ssl", String.valueOf(ssl));
  }

  @Override
  public void setAuthenticator(String authenticator) {
    this.properties.setProperty(SessionProperty.AUTHENTICATOR.getKey(), authenticator);
  }

  @Override
  public void setToken(String token) {
    this.properties.setProperty(SessionProperty.TOKEN.getKey(), token);
  }

  @Override
  public void setPrivateKey(PrivateKey privateKey) {
    String base64 = Base64.getEncoder().encodeToString(privateKey.getEncoded());
    this.properties.setProperty(SessionProperty.PRIVATE_KEY.getKey(), base64);
  }

  @Override
  public void setPrivateKeyFile(String location, String password) {
    this.properties.setProperty(SessionProperty.PRIVATE_KEY_FILE.getKey(), location);
    if (password != null) {
      this.properties.setProperty(SessionProperty.PRIVATE_KEY_PASSWORD.getKey(), password);
    }
  }

  @Override
  public void setPrivateKeyBase64(String privateKeyBase64, String password) {
    this.properties.setProperty(SessionProperty.PRIVATE_KEY.getKey(), privateKeyBase64);
    if (password != null) {
      this.properties.setProperty(SessionProperty.PRIVATE_KEY_PASSWORD.getKey(), password);
    }
  }

  @Override
  public void setPasscode(String passcode) {
    this.properties.setProperty(SessionProperty.PASSCODE.getKey(), passcode);
  }

  @Override
  public void setPasscodeInPassword(boolean isPasscodeInPassword) {
    this.properties.setProperty(
        PASSCODE_IN_PASSWORD.getKey(), Boolean.toString(isPasscodeInPassword));
  }

  @Override
  public void setOktaUsername(String oktaUsername) {
    this.properties.setProperty(OKTA_USERNAME.getKey(), oktaUsername);
  }

  @Override
  public void setDisableSamlURLCheck(boolean disableSamlURLCheck) {
    this.properties.setProperty(
        DISABLE_SAML_URL_CHECK.getKey(), Boolean.toString(disableSamlURLCheck));
  }

  @Override
  public void setClientStoreTemporaryCredential(boolean clientStoreTemporaryCredential) {
    this.properties.setProperty(
        CLIENT_STORE_TEMPORARY_CREDENTIAL.getKey(),
        Boolean.toString(clientStoreTemporaryCredential));
  }

  @Override
  public void setOauthClientId(String oauthClientId) {
    this.properties.setProperty(OAUTH_CLIENT_ID.getKey(), oauthClientId);
  }

  @Override
  public void setOauthClientSecret(String oauthClientSecret) {
    this.properties.setProperty(OAUTH_CLIENT_SECRET.getKey(), oauthClientSecret);
  }

  @Override
  public void setOauthAuthorizationUrl(String oauthAuthorizationUrl) {
    this.properties.setProperty(OAUTH_AUTHORIZATION_URL.getKey(), oauthAuthorizationUrl);
  }

  @Override
  public void setOauthTokenRequestUrl(String oauthTokenRequestUrl) {
    this.properties.setProperty(OAUTH_TOKEN_REQUEST_URL.getKey(), oauthTokenRequestUrl);
  }

  @Override
  public void setOauthRedirectUri(String oauthRedirectUri) {
    this.properties.setProperty(OAUTH_REDIRECT_URI.getKey(), oauthRedirectUri);
  }

  @Override
  public void setOauthScope(String oauthScope) {
    this.properties.setProperty(OAUTH_SCOPE.getKey(), oauthScope);
  }

  @Override
  public void setOauthEnableSingleUseRefreshTokens(boolean oauthEnableSingleUseRefreshTokens) {
    this.properties.setProperty(
        OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS.getKey(),
        Boolean.toString(oauthEnableSingleUseRefreshTokens));
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
