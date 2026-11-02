# TLS / CRL / proxy settings reference

Every TLS-, CRL-, and proxy-related connection setting in one place. For **where**
each can be set (connection param, `connections.toml`, `sf.odbc.ini`, env var) and
precedence, see the Runbook
[§4 Configurations supported](../../../troubleshooting-runbook.md#4-configurations-supported).

Sources: `sf_core/src/tls/config.rs`, `sf_core/src/tls/cipher_policy.rs`,
`sf_core/src/crl/config.rs`, `sf_core/src/config/connection_config.rs`,
`sf_params_spec/src/lib.rs` (the parameter registry: names, defaults, aliases).

Related: [tls-handshake.md](tls-handshake.md) · [cert-chain.md](cert-chain.md) · [proxy-tls.md](proxy-tls.md) · [crl-revocation.md](crl-revocation.md) · up to [CRL/TLS index](../crl-tls.md)

---

## TLS

| Setting | Type | Default | Effect |
|---|---|---|---|
| `verify_certificates` | bool | `true` | `false` disables **all** validation (also forces `verify_hostname=false`) — debug only; rejected in FIPS builds |
| `tls_skip_verify` | bool | `false` | `true` disables cert, hostname, and CRL checks in one switch (logs a warning) — debug only; rejected in FIPS builds |
| `verify_hostname` | bool | `true` | `false` disables the hostname check only |
| `custom_root_store_path` | path | — | PEM bundle used **instead of** the system roots (replaces, not adds) |
| `min_tls_version` | string | `tls12` | Lowest protocol version to offer (`tls12` / `tls13`) |
| `max_tls_version` | string | `tls13` | Highest protocol version to offer; must be ≥ `min_tls_version` |

### `SNOWFLAKE_TLS_CIPHERS`

Process environment variable, not a connection parameter. It applies to every
TLS client in the process: login, queries, result chunks, stage transfers,
in-band telemetry, and login-time cloud calls, including clients built with
`verify_certificates=false`. The value is a colon-separated list. A name that
starts with `TLS_` (for example `TLS_AES_256_GCM_SHA384`) selects a TLS 1.3
suite; any other name (for example `ECDHE-RSA-AES256-GCM-SHA384`) selects a
TLS 1.2 suite. Names for one version do not change the suites of the other:
a TLS 1.3-only list leaves the TLS 1.2 suites at their defaults, and a TLS
1.2-only list leaves the TLS 1.3 suites at their defaults. Empty or whitespace
is the same as unset. An unrecognized name, a name this build does not offer,
or an enabled version left with no suite makes the connection fail with an
error.

Limiting a connection to the two AES-GCM TLS 1.3 suites takes both
`min_tls_version=tls13` and
`SNOWFLAKE_TLS_CIPHERS=TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384`.

## CRL

| Setting | Type | Default | Effect |
|---|---|---|---|
| `crl_check_mode` | string | `DISABLED` | `DISABLED` / `ENABLED` / `ADVISORY` (or `0` / `1` / `2`), case-sensitive; other values are rejected. ODBC: `CRL_MODE`, `CRL_ENABLED` |
| `crl_allow_certificates_without_crl_url` | bool | `false` | Allow certs whose CDP extension is missing. ODBC: `CRL_ALLOW_NO_CRL` |
| `crl_http_timeout` | int (s) | `10` | Per-fetch HTTP timeout. ODBC: `CRL_DOWNLOAD_TIMEOUT` |
| `crl_connection_timeout` | int (s) | `10` | Per-fetch TCP connect timeout |
| `crl_max_download_size` | int (bytes) | `20971520` | Abort a CRL download larger than this |
| `crl_enable_disk_caching` | bool | `true` | Cache CRLs to disk. ODBC: `CRL_DISK_CACHING` |
| `crl_enable_memory_caching` | bool | `true` | Cache CRLs in memory. ODBC: `CRL_MEMORY_CACHING` |
| `crl_cache_dir` | path | `{cache}/snowflake/crls` | Override the CRL disk-cache directory |
| `crl_validity_time` | int (s) | `86400` | Maximum age of a cached CRL before re-fetch, regardless of its `nextUpdate` |
| `crl_on_disk_cache_removal_delay` | int (s) | `604800` | How long an expired CRL stays on disk before cleanup removes it |
| `crl_cache_cleanup_interval` | int (s) | `3600` | Interval between background cleanup passes |
| `crl_cache_start_cleanup` | bool | `false` | Enable the background disk-cache cleanup |
| `crl_unsafe_skip_file_permissions_check` | bool | `false` | Skip the CRL cache permission check (insecure perms otherwise → cache ignored + warning, never fatal) |

## Proxy

| Setting | Type | Default | Effect |
|---|---|---|---|
| `proxy_host` | string | — | Explicit proxy hostname (**no** scheme prefix) |
| `proxy_port` | int | — | Proxy port |
| `proxy_scheme` | string | `http` | `http` or `https`; `https` uses TLS for the hop to the proxy |
| `proxy_user` | string | — | Proxy auth username |
| `proxy_password` | string | — | Proxy auth password (redacted in logs) |
| `no_proxy` | string | — | Comma-separated bypass patterns |
| `use_proxy_env` | bool | `false` (Python: `true`) | Honor `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` on the main + transfer clients |
| `allow_empty_proxy` | bool | `true` | An empty `PROXY=` value explicitly disables the proxy |
| `proxy` (ODBC) | URL | — | Legacy ODBC form `[scheme://][user:pass@]host[:port]` |

> **Not every client obeys these proxy settings.** The CRL-fetch, AWS IMDS region,
> and platform-detection clients ignore `proxy_host` and always auto-detect env
> proxy regardless of `use_proxy_env`. See
> [proxy-tls.md](proxy-tls.md#which-clients-honor-the-proxy).
