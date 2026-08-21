# Authentication

Login flows, authenticator values, session-token lifecycle, and the on-disk
token cache. Part of the [troubleshooting deep-dive](../index.md); for the
task-oriented entry point see the
[Troubleshooting Runbook](../../troubleshooting-runbook.md) (the
**Auth / SSO / key-pair failure** row in
[Appendix A](../../troubleshooting-runbook.md#appendix-a--symptom--action)
points here).

> Never paste tokens, passwords, private keys, or passcodes into a ticket. The
> driver redacts them in its own logs (router
> [§1.4](../../troubleshooting-runbook.md#14-secret-redaction)); your repro
> scripts and shell history do not.

---

## How login works

A connect issues a single `POST /session/v1/login-request`. The request body is
assembled from the resolved credential (`sf_core/src/rest/snowflake/auth.rs`);
the authenticator value chosen by the caller selects which credential fields are
required and how the token is produced (`sf_core/src/config/rest_parameters.rs`,
`LoginMethod::from_settings`). A successful response returns a **session token**
(short-lived, used on every request) and a **master token** (used to renew the
session token) — see [Session expiry & renewal](#session-expiry--renewal) below.

---

## Authenticator values

The `authenticator` connection parameter selects the login flow. It is
**case-insensitive**. If it is set to something outside the set below the driver
rejects the connection up front with an *invalid parameter value* error listing
the allowed values:

> Allowed values are `snowflake`, `snowflake_jwt`, `snowflake_password`,
> `programmatic_access_token`, `username_password_mfa`, `externalbrowser`,
> `oauth`, `oauth_client_credentials`, `oauth_authorization_code`,
> `workload_identity` — or an `https://` URL for native Okta SSO.

| `authenticator` | Flow | Required (beyond `account`) | Notes |
|---|---|---|---|
| `snowflake` / `snowflake_password` / *(omitted)* | Username + password | `user`, `password` | Default when `authenticator` is unset. |
| `snowflake_jwt` | Key-pair (JWT) | `user`, one of `private_key` / `private_key_file` | Auto-selected when a private-key parameter is present even if `authenticator` is omitted. `private_key_password` decrypts an encrypted PEM. |
| `programmatic_access_token` | PAT | `token` | `user` optional. |
| `username_password_mfa` | Password + MFA | `user`, `password`, and `passcode` **or** `passcodeInPassword=true` | TOTP passcode; see [MFA](#symptom-mfa-passcode-not-accepted). |
| `oauth` | Pre-acquired OAuth access token | `token` | `user` optional. Token is a bearer **access** token, not a refresh token. |
| `oauth_authorization_code` | OAuth authorization-code flow | `oauth_client_id`, `oauth_client_secret`, `oauth_authorization_url`, `oauth_token_request_url` | Opens a browser; driver exchanges the code for a token. `oauth_redirect_uri`, `oauth_scope` optional. |
| `oauth_client_credentials` | OAuth client-credentials flow | `oauth_client_id`, `oauth_client_secret`, `oauth_token_request_url` | **External IdP only** — no browser. |
| `externalbrowser` | Browser SSO (IdP-initiated) | `user` | Opens the system browser for the federated login. |
| *(an `https://…` URL)* | Native Okta SSO | `user` (or `okta_username`), the Okta URL **as** the `authenticator` value | `authentication_timeout` bounds the wait; `disable_saml_url_check` relaxes the SAML endpoint check. |
| `workload_identity` | Workload Identity Federation | `workload_identity_provider` (`AWS` / `AZURE` / `GCP` / `OIDC`) | For `OIDC`, also pass the pre-acquired token (`token`); `AZURE` may need `workload_identity_entra_resource`; optional `workload_identity_impersonation_path`. |

Session-token reconnect (a previously issued `session_token` + `master_token`)
is detected before the table above and does not use `authenticator`.

---

## Troubleshooting

### Symptom: 401 / "Incorrect username or password"

**Diagnosis:**
1. Confirm `user` and `account` have no trailing whitespace and the correct
   region/segment. The endpoint is derived from `account` unless `host` is set
   explicitly.
2. For password auth, pass the password as a **raw** string — do not URL-encode
   it yourself.
3. Capture logs (router [§1](../../troubleshooting-runbook.md#1-troubleshooting-logs))
   and find the login HTTP call to confirm the endpoint actually hit.

**Resolution:** Correct the credentials. For MFA, ensure a `passcode` is present
(or `passcodeInPassword=true`).

---

### Symptom: key-pair (JWT) auth fails — "JWT token is invalid" / `JWT_TOKEN_INVALID_EXPIRATION_TIME`

**Diagnosis:**
1. Verify the signing private key matches the public key registered on the user
   (`DESC USER <user>` → `RSA_PUBLIC_KEY_FP` fingerprint).
2. Check for **clock skew** — the driver stamps `iat`/`exp` from the local
   clock. If it is more than a few tens of seconds off, the token can be invalid
   before it is even sent. Verify with `date -u`.
3. If the PEM is encrypted, confirm `private_key_password` is set and correct.

**Proxy latency / slow tunnel (no clock skew).** The driver signs a
**short-lived** JWT (validity is on the order of a minute) and stamps a fresh
proof on every send, including retries. If a corporate proxy takes a long time
to establish the HTTPS tunnel (CONNECT handshake, TLS inspection, proxy auth),
the token can expire **in transit** with correct clocks on both ends — the
server sees near-zero processing time but the token arrives past its `exp`.
Measure tunnel setup in isolation:

```sh
time curl -x <proxy_host>:<proxy_port> -sv \
  https://<account>.snowflakecomputing.com/ -o /dev/null 2>&1 \
  | grep -Ei "connected|ssl|tls"
```

If tunnel setup alone is slow, bypass the proxy for the Snowflake host or
pre-warm the tunnel with a cheap HTTPS request before connecting. See
[proxy-tls.md](crl-tls/proxy-tls.md).

Key-pair parsing lives in `sf_core/src/config/private_key.rs`.

---

### Symptom: PAT rejected — "Programmatic access token is invalid or expired"

**Diagnosis:**
1. PATs have a bounded lifetime set at creation — confirm it has not expired.
2. A PAT is tied to its owning user and role scope; confirm they match how you
   are connecting.

---

### Symptom: OAuth token rejected — 390301

**Diagnosis:**
1. Confirm the value in `token` is an OAuth **access** token, not a refresh
   token.
2. For the browser (`oauth_authorization_code`) and client-credentials
   (`oauth_client_credentials`) flows, verify `oauth_client_id` /
   `oauth_client_secret` / the URL parameters match the IdP integration.
3. OAuth access tokens are short-lived (minutes to hours) — re-acquire and
   retry.

The client-credentials flow is **external-IdP only** and never opens a browser;
the authorization-code flow does open one and exchanges the returned code at
`oauth_token_request_url`.

---

### Symptom: MFA passcode not accepted

**Diagnosis:**
1. With `authenticator=username_password_mfa`, supply the TOTP either as
   `passcode`, or appended to the password with `passcodeInPassword=true`.
2. Passcodes are single-use and time-boxed — a stale code fails; generate a
   fresh one.

---

### Symptom: external-browser / Okta SSO — browser opens then login fails, or "Account not found"

**Diagnosis:**
1. **Account underscore → hyphen.** With `externalbrowser` or an Okta
   authenticator, an account identifier that contains an underscore (`_`) must
   be written with a hyphen (`-`) in the `account` parameter — e.g. account
   `my_org` → `account="my-org"`. Password auth is not affected.
2. For native Okta, the `authenticator` value **is** the Okta `https://…` URL;
   set `okta_username` if the Okta login differs from the Snowflake `user`.
3. If the SSO wait times out, raise `authentication_timeout`.
4. External-browser SSO needs a reachable local browser and loopback — it does
   not work on a headless host without extra setup.

---

### Symptom: "Account not found" / wrong endpoint

**Diagnosis:**
1. Check `account`. For PrivateLink the identifier includes the PrivateLink
   segment — see [privatelink.md](privatelink.md).
2. The endpoint is `account` + `.snowflakecomputing.com` unless `host` / `port`
   override it — verify any overrides.

---

## Session expiry & renewal

A login returns two tokens: a short-lived **session token** sent on every
request, and a longer-lived **master token** used to renew it. When the session
token expires the driver renews it automatically using the master token; you do
**not** re-run the full login flow per request.

The session-lifecycle error codes are distinct, and the code tells you whether the
driver recovers on its own or you must reconnect:

| Code | Meaning | What it means for you |
|---|---|---|
| `390112` | Session token expired | Renewed automatically from the master token — normally transparent; you should not see it surface. |
| `390111` | Session gone — the server no longer has this session | Renewal cannot help; establish a new connection. |
| `390114` | Master token expired ("Authentication token has expired") | The renewal credential itself is gone; a full re-login (reconnect) is required. |

**Symptom — login succeeds but the next call fails with a session error
(`390114` "Authentication token has expired", or `390111` "session no longer
exists").** This is a session-lifecycle issue, not a credential issue:

- The **master token** itself has expired (long-idle connection) — reconnect.
- The session was terminated or aged out server-side (`390111`, e.g. an
  administrator ended it) — reconnect.
- A load balancer or proxy is not preserving affinity/cookies across the renewal
  call — capture logs and check whether the renew request reaches Snowflake.
- The client clock jumped — see the clock-skew note above.

### Keeping an idle session alive (heartbeat)

When `CLIENT_SESSION_KEEP_ALIVE=true` (default **off**), the driver runs a
background heartbeat (`POST /session/heartbeat`) that renews the session token
before it expires, so a long-idle connection stays usable. The interval is
derived from the master token's validity — roughly every 15 min to 1 h (capped at
1 h) at the 4-hour server default — and can be pinned with
`CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` (seconds).

**Symptom — "Session no longer exists" / an auth failure after a long idle
period.** Either keep-alive is off (the default), or the **master** token itself
expired and the heartbeat gave up — a `MasterTokenExpired` response exits the
heartbeat task, and the session is then unrecoverable (reconnect).

- For interactive or long-lived apps that sit idle between queries, set
  `CLIENT_SESSION_KEEP_ALIVE=true`.
- The heartbeat only renews the **session** token via the master token; it cannot
  outlive the master token, so idle longer than the master validity still requires
  a reconnect.

### Symptom: keep-alive is on but a query still times out, or the warehouse suspends

`CLIENT_SESSION_KEEP_ALIVE` renews the **session token** only. It does not extend
individual query limits and does not keep a warehouse running, so it is the wrong
lever for either of these:

- **A single statement is cancelled for running too long.**
  `STATEMENT_TIMEOUT_IN_SECONDS` bounds how long one statement may run (set per
  account, warehouse, or session); the statement is aborted at that limit
  regardless of keep-alive, and it surfaces as a **query** error, not an auth
  error. Raise the timeout or make the query cheaper — see
  [query-execution.md](query-execution.md).
- **The warehouse auto-suspended while idle.** Warehouse auto-suspend is
  independent of the client session: the session stays valid, and the next query
  simply resumes the warehouse (auto-resume) and pays the resume latency.

Keep-alive prevents *session-token* expiry on an idle connection — nothing more.

---

## On-disk token cache

`client_store_temporary_credential` (default **off**) opts into caching the
issued credential on disk so browser-SSO / MFA flows don't re-prompt on every
process start. Leave it off unless you need the convenience; when on, the cache
file holds a live credential and should be protected like any secret. It is a
per-user convenience, not a way to share credentials between users.

---

## Quick reference

| Setting | For | Notes |
|---|---|---|
| `account` | all | Determines the endpoint. Underscore→hyphen for SSO. |
| `user` / `login_name` | most flows | Optional for PAT / `oauth`. |
| `password` | `snowflake`, `username_password_mfa` | Raw string, not URL-encoded. |
| `authenticator` | all | Selects the flow (see table); case-insensitive. |
| `token` | `oauth`, `programmatic_access_token`, WIF `OIDC` | Access/PAT/OIDC token. |
| `private_key` / `private_key_file` | `snowflake_jwt` | Inline (base64/PEM) or file path. |
| `private_key_password` | `snowflake_jwt` | Decrypts an encrypted PEM. |
| `passcode` / `passcodeInPassword` | `username_password_mfa` | TOTP, or fold it into the password. |
| `oauth_client_id` / `oauth_client_secret` | OAuth code / client-credentials | IdP client credentials. |
| `oauth_authorization_url` / `oauth_token_request_url` | OAuth code / client-credentials | IdP endpoints. |
| `okta_username` / `authentication_timeout` / `disable_saml_url_check` | native Okta | SSO tuning. |
| `workload_identity_provider` | `workload_identity` | `AWS` / `AZURE` / `GCP` / `OIDC`. |
| `client_store_temporary_credential` | SSO / MFA | Opt-in on-disk credential cache. |
| `CLIENT_SESSION_KEEP_ALIVE` | long-idle sessions | Background heartbeat renews the session token; default off. |
