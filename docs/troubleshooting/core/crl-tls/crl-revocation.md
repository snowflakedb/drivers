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
| `Advisory` | `2` / `ADVISORY` | Only confirmed revocation fails; other CRL errors are logged and allowed. |

An unrecognized value logs a warning and falls back to `DISABLED`.

> **Confirmed revocation always fails** — an end-entity or chain revocation is
> fatal in **every** mode, including `ADVISORY`. There is no setting to accept a
> revoked certificate.

### Validation flow

```
verify_server_cert()                       crl_verifier.rs
  1. WebPki verify (chain + hostname)
  2. build all anchored candidate chains   x509_utils.rs
  3. per chain, CRL worker validates → NotRevoked (first clean chain wins)
                                        Revoked      (always fail)
                                        NotDetermined(fail in Enabled; allow in Advisory)
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
- Stale on-disk CRLs are cleaned up after a removal delay (default **7 days**),
  independent of the in-memory refresh.

> This is a *different* knob from the config-file permission check
> (`unsafe_skip_file_permissions_check`, which guards `connections.toml` /
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
`crl_http_timeout` (default 30 s) then `crl_connection_timeout` (default 10 s). In
`ADVISORY` the connection then proceeds — so the symptom is slowness, not an
error.

**Diagnosis:**
```sh
time tls_client https://<account>.snowflakecomputing.com --crl-mode advisory --no-crl-cache
time tls_client https://<account>.snowflakecomputing.com --crl-mode disabled
# If advisory is >10 s slower, a CRL fetch is timing out. Find the CDP:
tls_client https://<account>.snowflakecomputing.com --crl-mode advisory --no-crl-cache -vv 2>&1 \
  | grep -i "crl\|distribution\|timeout\|failed"
```

**Resolution:** open the CDP in the firewall (CRL-3); or fail fast with
`crl_http_timeout=5` / `crl_connection_timeout=3`; or set
`crl_check_mode=DISABLED` if revocation checking isn't required.

### CRL-1. Revocation vs fetch — classify first

```sh
# Advisory tolerates fetch errors; only confirmed revocation fails:
tls_client https://<account>.snowflakecomputing.com --crl-mode advisory --result-file advisory.json
# Enabled with a fresh fetch (bypass cache):
tls_client https://<account>.snowflakecomputing.com --crl-mode enabled --no-crl-cache -vv --result-file enabled.json
```

| advisory | enabled | Diagnosis |
|---|---|---|
| `success:true` | fails, error contains "revoked" | Confirmed revocation → CRL-2 |
| `success:true` | fails, `error_type:"timeout"`/"network" | Fetch failing → CRL-3 |
| `success:false` | `success:false` | Not a CRL issue → [cert-chain.md](cert-chain.md) or [tls-handshake.md](tls-handshake.md) |

`-vv` emits DEBUG logs (target `sf_core::crl`) with each CDP URL and the per-cert
outcome — the primary tool for pinpointing which cert's CRL failed.

### CRL-2. Certificate revoked — which cert?

Errors: `"End-entity certificate is revoked"`, `"Certificate chain is revoked or
indeterminate"`. Both fail in **all** modes.

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

Errors: `"Failed to download CRL from URL: <url>"`, `"HTTP timeout while fetching
CRL"`. Hard failures in `ENABLED`; logged-but-tolerated in `ADVISORY` (unless the
cert is also revoked).

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

**Timeouts** (`sf_core/src/crl/config.rs`): `crl_http_timeout` (30 s),
`crl_connection_timeout` (10 s).

**Resolution, in order:** (1) fix the network path; (2) `crl_check_mode=ADVISORY`
so fetch failures are non-fatal; (3) for a private PKI whose certs have no
reachable CDP, `crl_allow_certificates_without_crl_url=true` skips revocation for
certs lacking a usable CDP extension.

### CRL-4. Stale/expired cached CRL

Error: `"CRL has expired"`. The driver retries the fetch once on an expired
cached CRL (`sf_core/src/crl/validator.rs`); if the retry also fails, `ENABLED`
hard-fails and `ADVISORY` allows.

```sh
tls_client https://<account>.snowflakecomputing.com --crl-mode enabled --no-crl-cache -vv
```
If `--no-crl-cache` succeeds, the on-disk cache held an expired CRL the
background refresh hadn't yet replaced. `--no-crl-cache` is diagnostic only;
ensure the CDP is consistently reachable (CRL-3).
