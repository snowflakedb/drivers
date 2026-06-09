package net.snowflake.client.api.datasource;

import java.security.PrivateKey;
import java.util.Properties;
import javax.sql.DataSource;

/**
 * Snowflake-specific extension of {@link DataSource} that provides configuration methods for
 * Snowflake JDBC connections.
 *
 * <p>Use {@link SnowflakeDataSourceFactory} to create instances of this interface.
 */
public interface SnowflakeDataSource extends DataSource {
  // Only a minimal set of DataSource parameters has been migrated here.
  // More will be added once the parameter strategy for the core driver is finalized.

  void setUrl(String url);

  void setUser(String user);

  void setPassword(String password);

  void setAccount(String account);

  void setDatabase(String database);

  void setSchema(String schema);

  void setRole(String role);

  void setWarehouse(String warehouse);

  void setAuthenticator(String authenticator);

  void setToken(String token);

  void setPrivateKey(PrivateKey privateKey);

  void setPrivateKeyFile(String location, String password);

  void setPrivateKeyBase64(String privateKeyBase64, String password);

  void setPasscode(String passcode);

  void setPasscodeInPassword(boolean isPasscodeInPassword);

  void setClientStoreTemporaryCredential(boolean clientStoreTemporaryCredential);

  void setOauthClientId(String oauthClientId);

  void setOauthClientSecret(String oauthClientSecret);

  void setOauthAuthorizationUrl(String oauthAuthorizationUrl);

  void setOauthTokenRequestUrl(String oauthTokenRequestUrl);

  void setOauthRedirectUri(String oauthRedirectUri);

  void setOauthScope(String oauthScope);

  void setOauthEnableSingleUseRefreshTokens(boolean oauthEnableSingleUseRefreshTokens);

  String getUrl();

  Properties getProperties();
}
