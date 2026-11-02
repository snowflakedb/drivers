# CRL & certificate revocation

CRL modes, validation flow, cache behavior, and CRL-specific troubleshooting
(CRL-0 through CRL-4): slow first connection, revocation-vs-fetch classification,
identifying a revoked cert, fetch errors, and stale cache.

Related: [tls-handshake.md](tls-handshake.md) · [cert-chain.md](cert-chain.md) · [proxy-tls.md](proxy-tls.md) · [crl-tls-settings.md](crl-tls-settings.md) · up to [CRL/TLS index](../crl-tls.md) · [Runbook](../../../troubleshooting-runbook.md)

---

## CRL modes

`crl_check_mode` (`sf_core/src/crl/config.rs`) — **default `DISABLED`**:

| Mode | Value | Behavior |
|---|---|---|
| `Disabled` (default) | `0` / `DISABLED` | No CRL fetch; the handshake still validates chain + hostname. |
| `Enabled` | `1` / `ENABLED` | Any CRL issue (fetch failure, parse error, revocation) fails the connection. |
| `Advisory` | `2` / `ADVISORY` | Same as `ENABLED` for a failed CRL fetch, an unparseable or expired CRL, and a certificate with no usable CRL URL: the chain is treated as revoked and the connection fails. |

Values are matched case-sensitively (`DISABLED`, `ENABLED`, `ADVISORY`, `0`, `1`,
`2`); any other value, including lowercase, is rejected when the connection is
created. ODBC keys: `CRL_MODE`, `CRL_ENABLED`.

> **Confirmed revocation always fails** — an end-entity or chain revocation is
> fatal in **every** mode, including `ADVISORY`. There is no setting to accept a
> revoked certificate.

> **A CRL problem surfaces as "chain revoked".** For any certificate in the chain,
> a CRL that cannot be fetched, parsed, or verified counts as a failed chain, and the error reads `CRL validation failed: chain revoked`
> even though nothing was revoked. A certificate without a CRL distribution point
> fails the same way unless `crl_allow_certificates_without_crl_url=true`. See
> [CRL-1](#crl-1-revocation-vs-fetch--classify-first).

### Validation flow

```
verify_server_cert()                       crl_verifier.rs
  1. WebPki verify (chain + hostname)
  2. build all anchored candidate chains   x509_utils.rs
  3. per chain, CRL worker validates → NotRevoked (first clean chain wins)
                                        Revoked      (always fail)
                                        fetch/parse error or no CRL URL
                                                     (reported as "chain revoked";
                                                      allowed for no-CRL-URL only
                                                      with crl_allow_certificates_without_crl_url)
```

### Cache & short-lived certs

- Memory cache + optional disk cache (`sf_core/src/crl/cache.rs`).
- Background refresh at **half** the CRL's validity period.
- Default disk cache dir: `{platform_cache_dir}/snowflake/crls/` (override with
  `crl_cache_dir`).
- **Short-lived certificates skip the CRL check entirely.** The threshold
  follows the CA/Browser-Forum short-lived definition: certificates whose total
  validity is **≤ 7 days** (it was ≤ 10 days before 2026-03-15). Short-lived
  Snowflake leaf certs therefore never trigger a CRL fetch.

### Disk-cache file hygiene

- On Unix the disk cache is expected to be owner-only. If the cache directory or
  a cache file has looser permissions, the driver **ignores the disk cache and
  logs a warning** — it never hard-fails on cache permissions. Override with
  `crl_unsafe_skip_file_permissions_check=true` (default off).
- Stale on-disk CRLs are removed after a removal delay (`crl_on_disk_cache_removal_delay`,
  default **7 days**), but only when the background cleanup is enabled
  (`crl_cache_start_cleanup=true`, default off; passes run every
  `crl_cache_cleanup_interval`, default 1 hour).
- A cached CRL is re-fetched once it is older than `crl_validity_time` (default
  24 hours), whatever its own `nextUpdate` says.

> This is a *different* knob from the config-file permission check
> (`unsafe_skip_config_file_permissions_check`, which guards `connections.toml` /
> `config.toml` — see the router
> [§4.1](../../../troubleshooting-runbook.md#41-connectionstoml--configtoml)).
> The `crl_`-prefixed one guards only the CRL cache.

---

## Fetch/revocation errors

**Entry: the error contains "revoked", "CRL", "distribution point", "failed to
download CRL", or "HTTP timeout while fetching CRL"; or CRL mode is on and
connections fail only through certain networks.** These fire only when
`crl_check_mode` is `ENABLED` or `ADVISORY`; the default `DISABLED` fetches no CRL.

### CRL-0. Slow first connection (fetch timing out, not erroring)

**Entry: connections succeed but take 10–60 s the first time; subsequent
connections on the warm cache are fast; no error is returned.**

With CRL on, a new TLS connection triggers a fetch when the cached CRL is absent
or expired. If the CRL distribution point (CDP) is blocked, the fetch waits for
`crl_http_timeout` (default 10 s) and `crl_connection_timeout` (default 10 s). The
connection then fails with `CRL validation failed: chain revoked` (see CRL-1), so
the slowness ends in an error unless the CRL is eventually fetched or CRL checking
is disabled.

**Diagnosis:**
```sh
time tls_client https://<account>.snowflakecomputing.com --crl-mode advisory --no-crl-cache
time tls_client https://<account>.snowflakecomputing.com --crl-mode disabled
# If advisory is >10 s slower (or fails), a CRL fetch is timing out. Find the CDP:
tls_client https://<account>.snowflakecomputing.com --crl-mode advisory --no-crl-cache -vv 2>&1 \
  | grep -i "crl\|distribution\|timeout\|failed"
```

**Resolution:** open the CDP in the firewall (CRL-3); or fail fast with
`crl_http_timeout=5` / `crl_connection_timeout=3`; or set
`crl_check_mode=DISABLED` if revocation checking isn't required. ODBC key for
`crl_http_timeout`: `CRL_DOWNLOAD_TIMEOUT`.

### CRL-1. Revocation vs fetch — classify first

```sh
# Disabled: no CRL involved, the baseline:
tls_client https://<account>.snowflakecomputing.com --crl-mode disabled --result-file disabled.json
# Enabled with a fresh fetch (bypass cache):
tls_client https://<account>.snowflakecomputing.com --crl-mode enabled --no-crl-cache -vv --result-file enabled.json
```

| disabled | enabled | Diagnosis |
|---|---|---|
| `success:true` | fails, `-vv` log shows a fetch/parse error for a CDP | Fetch failing → CRL-3 |
| `success:true` | fails, `-vv` log shows a revocation hit in the CRL | Confirmed revocation → CRL-2 |
| `success:true` | `success:true` | CRL path is healthy |
| `success:false` | `success:false` | Not a CRL issue → [cert-chain.md](cert-chain.md) or [tls-handshake.md](tls-handshake.md) |

`ADVISORY` gives the same result as `ENABLED` for fetch failures, so the
`ENABLED` run is sufficient to classify; the error text alone
(`CRL validation failed: chain revoked`) does not distinguish the two cases.

`-vv` emits DEBUG logs (target `sf_core::crl`) with each CDP URL and the per-cert
outcome — the primary tool for pinpointing which cert's CRL failed.

### CRL-2. Certificate revoked — which cert?

Errors: `"CRL validation failed: end-entity certificate revoked"` (a confirmed
revocation of the server certificate) and `"CRL validation failed: chain revoked"`
(an issuer in the chain is revoked, **or** its CRL could not be fetched or
verified — see CRL-3). Both fail in **all** modes.

```sh
# 1. Split the chain into per-cert files:
openssl s_client -connect <account>.snowflakecomputing.com:443 -showcerts </dev/null 2>/dev/null \
  | awk '/-----BEGIN CERTIFICATE-----/,/-----END CERTIFICATE-----/' \
  | csplit --quiet - '/-----BEGIN CERTIFICATE-----/' '{*}'
# 2. Find each cert's CDP:
for f in xx0[1-9]; do echo "=== $f ==="; openssl x509 -in "$f" -noout -text 2>/dev/null \
  | grep -A4 "CRL Distribution Points" || echo "(no CDP)"; done
# 3. Fetch the CRL and look for the suspect serial:
SERIAL=$(openssl x509 -in xx01 -noout -serial | cut -d= -f2)
curl -s <CDP_URL> -o crl.der
openssl crl -inform DER -in crl.der -text -noout | grep -i "$SERIAL"
```

**Resolution:** A revoked cert is server-side — the endpoint or an intermediate
CA has been revoked; not fixable by driver config. Contact Snowflake Support with
the account name and the step-2 output.

### CRL-3. Fetch errors — network / timeout

Errors in the logs (target `sf_core::crl`): `"Failed to download CRL from <url>"`
(URL with the query string stripped), `"HTTP timeout while fetching CRL"`, `"CRL
download from <url> exceeded max size …"` (limit `crl_max_download_size`, default
20 MiB). The TLS error the application sees is `CRL validation failed: chain
revoked`, in `ENABLED` and `ADVISORY` alike.

```sh
# 1. Identify the failing CDP:
tls_client https://<account>.snowflakecomputing.com --crl-mode enabled --no-crl-cache -vv 2>&1 \
  | grep -i "crl\|distribution\|download\|timeout"
# 2. Test it directly:
curl -v --max-time 15 "<CDP_URL>" -o /dev/null
# Expect HTTP 200, Content-Type application/pkix-crl (or x-pkcs7-crl).
```

**Network path.** CDP URLs are embedded in the public CA certificates and point
to **public CA infrastructure over HTTP port 80** — regardless of whether the
Snowflake connection uses PrivateLink (see [privatelink.md](../privatelink.md)).
So **port 80 outbound to the CDP host must be open whenever `crl_check_mode` is
`ENABLED` or `ADVISORY`**; the default `DISABLED` needs no port 80.

**Proxy caveat.** The CRL client does **not** use the connection's explicit
`proxy_host`, but it **does** auto-detect `HTTP_PROXY`/`HTTPS_PROXY` env vars
(see [proxy-tls.md](proxy-tls.md#which-clients-honor-the-proxy)). So test the CDP
the way the driver will actually reach it — via env proxy if one is set:
```sh
HTTPS_PROXY=<proxy_host>:<proxy_port> curl -v --max-time 15 "<CDP_URL>" -o /dev/null
```

**Timeouts** (`sf_core/src/crl/config.rs`): `crl_http_timeout` (10 s),
`crl_connection_timeout` (10 s).

**Resolution, in order:** (1) fix the network path so the CDP is reachable;
(2) for a private PKI whose certs have no CDP extension,
`crl_allow_certificates_without_crl_url=true` skips revocation for certs lacking
one (it does not help when a CDP exists but is unreachable); (3) set
`crl_check_mode=DISABLED` if revocation checking isn't required.

### CRL-4. Stale/expired cached CRL

Error: `"CRL has expired"`. The driver retries the fetch once on an expired
cached CRL (`sf_core/src/crl/validator.rs`); if the retry also fails, the
connection fails with `CRL validation failed: chain revoked` in `ENABLED` and
`ADVISORY`.

```sh
tls_client https://<account>.snowflakecomputing.com --crl-mode enabled --no-crl-cache -vv
```
If `--no-crl-cache` succeeds, the on-disk cache held an expired CRL the
background refresh hadn't yet replaced. `--no-crl-cache` is diagnostic only;
ensure the CDP is consistently reachable (CRL-3).
