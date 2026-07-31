package net.snowflake.client.api.datasource;

import java.security.PrivateKey;
import java.util.Properties;
import javax.sql.DataSource;

/**
 * Snowflake-specific extension of {@link DataSource} that provides configuration methods for
 * Snowflake JDBC connections.
 *
 * <p>The setter surface covers sf_core connection parameters plus JDBC client-side knobs the
 * wrapper already honors (see {@code BehaviorDifferences.yaml} BD#31). Legacy DataSource setters
 * without a corresponding parameter remain intentionally unported.
 *
 * <p>Use {@link SnowflakeDataSourceFactory} to create instances of this interface.
 */
public interface SnowflakeDataSource extends DataSource {

  void setUrl(String url);

  void setUser(String user);

  void setPassword(String password);

  void setAccount(String account);

  void setDatabase(String database);

  void setDatabaseName(String databaseName);

  void setSchema(String schema);

  void setRole(String role);

  void setWarehouse(String warehouse);

  void setPortNumber(int portNumber);

  void setServerName(String serverName);

  void setSsl(boolean ssl);

  void setAuthenticator(String authenticator);

  void setToken(String token);

  void setOauthToken(String oauthToken);

  void setPat(String pat);

  void setPrivateKey(PrivateKey privateKey);

  void setPrivateKeyFile(String location, String password);

  void setPrivateKeyBase64(String privateKeyBase64, String password);

  void setPasscode(String passcode);

  void setPasscodeInPassword(boolean isPasscodeInPassword);

  void setOktaUsername(String oktaUsername);

  void setDisableSamlURLCheck(boolean disableSamlURLCheck);

  void setClientStoreTemporaryCredential(boolean clientStoreTemporaryCredential);

  void setOauthClientId(String oauthClientId);

  void setOauthClientSecret(String oauthClientSecret);

  void setOauthAuthorizationUrl(String oauthAuthorizationUrl);

  void setOauthTokenRequestUrl(String oauthTokenRequestUrl);

  void setOauthRedirectUri(String oauthRedirectUri);

  void setOauthScope(String oauthScope);

  void setOauthEnableSingleUseRefreshTokens(boolean oauthEnableSingleUseRefreshTokens);

  void setApplication(String application);

  void setAllowUnderscoresInHost(boolean allowUnderscoresInHost);

  void setQueryTimeout(int queryTimeoutSeconds);

  void setMaxHttpRetries(int maxHttpRetries);

  void setPutGetMaxRetries(int putGetMaxRetries);

  void setProxyHost(String proxyHost);

  void setProxyPort(int proxyPort);

  void setProxyUser(String proxyUser);

  void setProxyPassword(String proxyPassword);

  void setNonProxyHosts(String nonProxyHosts);

  void setEnableDiagnostics(boolean enableDiagnostics);

  void setDiagnosticsAllowlistFile(String diagnosticsAllowlistFile);

  void setBrowserResponseTimeout(int browserResponseTimeoutSeconds);

  void setTracing(String tracing);

  void setEnablePatternSearch(boolean enablePatternSearch);

  /**
   * Controls whether scale-0 FIXED/DECIMAL columns are treated as integer types.
   *
   * <p>Stores {@code JDBC_TREAT_DECIMAL_AS_INT} (the parameter the universal driver reads). Legacy
   * snowflake-jdbc stored {@code JDBC_ARROW_TREAT_DECIMAL_AS_INT} instead.
   */
  void setArrowTreatDecimalAsInt(boolean treatDecimalAsInt);

  void setJDBCDefaultFormatDateWithTimezone(Boolean jdbcDefaultFormatDateWithTimezone);

  void setGetDateUseNullTimezone(Boolean getDateUseNullTimezone);

  String getUrl();

  Properties getProperties();
}
