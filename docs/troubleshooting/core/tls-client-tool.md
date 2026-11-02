# `tls_client` — TLS/CRL diagnostic tool

`sf_core/src/bin/tls_client.rs` is a minimal standalone HTTP client that uses the
**exact same TLS stack as the driver** (rustls + aws-lc-rs, the same `TlsConfig`
and `CrlConfig` structs). Because every language wrapper (Python, JDBC, Node.js,
ODBC, .NET) sits on top of this one core, `tls_client` reproduces a wrapper's TLS
behavior without needing that wrapper installed.

Use it to isolate whether a TLS failure is driver-specific or system-wide,
reproduce CRL scenarios, and test custom root stores. It is referenced throughout
the [CRL/TLS pages](crl-tls.md).

---

## Build

```sh
# Debug build (faster to compile; fine for one-off diagnosis)
cargo build --package sf_core --features cli --bin tls_client
./target/debug/tls_client <URL>

# Release build (faster to run; use for timing measurements)
cargo build --release --package sf_core --features cli --bin tls_client
./target/release/tls_client <URL>
```

Run from the workspace root (the directory with the top-level `Cargo.toml`). The
binary requires the `cli` feature; without `--features cli` cargo does not build it.

---

## Flag reference

| Flag | Short | Default | Description |
|---|---|---|---|
| `<URL>` | | *required* | Target URL (positional) |
| `--cert-store <FILE>` | `-s` | default roots | PEM bundle to use as trust anchor **instead of** the default roots |
| `--no-verify-certs` | | false | Disable certificate-chain validation (INSECURE; unavailable in FIPS builds) |
| `--no-verify-hostname` | | false | Disable the hostname check only (INSECURE) |
| `--insecure` | `-k` | false | Disable all TLS verification (sets both flags above) |
| `--crl-mode <MODE>` | | `disabled` | CRL revocation check: `disabled` / `enabled` / `advisory` |
| `--no-crl-cache` | | false | Disable both disk and memory CRL cache (forces a fresh fetch) |
| `--allow-certs-without-crl-url` | | false | Allow certs with no CRL distribution-point extension |
| `--http-timeout <SECONDS>` | | 30 | HTTP response timeout |
| `--connect-timeout <SECONDS>` | | 10 | TCP connect timeout |
| `--method <METHOD>` | | `GET` | `GET` / `POST` / `HEAD` / `OPTIONS` |
| `--header <Name: Value>` | `-H` | | Add a request header (repeatable) |
| `--body <DATA>` | `-d` | | Request body string |
| `--output <FILE>` | `-o` | stdout | Write the response body to a file |
| `--result-file <FILE>` | | | Write a machine-readable JSON result (below) |
| `--verbose` | `-v` | INFO | `-v` → DEBUG, `-vv` → TRACE |

**Timeout note:** the total request deadline is `http-timeout + connect-timeout`.
The two values are **summed** into a single reqwest timeout, not applied as
independent limits. The driver's own CRL download timeout (`crl_http_timeout`)
defaults to 10 s, not 30.

**Default roots:** without `--cert-store`, the trust set is the OS root store plus
the bundled Mozilla roots; with `--crl-mode enabled` or `advisory`, the OS root
store only — the same rules as the driver.

**Proxy:** `tls_client` has no proxy flags and picks up `HTTP_PROXY` /
`HTTPS_PROXY` / `NO_PROXY` from the environment automatically. The driver uses
env proxies only with `use_proxy_env=true`, so unset them to reproduce a driver
that connects directly.

**Headers in the log:** from `-v` up, `tls_client` logs every `-H` request header
and every response header with its value. Before sharing `-v` / `-vv` output,
remove any `Authorization` header (and `Set-Cookie` / `Cookie` values), or leave
those headers off the command line.

---

## Common invocations

```sh
# Basic connectivity check (INFO log)
tls_client https://<account>.<region>.snowflakecomputing.com

# Full TLS + proxy debug trace
tls_client https://<account>.<region>.snowflakecomputing.com -vv

# Baseline: disable all verification (rules out cert issues)
tls_client https://<account>.<region>.snowflakecomputing.com -k

# Test with a custom CA bundle (remember: it REPLACES the system roots)
tls_client https://<account>.<region>.snowflakecomputing.com -s /path/to/ca-bundle.pem

# Reproduce with CRL enabled
tls_client https://<account>.<region>.snowflakecomputing.com --crl-mode enabled

# CRL + no cache (forces a fresh CRL fetch every time)
tls_client https://<account>.<region>.snowflakecomputing.com --crl-mode enabled --no-crl-cache

# Machine-readable result for scripting / CI
tls_client https://<account>.<region>.snowflakecomputing.com --result-file result.json
echo $?   # 0 = success (2xx), 1 = any failure
```

---

## Result JSON schema

Written to `--result-file` when specified. The process also exits non-zero on any
failure, so shell scripts can rely on `$?` without parsing JSON. Absent fields are
omitted, not written as `null`.

```json
{
  "success": true,
  "status_code": 200
}
```

| Field | Type | Present when |
|---|---|---|
| `success` | bool | always |
| `status_code` | int | an HTTP response was received (including 4xx/5xx) |
| `error` | string | `success=false` |
| `error_type` | string | `success=false` |

`error_type` values and their triggers:

| Value | Trigger |
|---|---|
| `timeout` | reqwest `.is_timeout()` is true |
| `certificate` | error string contains `certificate`, `tls`, `ssl`, `handshake`, `verify`, `crl`, or `revok` |
| `http` | an HTTP response was received but status is not 2xx |
| `network` | any other error |

`success=false` with `error_type="http"` means TLS **succeeded** and a response
was received — the failure is at the application layer (check `status_code`).

---

Related: [CRL/TLS index](crl-tls.md) ·
[tls-handshake.md](crl-tls/tls-handshake.md) ·
[cert-chain.md](crl-tls/cert-chain.md) ·
[crl-revocation.md](crl-tls/crl-revocation.md) · up to the
[deep-dive index](../index.md) · [Runbook](../../troubleshooting-runbook.md)
