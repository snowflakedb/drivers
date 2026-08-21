# Proxy interception

Proxy configuration and interception (C1–C7), plus which driver HTTP clients
actually honor the proxy.

Related: [tls-handshake.md](tls-handshake.md) · [cert-chain.md](cert-chain.md) · [crl-revocation.md](crl-revocation.md) · [crl-tls-settings.md](crl-tls-settings.md) · up to [CRL/TLS index](../crl-tls.md) · [Runbook](../../../troubleshooting-runbook.md)

**Entry: the connection works on a direct network but fails through a corporate
proxy; or `HTTP_PROXY`-style env vars are set but not picked up.**

---

## Which clients honor the proxy

The driver makes outbound requests from several HTTP clients, and they do **not**
all treat the proxy the same way. This matters when a proxy is mandatory on the
network:

| Client | Honors explicit `proxy_host`? | Env `HTTP(S)_PROXY`? |
|---|---|---|
| **Main REST** (login, queries) | yes | only when `use_proxy_env=true` |
| **Cloud-storage transfers** (PUT/GET to S3/Azure/GCS) | yes — uses the connection's proxy config and its env policy | follows the same `use_proxy_env` policy |
| **CRL fetch** (`sf_core/src/crl/cache.rs`) | **no** — built with timeouts only | **yes, always auto-detected** |
| **Cloud metadata / IMDS** (workload-identity attestation) | **no** | **yes, always auto-detected** |

The two consequences that surprise people:

1. An explicit `proxy_host`/`proxy_port` is **not** applied to CRL fetches or to
   the cloud-metadata calls used by workload-identity auth. If those endpoints
   are only reachable via the proxy, the explicit-proxy config alone won't route
   them.
2. `HTTP_PROXY`/`HTTPS_PROXY` env vars are honored by the CRL-fetch and
   IMDS clients **even when `use_proxy_env=false`**, because those clients never
   suppress env detection. `use_proxy_env=false` only suppresses env proxy on the
   main REST and cloud-storage clients.

So the reliable way to route **every** path (including CRL and IMDS) through one
proxy is the `HTTP_PROXY`/`HTTPS_PROXY` env vars; use `proxy_host` when you only
need the Snowflake REST + transfer traffic proxied. See
[crl-revocation.md](crl-revocation.md) for the CRL-specific firewall test.

---

### C1. Default: `use_proxy_env=false`

`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY` (and lowercase) are **ignored by the main
REST and cloud-storage clients by default** (`sf_core/src/tls/config.rs`). They
apply to those clients only when `use_proxy_env=true` (or ODBC `PROXYWITHENV=true`).
(The CRL/IMDS clients are the exception noted above.)

### C2. Two input forms

Merged by `ProxyConfig::from_settings` (`sf_core/src/tls/config.rs`):

| Setting | Description |
|---|---|
| `proxy_host` | Hostname only — **no** scheme prefix |
| `proxy_port` | Integer; 0 or negative → omitted |
| `proxy_user` | Proxy auth username |
| `proxy_password` | Proxy auth password (redacted in logs) |
| `no_proxy` | Comma-separated bypass patterns |

**Legacy ODBC URL form:** `PROXY = [scheme://][user:pass@]host[:port]`.
Individual fields override URL components. ODBC aliases: `NOPROXY`→`no_proxy`,
`PROXYWITHENV`→`use_proxy_env`, `ALLOWEMPTYPROXY`→`allow_empty_proxy`.

### C3. Precedence

1. Explicit `proxy_host` → applied for all schemes on the main/transfer clients; their env vars disabled.
2. ODBC `PROXY=` URL → parsed; individual fields override.
3. `use_proxy_env=true` + no `proxy_host` → env vars used (main/transfer clients).
4. `use_proxy_env=false` (default) → env proxy suppressed on the main/transfer clients.
5. Empty `PROXY=` + `allow_empty_proxy=true` → proxy explicitly disabled even if `use_proxy_env=true`.

### C4. TLS interception by a corporate MITM proxy

SSL-inspection proxies terminate TLS and re-sign with their own CA. The driver's
trust store doesn't contain that CA → chain validation fails ("unknown issuer" /
"no anchored chains").

**Diagnosis:**
```sh
openssl s_client -connect <host>:443 -proxy <proxy_host>:<proxy_port>   # what cert is presented?
tls_client <URL> -vv --result-file r.json
```

**Resolution:**
1. Export the proxy CA as PEM.
2. Point `custom_root_store_path` at a bundle that includes it (remember: it
   **replaces** the system roots — include the public CAs too; see
   [cert-chain.md](cert-chain.md#b2-custom-trust-store-empty-or-malformed)).
3. If the proxy CA rotates often, consider `crl_check_mode=ADVISORY` so a
   not-yet-cached CRL doesn't hard-fail.

### C5. Credential special characters

Proxy credentials are percent-encoded when the proxy URL is built
(`sf_core/src/tls/client.rs`), so `:`, `@`, `/` in a password are safe. Do
**not** double-encode a pre-formed `PROXY=` URL.

### C6. `ProxyBuild` error

Returned when the constructed proxy URL can't be parsed — usually a `proxy_host`
that wrongly includes a scheme prefix (use `example.com`, not
`http://example.com`).

### C7. `NO_PROXY` not honored

`no_proxy` is active when an explicit `proxy_host` is set. When relying on env
vars (`use_proxy_env=true`, no `proxy_host`), the HTTP stack handles `NO_PROXY`
natively.

---

> **mTLS / client certificates are not supported.** All three production TLS
> client builders finish with `.with_no_client_auth()` (`sf_core/src/tls/client.rs`);
> there is no `client_cert`/`client_key` setting. If a "certificate required"
> error comes from an intermediary, terminate that client-cert requirement at the
> network boundary. Key-pair auth (`authenticator=snowflake_jwt`) is
> application-layer Snowflake authentication — unrelated to TLS client
> certificates, and fully supported ([authentication.md](../authentication.md)).
