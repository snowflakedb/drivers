package net.snowflake.client.api.datasource;

import java.security.PrivateKey;
import java.util.Properties;
import javax.sql.DataSource;

/**
 * Snowflake-specific extension of {@link DataSource} that provides configuration methods for
 * Snowflake JDBC connections.
 *
 * <p>The setter surface is limited to parameters supported by sf_core (see {@code
 * BehaviorDifferences.yaml} BD#31). Legacy JDBC DataSource setters without a core parameter are
 * intentionally not ported.
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

  String getUrl();

  Properties getProperties();
}
