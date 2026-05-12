# OAuth Authentication

This document describes how OAuth 2.x authentication works in the universal
driver: the supported flows, the configuration surface exposed by every
wrapper, the cross-driver behavioural contract, end-to-end code examples, and
common troubleshooting.

Implementation lives almost entirely in the Rust core
(`sf_core/src/rest/snowflake/oauth/`); the wrappers (ODBC, Python) only
expose the configuration keys and forward kwargs to the core. The
exhaustive per-driver behavioural matrix (PKCE sizes, token-cache layouts,
gotchas inherited from JDBC/ODBC/Python/.NET/Go/Node) is captured in
[`analysis_feature_oauth.md`](../analysis_feature_oauth.md).

User-facing test scenarios live in
[`tests/definitions/shared/authentication/oauth.feature`](../tests/definitions/shared/authentication/oauth.feature).

---

## 1. Supported flows

| Flow                                  | `authenticator` value          | IdP                  | When to use                                                                                  |
| ------------------------------------- | ------------------------------ | -------------------- | -------------------------------------------------------------------------------------------- |
| Authorization Code (with PKCE S256)   | `OAUTH_AUTHORIZATION_CODE`     | Snowflake / external | Interactive end-user login. Driver opens a browser, listens on a loopback redirect URI.       |
| Client Credentials                    | `OAUTH_CLIENT_CREDENTIALS`     | **External only**    | Non-interactive machine-to-machine. Snowflake's GS does not mint client-credentials tokens.   |
| Pre-acquired access token (legacy)    | `OAUTH`                        | n/a                  | Caller already has an access token (e.g. issued out-of-band); driver forwards it unchanged.   |

`authenticator` is matched case-insensitively — `oauth`, `OAuth`,
`OAUTH_authorization_code`, … all resolve identically.

---

## 2. Configuration parameters

All keys are registered in `sf_core/src/config/param_registry.rs` and exposed
on every wrapper. The canonical name is the lowercase form; ODBC accepts the
`SCREAMING_SNAKE_CASE` alias in connection strings / DSN entries, Python
accepts the lowercase form as a `connect()` kwarg.

| Canonical name                            | ODBC connection-string key                 | Sensitive | Default                                       | Notes |
| ----------------------------------------- | ------------------------------------------ | --------- | --------------------------------------------- | ----- |
| `oauth_client_id`                         | `OAUTH_CLIENT_ID`                          | no        | `LOCAL_APPLICATION` when Snowflake is the IdP | Required for AC/CC against an external IdP. |
| `oauth_client_secret`                     | `OAUTH_CLIENT_SECRET`                      | **yes**   | `LOCAL_APPLICATION` when Snowflake is the IdP | Never logged, never persisted to DSN. |
| `oauth_authorization_url`                 | `OAUTH_AUTHORIZATION_URL`                  | no        | `https://{host}/oauth/authorize`              | AC flow only. |
| `oauth_token_request_url`                 | `OAUTH_TOKEN_REQUEST_URL`                  | no        | `https://{host}/oauth/token-request` (AC)     | Required for CC. Also used to derive the token-cache host key. |
| `oauth_redirect_uri`                      | `OAUTH_REDIRECT_URI`                       | no        | `http://127.0.0.1:<random>`                   | AC flow only. The driver binds to `127.0.0.1` — never `0.0.0.0`. |
| `oauth_scope`                             | `OAUTH_SCOPE`                              | no        | `session:role:<role>`                         | Space-separated scope list. |
| `oauth_enable_single_use_refresh_tokens`  | `OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS`   | no        | `false`                                       | Snowflake-as-IdP only; adds `enable_single_use_refresh_tokens=true` to the token request body. |
| `oauth_disable_pkce`                      | `OAUTH_DISABLE_PKCE`                       | no        | `false`                                       | Python parity escape hatch; all other drivers always run PKCE S256. |
| `oauth_enable_dpop`                       | `OAUTH_ENABLE_DPOP`                        | no        | `false`                                       | Opt-in to RFC 9449 DPoP. JDBC parity feature. |
| `oauth_disable_console_login`             | `OAUTH_DISABLE_CONSOLE_LOGIN`              | no        | `false`                                       | Affects the legacy EXTERNALBROWSER `console_login` form, **not** OAuth flows; carried for JDBC parity. |
| `client_store_temporary_credential`       | `CLIENT_STORE_TEMPORARY_CREDENTIAL`        | no        | `false`                                       | When `true`, the AC flow persists OAuth access/refresh tokens to the OS keyring and short-circuits subsequent connects. |
| `token`                                   | `TOKEN`                                    | **yes**   | —                                             | Legacy `AUTHENTICATOR=OAUTH` only — the pre-acquired access token. |
| `user`                                    | `UID`                                      | no        | —                                             | Required for every flow (sent as `LOGIN_NAME` in `/session/v1/login-request`). |
| `authentication_timeout`                  | `AUTHENTICATION_TIMEOUT`                   | no        | core default (`DEFAULT_AUTHENTICATION_TIMEOUT_SECS`) | End-to-end auth budget covering the loopback wait + token exchange + Snowflake login. |

### Required parameters per flow

- **`OAUTH_AUTHORIZATION_CODE`**: `user`. Everything else has a default
  (Snowflake-as-IdP) — but for an external IdP you almost always set
  `oauth_client_id`, `oauth_client_secret`, `oauth_authorization_url`,
  `oauth_token_request_url`.
- **`OAUTH_CLIENT_CREDENTIALS`**: `user`, `oauth_client_id`,
  `oauth_client_secret`, `oauth_token_request_url`. Missing any of these
  produces a `missing-parameter` error citing the offending key.
- **`OAUTH`** (legacy): `user`, `token`.

### Sensitive keys

`oauth_client_secret` and `token` are redacted from:

- the wrapper's `connection.kwargs` view (`***`),
- every `tracing` / `logging` sink (`****`),
- the ODBC DSN registry — `setup_common::write_dsn_values` refuses to persist
  them.

`SensitiveString` masks any value passed through it as `****` for both
`Display` and `Debug`; the only way to read the raw value is the explicit
`reveal()` method, which is only called at the wire boundary.

---

## 3. Authorization Code flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Driver as Driver (sf_core)
    participant Loop as Loopback listener<br/>(127.0.0.1:&lt;port&gt;)
    participant Browser
    participant IdP as Authorization Server
    participant SF as Snowflake<br/>/session/v1/login-request

    App->>Driver: connect(authenticator=OAUTH_AUTHORIZATION_CODE, ...)
    Driver->>Driver: Check token cache (if client_store_temporary_credential)
    alt Cached access token
        Driver->>SF: AUTHENTICATOR=OAUTH + TOKEN=&lt;cached&gt;
        SF-->>Driver: Session token
    else Cached refresh token only
        Driver->>IdP: POST token endpoint (grant_type=refresh_token)
        IdP-->>Driver: New access (and rotated refresh) token
        Driver->>SF: AUTHENTICATOR=OAUTH + TOKEN=&lt;refreshed&gt;
    else No cache / refresh failed
        Driver->>Driver: Generate PKCE verifier + S256 challenge + state
        Driver->>Loop: Bind 127.0.0.1:&lt;random port&gt;
        Driver->>Browser: open(authorize_url?response_type=code&amp;client_id=...&amp;<br/>code_challenge=...&amp;code_challenge_method=S256&amp;state=...)
        Browser->>IdP: GET /authorize
        IdP-->>Browser: 302 redirect to loopback?code=...&amp;state=...
        Browser->>Loop: GET / with code &amp; state
        Loop-->>Browser: HTML "Authorization completed."
        Loop->>Driver: Validate state, extract code
        Driver->>IdP: POST token endpoint (grant_type=authorization_code,<br/>code, code_verifier, redirect_uri, &lt;DPoP if enabled&gt;)
        IdP-->>Driver: {access_token, refresh_token, expires_in, ...}
        Driver->>Driver: Cache tokens (if enabled)
        Driver->>SF: AUTHENTICATOR=OAUTH + TOKEN=&lt;access_token&gt;
        SF-->>Driver: Session token
    end
    Driver-->>App: Connection ready
```

Key properties:

- **PKCE S256** is on by default. The verifier is generated with a CSPRNG and
  is never persisted — it lives only in process memory until the token
  exchange completes.
- **State parameter** is a 256-bit random value. A mismatch on the loopback
  redirect raises `OAuthError::StateMismatch` with the canonical
  `It might indicate an XSS attack.` wording — SREs grep for it across logs,
  so the message is pinned by unit tests and must not be paraphrased.
- **Loopback binding** is always `127.0.0.1`; binding to `0.0.0.0` is
  rejected. The driver picks a random ephemeral port unless the caller pins
  one via `oauth_redirect_uri=http://127.0.0.1:NNNN/`.
- **Browser launch** uses `xdg-open` / `open` / `cmd /C start` depending on
  the OS. If launching fails, the driver prints a copy/paste fallback URL
  to stdout. Headless ODBC contexts should anticipate this fallback.
- **DPoP** (`oauth_enable_dpop=true`) generates an ES256 P-256 key, signs a
  proof JWT on every token + Snowflake login request, and retries once on a
  server-supplied `DPoP-Nonce` (`use_dpop_nonce` slug). The DPoP private key
  and access token are cached as a bundled JSON blob.
- **Snowflake error codes `390303` (invalid OAuth access token)** and
  **`390318` (expired OAuth access token)** trigger a single retry: the
  driver evicts the cached access token, attempts a refresh-token exchange,
  and replays `/session/v1/login-request`. If the refresh also fails the
  refresh token is evicted and the interactive flow restarts.

---

## 4. Client Credentials flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Driver as Driver (sf_core)
    participant IdP as External IdP token endpoint
    participant SF as Snowflake<br/>/session/v1/login-request

    App->>Driver: connect(authenticator=OAUTH_CLIENT_CREDENTIALS,<br/>oauth_client_id, oauth_client_secret, oauth_token_request_url, ...)
    Driver->>IdP: POST token endpoint<br/>(grant_type=client_credentials, scope, Basic auth)
    IdP-->>Driver: {access_token, expires_in, ...}
    Driver->>SF: AUTHENTICATOR=OAUTH + TOKEN=&lt;access_token&gt;<br/>CLIENT_ENVIRONMENT.OAUTH_TYPE=oauth_client_credentials
    SF-->>Driver: Session token
    Driver-->>App: Connection ready
```

Notes:

- Snowflake itself does **not** support `grant_type=client_credentials`
  (analysis §4). CC therefore *requires* an external IdP and explicit
  `oauth_token_request_url`.
- No browser, no loopback, no PKCE — this is a pure HTTP back-channel
  exchange. The flow is suitable for batch jobs / service accounts.
- Snowflake-side `390303`/`390318` retries call back into the IdP token
  endpoint to obtain a fresh access token (there is no refresh token in CC).
- DPoP follows the same opt-in shape as the AC flow.

---

## 5. Code snippets

### 5.1 Python

```python
import snowflake.connector

# Authorization Code (Snowflake as IdP, defaults to LOCAL_APPLICATION client)
conn = snowflake.connector.connect(
    account="your-account",
    user="alice@example.com",
    authenticator="OAUTH_AUTHORIZATION_CODE",
    role="ANALYST",
    client_store_temporary_credential=True,
)

# Authorization Code (external IdP, e.g. Okta)
conn = snowflake.connector.connect(
    account="your-account",
    user="alice@example.com",
    authenticator="OAUTH_AUTHORIZATION_CODE",
    oauth_client_id="0oa…",
    oauth_client_secret="…",
    oauth_authorization_url="https://example.okta.com/oauth2/v1/authorize",
    oauth_token_request_url="https://example.okta.com/oauth2/v1/token",
    oauth_scope="session:role:ANALYST offline_access",
    oauth_redirect_uri="http://127.0.0.1:8765/",
    client_store_temporary_credential=True,
)

# Client Credentials (external IdP only)
conn = snowflake.connector.connect(
    account="your-account",
    user="service-account",
    authenticator="OAUTH_CLIENT_CREDENTIALS",
    oauth_client_id="…",
    oauth_client_secret="…",
    oauth_token_request_url="https://example.okta.com/oauth2/v1/token",
    oauth_scope="session:role:SVC_ROLE",
)

# Legacy pre-acquired access token
conn = snowflake.connector.connect(
    account="your-account",
    user="alice@example.com",
    authenticator="OAUTH",
    token="ya29.…",
)
```

Legacy `snowflake-connector-python` aliases handled by the wrapper:

- `oauth_token_url` is rewritten to `oauth_token_request_url`.
- `oauth_enable_refresh_tokens`, `oauth_credentials_in_body` and
  `oauth_socket_uri` emit a `DeprecationWarning` and are silently dropped —
  refresh-token reuse is always on (gated by
  `client_store_temporary_credential`), the CC flow always uses HTTP Basic
  for client credentials, and the loopback listener always binds to the
  redirect URI host.

### 5.2 ODBC

DSN-style connection string (Windows / unixODBC):

```
DRIVER={SnowflakeUniversalDriver};
SERVER=your-account.snowflakecomputing.com;
UID=alice@example.com;
AUTHENTICATOR=OAUTH_AUTHORIZATION_CODE;
OAUTH_CLIENT_ID=0oa…;
OAUTH_CLIENT_SECRET=…;
OAUTH_AUTHORIZATION_URL=https://example.okta.com/oauth2/v1/authorize;
OAUTH_TOKEN_REQUEST_URL=https://example.okta.com/oauth2/v1/token;
OAUTH_SCOPE=session:role:ANALYST offline_access;
OAUTH_REDIRECT_URI=http://127.0.0.1:8765/;
CLIENT_STORE_TEMPORARY_CREDENTIAL=true;
```

Client Credentials:

```
DRIVER={SnowflakeUniversalDriver};
SERVER=your-account.snowflakecomputing.com;
UID=service-account;
AUTHENTICATOR=OAUTH_CLIENT_CREDENTIALS;
OAUTH_CLIENT_ID=…;
OAUTH_CLIENT_SECRET=…;
OAUTH_TOKEN_REQUEST_URL=https://example.okta.com/oauth2/v1/token;
OAUTH_SCOPE=session:role:SVC_ROLE;
```

Legacy pre-acquired token:

```
DRIVER={SnowflakeUniversalDriver};
SERVER=your-account.snowflakecomputing.com;
UID=alice@example.com;
AUTHENTICATOR=OAUTH;
TOKEN=ya29.…;
```

The setup dialog refuses to persist `OAUTH_CLIENT_SECRET` and `TOKEN` to the
DSN registry — they must always be supplied at connect time (or sourced from
the OS keyring through `CLIENT_STORE_TEMPORARY_CREDENTIAL=true`).

### 5.3 Rust (sf_core)

`sf_core` is consumed via FFI by the wrappers; embedded Rust callers wire
the OAuth flow through the same `LoginMethod` enum that the wrappers
populate:

```rust
use sf_core::config::rest_parameters::{
    LoginMethod, OAuthAuthorizationCodeConfig, OAuthClientCredentialsConfig,
};

let method = LoginMethod::OAuthAuthorizationCode(OAuthAuthorizationCodeConfig {
    username: "alice@example.com".into(),
    client_id: "0oa…".into(),
    client_secret: "…".into(),
    authorization_url: Some("https://example.okta.com/oauth2/v1/authorize".parse()?),
    token_url: Some("https://example.okta.com/oauth2/v1/token".parse()?),
    redirect_uri: None, // ephemeral 127.0.0.1:<random>
    scope: Some("session:role:ANALYST offline_access".into()),
    enable_single_use_refresh_tokens: false,
    disable_pkce: false,
    enable_dpop: false,
    client_store_temporary_credential: true,
    authentication_timeout_secs: 120,
});
```

---

## 6. Token caching

When `client_store_temporary_credential=true` (and the OS provides a
keyring backend), the driver caches:

- **Access token** under `(host, user, OAUTH_ACCESS_TOKEN)`.
- **Refresh token** under `(host, user, OAUTH_REFRESH_TOKEN)`.
- **DPoP-bundled access token** (DPoP key + access token JSON) under
  `(host, user, OAUTH_ACCESS_TOKEN)` when DPoP is enabled.

The cache host key is derived from `oauth_token_request_url` (falling back to
the Snowflake server URL) so that tokens from different IdPs do not collide
on the same Snowflake account.

Eviction rules:

- Snowflake `390303` / `390318` evicts the access token and triggers a
  single refresh attempt.
- A failed refresh evicts the refresh token and restarts the interactive
  flow (AC) or returns the IdP error (CC).
- The `oauth2` crate's single-use-refresh-token rotation rewrites the cached
  refresh token on every successful refresh.

---

## 7. Logging

OAuth code paths follow the project-wide logging contract:

- **INFO** level: lifecycle milestones — `Starting OAuth authorization code flow`,
  `OAuth authorization code flow served from cache`,
  `Refreshed OAuth access token`,
  `Cached OAuth access token for future use`,
  `Starting OAuth client credentials flow`,
  `OAuth client credentials flow completed`.
- **DEBUG** level: structured fields — endpoint URLs, response status codes,
  PKCE challenge method, scope, refresh-token presence flag.
- **NEVER**: access tokens, refresh tokens, client secrets, PKCE verifier,
  IdP authorization code, DPoP private key. `SensitiveString` redacts these
  to `****` at `Display` / `Debug`; `OAuthError` variants intentionally do
  not carry any of these values.

A unit-tested redaction guard (`oauth_error_display_and_debug_never_leak_secret_canaries`
in `sf_core/src/rest/snowflake/oauth/error.rs`) pins this contract.

---

## 8. Troubleshooting

| Symptom                                                                 | Likely cause                                                                                                | What to check                                                                                  |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `Identity Provider did not provide expected state parameter! It might indicate an XSS attack.` | The loopback `state=` query parameter does not match what the driver sent. Possible XSS or stale browser tab. | Close any stray browser tabs pointing at `127.0.0.1:<port>/?...` and retry. Never paste a redirect URL across sessions. |
| `OAuth browser authorization timed out`                                 | No redirect arrived before the auth budget (`authentication_timeout`) elapsed.                              | Increase `authentication_timeout`, or use `OAUTH_CLIENT_CREDENTIALS` if the host has no browser. |
| `Failed to launch OS browser for OAuth authorization`                   | `xdg-open` / `open` / `cmd /C start` is missing or returned a non-zero exit code.                            | The driver prints a paste-fallback URL; copy it into a browser manually. On Linux servers install `xdg-utils`. |
| `Failed to bind loopback listener for OAuth redirect`                   | The pinned `oauth_redirect_uri` port is already in use (or the user lacks permission).                       | Pick another port or omit `oauth_redirect_uri` to let the driver bind ephemerally.             |
| `OAuth token exchange failed with HTTP 401`                             | Wrong `oauth_client_id` / `oauth_client_secret`, or the IdP requires a different `code_challenge_method`.    | Verify credentials with the IdP admin; for Python you can set `oauth_disable_pkce=true` if the IdP rejects PKCE. |
| `OAuth token response did not include an access_token`                  | The IdP responded `200 OK` with a body that lacks `access_token`.                                            | Inspect the IdP response (check IdP logs); usually a misconfigured client (e.g. `response_type` mismatch). |
| `OAuth refresh-token exchange failed`                                   | Refresh token revoked, expired, or rotated out from under us.                                                | The driver auto-evicts and falls back to the interactive flow. Re-authenticate.                 |
| `OAuth HTTP transport error`                                            | Network / TLS / proxy issue talking to the IdP token endpoint.                                               | Verify proxy settings, IdP TLS chain, DNS. Look for `tracing` `error = %e` on the same span.    |
| Connection fails with `390303` / `390318` after a refresh               | Both access and refresh tokens were rejected by Snowflake.                                                  | Tokens were minted for a different `host` / `user`; re-run the interactive flow.                |
| `is_oauth_authenticator` returns `false` for a custom value             | `authenticator` is misspelled (e.g. `OAUH_AUTHORIZATION_CODE`).                                              | Use one of `OAUTH`, `OAUTH_AUTHORIZATION_CODE`, `OAUTH_CLIENT_CREDENTIALS` (case-insensitive).  |

Useful environment toggles when diagnosing OAuth issues:

- `RUST_LOG=sf_core::rest::snowflake::oauth=debug` — turns on DEBUG logging
  for every OAuth code path.
- ODBC: set `LOG_LEVEL=DEBUG` in the DSN / connection string; the wrapper
  redacts `OAUTH_CLIENT_SECRET` and `TOKEN` automatically so DEBUG dumps of
  the parameter map remain safe to share.

---

## 9. Test coverage

| Layer       | Location                                                                    | Notes                                                                                       |
| ----------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Gherkin     | [`tests/definitions/shared/authentication/oauth.feature`](../tests/definitions/shared/authentication/oauth.feature) | Cross-driver scenarios (integration + E2E); steps appear as comments in each driver's tests. |
| Rust unit   | `sf_core/src/rest/snowflake/oauth/`                                         | PKCE generator, state validator, response parsing, DPoP nonce retry, redaction guards.       |
| Rust integ. | `sf_core/tests/integration/oauth/`                                          | Wiremock-driven IdP + Snowflake mock; covers AC, CC, refresh, DPoP, error mapping.           |
| Rust E2E    | `sf_core/tests/e2e/authentication/oauth.rs` (feature `auth_oauth_e2e`)      | Real Snowflake account + (optional) real browser.                                            |
| ODBC integ. | `odbc_tests/tests/integration/authentication/oauth.cpp`                     | Connection-string parsing, required-param validation, DSN persistence policy, redaction.     |
| ODBC E2E    | `odbc_tests/tests/e2e/authentication/oauth.cpp`                             | Real Snowflake; browser AC path gated by `SNOWFLAKE_OAUTH_E2E_BROWSER=1`.                    |
| Python int. | `python/tests/integ/authentication/test_oauth.py`                           | kwarg shape, alias rewriting, secret redaction.                                              |
| Python E2E  | `python/tests/e2e/authentication/test_oauth.py`                             | Real Snowflake; browser AC path gated by `SNOWFLAKE_OAUTH_E2E_BROWSER=1`.                    |

---

## 10. References

- [`analysis_feature_oauth.md`](../analysis_feature_oauth.md) — exhaustive
  cross-driver analysis: configuration matrix, state machines, gotchas,
  error taxonomy.
- [RFC 6749](https://datatracker.ietf.org/doc/html/rfc6749) — The OAuth 2.0
  Authorization Framework.
- [RFC 7636](https://datatracker.ietf.org/doc/html/rfc7636) — PKCE.
- [RFC 8252](https://datatracker.ietf.org/doc/html/rfc8252) — OAuth 2.0 for
  Native Apps (loopback redirect recommendations).
- [RFC 9449](https://datatracker.ietf.org/doc/html/rfc9449) — Demonstrating
  Proof-of-Possession (DPoP).
- [`oauth2` crate](https://crates.io/crates/oauth2) — the underlying Rust
  primitive used by `sf_core`.
