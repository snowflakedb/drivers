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
  PRIVATE_KEY_PASSWORD("private_key_password");

  private final String propertyKey;
}
