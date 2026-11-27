package net.snowflake.client.api.auth;

/**
 * Names of values of the {@code authenticator} connection property. The driver does not read this
 * enum; the property is a case-insensitive string. {@link #OKTA} and {@link #ID_TOKEN} are not
 * accepted values.
 *
 * <h2 id="usage-example">Usage Example</h2>
 *
 * <pre>{@code
 * Properties props = new Properties();
 * props.put("user", "myuser");
 * props.put("authenticator", AuthenticatorType.EXTERNALBROWSER.name());
 * Connection conn = DriverManager.getConnection(url, props);
 * }</pre>
 *
 * @deprecated Retained for source compatibility with snowflake-jdbc. Set the {@code authenticator}
 *     connection property to the string value directly.
 */
@Deprecated
public enum AuthenticatorType {
  /** Regular login with username and password via Snowflake, may or may not have MFA */
  SNOWFLAKE,

  /** Not an accepted {@code authenticator} value; native Okta takes an {@code https://} URL. */
  OKTA,

  /** Web-browser-based authenticator for SAML 2.0 compliant service/application */
  EXTERNALBROWSER,

  /** OAuth 2.0 authentication flow */
  OAUTH,

  /** Snowflake JWT token authentication using a private key */
  SNOWFLAKE_JWT,

  /** Not an accepted {@code authenticator} value. */
  ID_TOKEN,

  /** Authenticator to enable token for regular login with MFA */
  USERNAME_PASSWORD_MFA,

  /** OAuth authorization code flow with browser popup */
  OAUTH_AUTHORIZATION_CODE,

  /** OAuth client credentials flow with clientId and clientSecret */
  OAUTH_CLIENT_CREDENTIALS,

  /** Programmatic Access Token (PAT) authentication created in Snowflake */
  PROGRAMMATIC_ACCESS_TOKEN,

  /**
   * Workload identity authentication using existing AWS/GCP/Azure/OIDC workload identity
   * credentials
   */
  WORKLOAD_IDENTITY
}
