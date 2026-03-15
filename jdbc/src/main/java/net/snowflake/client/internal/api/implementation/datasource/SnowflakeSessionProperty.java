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
  PRIVATE_KEY_FILE("private_key_file"),
  PRIVATE_KEY_PWD("private_key_pwd");

  private final String propertyKey;
}
