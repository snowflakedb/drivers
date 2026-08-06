package net.snowflake.client.internal.api.implementation.datasource;

import lombok.AccessLevel;
import lombok.Getter;
import lombok.RequiredArgsConstructor;

/**
 * Authenticator wire values for {@link SnowflakeBasicDataSource} promotion and alignment with core.
 *
 * <p>Variants mirror {@code sf_core::config::connection_config::AuthConfig}; {@link #wireValue}
 * mirrors the {@code authenticator} connection parameter when one exists ({@code null} when auth is
 * selected by other parameters or a per-account URL). Could be generated from core in the future.
 */
@Getter
@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
enum DataSourceAuthenticator {
  PASSWORD("SNOWFLAKE"),
  MFA("USERNAME_PASSWORD_MFA"),
  JWT("SNOWFLAKE_JWT"),
  PAT("PROGRAMMATIC_ACCESS_TOKEN"),
  NATIVE_OKTA(null),
  EXTERNAL_BROWSER("EXTERNALBROWSER"),
  OAUTH_ACCESS_TOKEN("OAUTH"),
  OAUTH_AUTHORIZATION_CODE("OAUTH_AUTHORIZATION_CODE"),
  OAUTH_CLIENT_CREDENTIALS("OAUTH_CLIENT_CREDENTIALS"),
  SESSION_TOKEN(null),
  WORKLOAD_IDENTITY("WORKLOAD_IDENTITY");

  private final String wireValue;
}
