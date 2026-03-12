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
  LOGIN_TIMEOUT("loginTimeout");

  private final String propertyKey;
}
