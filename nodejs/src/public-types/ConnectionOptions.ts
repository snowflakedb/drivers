import type { DataType, RowMode } from '../query-result/types.js';

export type ConnectionOptions = {
  /**
   * Your account identifier.
   */
  account?: string;

  /**
   * Specifies a fully-qualified endpoint for connecting to Snowflake.
   */
  accessUrl?: string;

  /**
   * Host address to which the driver should connect.
   */
  host?: string;

  /**
   * TCP port of the Snowflake endpoint. Used with `host` when {@link accessUrl}
   * is not set.
   */
  port?: number | string;

  /**
   * Scheme of the Snowflake endpoint, `https` or `http`. Used with `host` when
   * {@link accessUrl} is not set.
   *
   * @default "https"
   */
  protocol?: 'http' | 'https';

  /**
   * The login name for your Snowflake user or your Identity Provider (e.g. your login name for Okta).
   */
  username?: string;

  /**
   * Password for the user.
   */
  password?: string;

  /**
   * Specifies the authenticator to use for verifying user login credentials.
   *
   * Available options:
   * * `SNOWFLAKE` - Default authenticator that uses basic username/password authentication
   * * `EXTERNALBROWSER` - Browser-based SSO authentication through your identity provider
   * * `SNOWFLAKE_JWT` - Key pair authentication using a private key and public key pair
   * * `OAUTH` - OAuth authentication using a token obtained from OAuth flow
   * * `USERNAME_PASSWORD_MFA` - Username/password with multi-factor authentication
   * * `PROGRAMMATIC_ACCESS_TOKEN` - Using a programmatic access token set in {@link token} or {@link password}
   * * `OAUTH_AUTHORIZATION_CODE` - OAuth authorization code flow for web applications
   * * `OAUTH_CLIENT_CREDENTIALS` - OAuth client credentials flow for service-to-service auth
   * * `WORKLOAD_IDENTITY` - Workload identity authentication
   * * `https://<okta_account_name>.okta.com` - Native SSO authentication through Okta
   */
  authenticator?: string;

  /**
   * The default virtual warehouse to use for the session after connecting. Used for performing queries, loading data, etc.
   */
  warehouse?: string;

  /**
   * The default database to use for the session after connecting.
   */
  database?: string;

  /**
   * The default schema to use for the session after connecting.
   */
  schema?: string;

  /**
   * The default security role to use for the session after connecting.
   */
  role?: string;

  /**
   * When authenticator=SNOWFLAKE_JWT, specifies the private key (in PEM format).
   *
   * If the key is encrypted, use {@link privateKeyPass} to provide the passphrase.
   */
  privateKey?: string;

  /**
   * When authenticator=SNOWFLAKE_JWT, specifies the local path to the private key file (in PEM
   * format).
   *
   * If the key is encrypted, use {@link privateKeyPass} to provide the passphrase.
   */
  privateKeyPath?: string;

  /**
   * When authenticator=SNOWFLAKE_JWT, specifies the passcode to decrypt the encrypted private key
   * passed in {@link privateKey} or {@link privateKeyPath}.
   */
  privateKeyPass?: string;

  /**
   * When authenticator=WORKLOAD_IDENTITY, specifies the identity provider. Available options:
   * * AWS - Finds AWS credentials in the environment. Supports two attestation methods:
   *   * `GetCallerIdentity` (default) - encodes a SigV4-signed `GetCallerIdentity` request as the token
   *   * `GetWebIdentityToken` - obtains a signed JWT token, enabled via {@link workloadIdentityAwsUseOutboundToken}
   * * AZURE - Gets a JWT token for the Azure managed identity
   * * GCP - Gets a JWT token from the GCP metadata server
   * * OIDC - Reads JWT token from {@link token}
   */
  workloadIdentityProvider?: 'AWS' | 'AZURE' | 'GCP' | 'OIDC';

  /**
   * When workloadIdentityProvider=AWS, overrides the AWS STS host used by the attestation
   * flows, so AWS partitions unknown to the driver can be reached.
   *
   * Accepts a bare host (`sts.example.com`), a host with a port (`sts.example.com:8443`) or a
   * full URL (`https://sts.example.com`). The value is used as given - no partition suffix is
   * appended or rewritten - and `https` is assumed when no scheme is provided.
   *
   * When not set, the driver derives the STS host from the AWS region.
   *
   * @example
   * // us-isob-east-1
   * workloadIdentityHost: 'sts.sc2s.sgov.gov'
   */
  workloadIdentityHost?: string;

  /**
   * When authenticator=WORKLOAD_IDENTITY, specifies a chain of identities to impersonate in order.
   * This lets a workload authenticate as a different identity than its default attached one.
   *
   * Supported for:
   * * AWS and GCP - one or more identities.
   * * AZURE - exactly one service principal client ID. Azure supports only a single
   *   impersonation step.
   *
   * Not supported for OIDC.
   */
  workloadIdentityImpersonationPath?: string[];

  /**
   * When workloadIdentityProvider=AZURE, customizes Azure Entra Id Resource
   */
  workloadIdentityAzureEntraIdResource?: string;

  /**
   * When workloadIdentityProvider=AZURE, customizes Azure Managed Identity Client Id
   */
  workloadIdentityAzureClientId?: string;

  /**
   * When workloadIdentityProvider=AWS, selects the AWS attestation method.
   *
   * AWS WIF supports two methods:
   * * `GetCallerIdentity` (default, `false`) - the driver encodes a SigV4-signed
   *   `GetCallerIdentity` request as the attestation token.
   * * `GetWebIdentityToken` (`true`) - the driver calls STS `GetWebIdentityToken` and
   *   forwards a standards-based JWT instead. This provides stateless token verification and
   *   compatibility with AWS outbound identity federation. It requires the AWS IAM role to have
   *   `sts:GetWebIdentityToken` permission and the Snowflake service user to be configured with an
   *   `ISSUER`.
   *
   * See {@link https://docs.snowflake.com/en/user-guide/workload-identity-federation#label-wif-aws-upgrade-jwt}
   * for setup instructions.
   *
   * @default false
   */
  workloadIdentityAwsUseOutboundToken?: boolean;

  /**
   * Default shape of the rows returned by statements on this connection:
   * * `object` - each row is an object keyed by column name
   * * `array` - each row is an array of values in column order
   * * `object_with_renamed_duplicated_columns` - like `object`, but a repeated column name gets
   *   a `_2`, `_3`, ... suffix so no value is overwritten
   *
   * A statement can override it with {@link StatementOption.rowMode}.
   *
   * @default "object"
   */
  rowMode?: RowMode;

  /**
   * Data types returned as strings instead of their native JavaScript type for every statement
   * on this connection.
   *
   * A statement can override it with {@link StatementOption.fetchAsString}.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume
   * @default []
   */
  fetchAsString?: DataType[];

  /**
   * When `true`, Snowflake INTEGER columns (NUMBER with scale 0) are returned as JavaScript
   * `bigint`, which can store larger values than `number`.
   *
   * When set, the connection starts with this value as its `JS_TREAT_INTEGER_AS_BIGINT` session
   * parameter.
   *
   * @default User's JS_TREAT_INTEGER_AS_BIGINT value
   */
  jsTreatIntegerAsBigInt?: boolean;

  /**
   * Controls how a SQL NULL is rendered for columns that {@link fetchAsString}
   * returns as strings. A column returned as its native JavaScript type is
   * unaffected — its NULL is always `null`.
   *
   * When `true`, a NULL in a string-rendered column comes back as the string
   * `'NULL'`; when `false`, it stays `null`.
   *
   * TODO(BD#18): some data types do not route their NULL through this option yet
   * (see `nodejs/BehaviorDifferences.yaml`). Document the per-type behavior here
   * once every data type is implemented.
   *
   * @default true
   */
  representNullAsStringNull?: boolean;

  /**
   * Maximum number of rows buffered by the stream returned from `statement.streamRows()`.
   * Passed as the `highWaterMark` of the underlying Node.js `Readable`.
   *
   * This does not bound the memory used for fetched result chunks; that is controlled by the
   * `CLIENT_MEMORY_LIMIT` and `CLIENT_PREFETCH_THREADS` session parameters.
   *
   * @default 10
   */
  rowStreamHighWaterMark?: number;

  /**
   * Decides when large bulk {@link StatementOption.binds} are uploaded to a temporary stage instead
   * of being sent with the request.
   *
   * The driver counts the total number of bound values (rows * columns). If the count is greater
   * than this number, the driver uploads the values to a stage as a CSV file, and the query reads
   * them from that file. If the count is this number or lower, the values are sent inline in the
   * request. Uploading is faster for big batches; inline is simpler for small ones. Single-row
   * binds are always sent inline and ignore this option.
   *
   * When set, the connection starts with this value as its custom
   * `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` session parameter.
   *
   * @default User's CLIENT_STAGE_ARRAY_BINDING_THRESHOLD value
   */
  arrayBindingThreshold?: number;

  /**
   * Name of the connection profile to use from `connections.toml` or `config.toml`.
   *
   * The driver reads a profile only in these cases:
   * - `connectionName` is set. Other connection options passed next to it replace the
   *   profile's values for those options.
   * - `createConnection()` gets no options. The driver uses the profile named by:
   *   1. the `SNOWFLAKE_DEFAULT_CONNECTION_NAME` environment variable
   *   2. the `default_connection_name` key in `config.toml`
   *   3. `"default"`
   *
   * In all other cases, no profile is read, and only the options you pass are used.
   */
  connectionName?: string;

  /**
   * Optional string that can be used to tag queries and other SQL statements executed within a
   * connection. The tags are displayed in the output of the QUERY_HISTORY, QUERY_HISTORY_BY_*
   * functions.
   */
  queryTag?: string;

  /**
   * Name of the application that uses the driver. It is sent to Snowflake at login.
   */
  application?: string;

  /**
   * By default, client connections typically time out approximately 3-4 hours after the most recent query was executed.
   *
   * @default false
   */
  clientSessionKeepAlive?: boolean;

  /**
   * Sets the frequency (interval in seconds) between heartbeat messages.
   */
  clientSessionKeepAliveHeartbeatFrequency?: number;

  /**
   * When true, the session is not destroyed on the server side when the connection
   * is closed. This allows async queries to continue running after disconnect.
   * Any unfinished queries will continue to live in Snowflake and consume credits
   * until they finish.
   *
   * @default false
   */
  serverSessionKeepAlive?: boolean;

  /**
   * Enables caching of MFA, external-browser ID, and OAuth tokens.
   *
   * See {@link https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-authenticate#authentication-token-caching}
   *
   * @default true
   */
  clientStoreTemporaryCredential?: boolean;

  /**
   * Specifies the token to use for authentication. Set this option if you set the authenticator option to:
   * * OAUTH
   * * PROGRAMMATIC_ACCESS_TOKEN
   * * WORKLOAD_IDENTITY
   */
  token?: string;

  /**
   * Specifies the path to a file containing the token for authentication.
   *
   * This option works similarly to the `token` property, but retrieves the token value by reading the specified file from disk.
   *
   * If `token` is not provided, the driver will attempt to read the token from this file for suitable token-based authenticators.
   *
   * If both `token` and `tokenFilePath` are set, `token` takes precedence and this file path will be ignored.
   */
  tokenFilePath?: string;

  /**
   * When true, the driver does not keep a client-side query context cache.
   * No context is sent on requests, and context returned by the server is ignored.
   *
   * @default false
   */
  disableQueryContextCache?: boolean;

  /**
   * Enables Certificate Revocation List (CRL) validation.
   *
   * When `ENABLED` is set, it fails if the certificate is revoked or if any error occurs (network, parsing, etc.).
   * When `ADVISORY` is set, it fails only if the certificate is revoked.
   *
   * @default "DISABLED"
   */
  certRevocationCheckMode?: 'DISABLED' | 'ENABLED' | 'ADVISORY';

  /**
   * Allows to connect when certificate doesn't have CRL URLs (cRLDistributionPoints)
   *
   * This option applies only when certRevocationCheckMode is `ADVISORY` or `ENABLED`
   *
   * @default false
   */
  crlAllowCertificatesWithoutCrlURL?: boolean;

  /**
   * Enable CRL caching in memory.
   *
   * This option applies only when certRevocationCheckMode is `ADVISORY` or `ENABLED`
   *
   * @default true
   */
  crlInMemoryCache?: boolean;

  /**
   * Enable CRL caching on disk. Disk read/write failures are ignored.
   *
   * This option applies only when certRevocationCheckMode is `ADVISORY` or `ENABLED`
   *
   * @default true
   */
  crlOnDiskCache?: boolean;

  /**
   * Overrides {@link ConfigureOptions.crlDownloadTimeout} for this connection, in milliseconds.
   * The value is rounded down to whole seconds; a positive value under 1000 becomes 1 second.
   */
  crlDownloadTimeout?: number;

  /**
   * Overrides {@link ConfigureOptions.crlDownloadMaxSize} for this connection, in bytes.
   */
  crlDownloadMaxSize?: number;

  /**
   * Overrides {@link ConfigureOptions.crlCacheValidityTime} for this connection, in milliseconds.
   * The value is rounded down to whole seconds; a positive value under 1000 becomes 1 second.
   */
  crlCacheValidityTime?: number;

  /**
   * Overrides {@link ConfigureOptions.crlCacheDir} for this connection.
   */
  crlCacheDir?: string;

  /**
   * Per-attempt timeout of login, query, and other requests, in milliseconds. The value is
   * rounded down to whole seconds; a positive value under 1000 becomes 1 second. 0 and negative
   * values behave like an omitted timeout.
   *
   * @default login attempts time out after 30 seconds; other requests have no per-request timeout
   */
  timeout?: number;

  /**
   * Deadline for the whole login, including retries, in seconds. 0 and negative values mean
   * login has no deadline.
   *
   * @default 120
   */
  retryTimeout?: number;

  /**
   * Time budget for external-browser SSO, native Okta, and OAuth authorization-code login, in
   * milliseconds.
   * The value is rounded down to whole seconds; a positive value under 1000 becomes 1 second.
   *
   * @default 120000
   */
  browserActionTimeout?: number;

  /**
   * When authenticator=EXTERNALBROWSER, the port of the local server that
   * receives the authentication callback. The listener binds `127.0.0.1`.
   * `0` or omitted lets the OS pick a random available port. The bound port
   * is sent as `BROWSER_MODE_REDIRECT_PORT`. This does not change the callback
   * host or path, and it is not `oauthRedirectUri`.
   *
   * @default 0 (random available port)
   */
  browserRedirectPort?: number;

  /**
   * Proxy URL (`[scheme://][user:pass@]host[:port]`). `proxyHost`, `proxyPort`,
   * `proxyProtocol`, `proxyUser`, and `proxyPassword` override matching URL
   * components when both are set.
   */
  proxy?: string;

  /**
   * Hostname of the proxy server. Traffic to Snowflake is sent here instead of
   * directly to the account host.
   */
  proxyHost?: string;

  /**
   * Port of the proxy server.
   */
  proxyPort?: number;

  /**
   * Username for an authenticated proxy server.
   */
  proxyUser?: string;

  /**
   * Password for the user specified by {@link proxyUser}.
   */
  proxyPassword?: string;

  /**
   * Protocol used to connect to the proxy server (`http` or `https`).
   *
   * @default "http"
   */
  proxyProtocol?: string;

  /**
   * Hosts the driver connects to directly, bypassing the proxy.
   *
   * Separate entries with commas. `*` bypasses every host only when it is the
   * whole entry. A bare domain matches that host and its subdomains, so
   * `snowflakecomputing.com` bypasses `account.snowflakecomputing.com`.
   *
   * @example
   * noProxy: 'localhost,snowflakecomputing.com'
   */
  noProxy?: string;

  /**
   * The 6-digit TOTP code for `USERNAME_PASSWORD_MFA` authenticator. When this is
   * omitted the driver sends a Duo push instead.
   */
  passcode?: string;

  /**
   * When true, the TOTP code is the last six digits of `password` and the driver
   * splits it off before login.
   *
   * @default false
   */
  passcodeInPassword?: boolean;

  /**
   * Value of `client id` provided by the identity provider for Snowflake integration.
   */
  oauthClientId?: string;

  /**
   * Value of the `client secret` provided by the identity provider for Snowflake integration.
   */
  oauthClientSecret?: string;

  /**
   * Redirect URI registered with the identity provider for the authorization-code flow.
   */
  oauthRedirectUri?: string;

  /**
   * Identity Provider endpoint that starts the authorization-code flow.
   */
  oauthAuthorizationUrl?: string;

  /**
   * Identity Provider endpoint supplying the access tokens to the driver.
   */
  oauthTokenRequestUrl?: string;

  /**
   * When true, the authorization-code token request includes
   * `enable_single_use_refresh_tokens=true`. Snowflake as the IdP only.
   *
   * @default false
   */
  oauthEnableSingleUseRefreshTokens?: boolean;

  /**
   * Replaces the system-browser launch used by `EXTERNALBROWSER` SSO and
   * `OAUTH_AUTHORIZATION_CODE`. The driver still binds the loopback
   * listener and waits for the IdP redirect; this function only receives
   * the URL that would otherwise be opened. The callback must return or
   * throw; a function that never returns holds a driver thread until
   * process exit, and login timeout does not unblock it. Login waits for
   * that synchronous return or throw. A returned Promise is not awaited
   * (an `async` callback that throws becomes a rejected Promise and is not
   * observed). Complete any loopback redirect async so login can keep
   * waiting.
   */
  openExternalBrowserCallback?: (url: string) => void | Promise<void>;
};
