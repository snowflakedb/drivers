package net.snowflake.client.internal.api.implementation.datasource;

import lombok.AccessLevel;
import lombok.Getter;
import lombok.RequiredArgsConstructor;

@Getter
@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
enum SnowflakeSessionProperty {
  USER("user"),
  PASSWORD("password"),
  ACCOUNT("account"),
  DATABASE("database"),
  SCHEMA("schema"),
  ROLE("role"),
  WAREHOUSE("warehouse"),
  LOGIN_TIMEOUT("loginTimeout"),
  AUTHENTICATOR("authenticator"),
  TOKEN("token"),
  PRIVATE_KEY_FILE("private_key_file"),
  PRIVATE_KEY("private_key"),
  PRIVATE_KEY_PASSWORD("private_key_password"),
  PASSCODE("passcode"),
  PASSCODE_IN_PASSWORD("passcodeInPassword"),
  CLIENT_STORE_TEMPORARY_CREDENTIAL("clientStoreTemporaryCredential"),
  OAUTH_CLIENT_ID("oauth_client_id"),
  OAUTH_CLIENT_SECRET("oauth_client_secret"),
  OAUTH_AUTHORIZATION_URL("oauth_authorization_url"),
  OAUTH_TOKEN_REQUEST_URL("oauth_token_request_url"),
  OAUTH_REDIRECT_URI("oauth_redirect_uri"),
  OAUTH_SCOPE("oauth_scope"),
  OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS("oauth_enable_single_use_refresh_tokens");

  private final String propertyKey;
}
