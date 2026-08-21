# Certificate-chain errors

Chain validation failures (B1–B6): no anchored chains, unknown issuer, custom
trust store, cross-signed intermediates, hostname mismatch, expired certs.

Related: [tls-handshake.md](tls-handshake.md) · [proxy-tls.md](proxy-tls.md) · [crl-revocation.md](crl-revocation.md) · [crl-tls-settings.md](crl-tls-settings.md) · up to [CRL/TLS index](../crl-tls.md) · [Runbook](../../../troubleshooting-runbook.md)

**Entry: the error mentions "no anchored chains", "unknown issuer", "unable to
verify", or a WebPki chain failure.**

---

### B1. "no anchored chains"

The chain builder (`sf_core/src/tls/crl_verifier.rs`, `x509_utils.rs`) could not
build a path from the server (end-entity) certificate to any trusted root.
Common causes:

- Server is not sending its intermediate certificates (server misconfiguration).
- A custom trust store (`custom_root_store_path`) that does not contain the
  signing CA.
- The system root store failed to load (minimal container images without
  `ca-certificates`).

**Diagnosis:**
```sh
# Are all intermediates present in the server chain?
openssl s_client -connect <host>:443 -showcerts 2>/dev/null | grep -E "^(subject|issuer)"

# Verify a custom PEM bundle actually anchors the server cert:
openssl verify -CAfile /path/to/bundle.pem <server-cert.pem>

# Reproduce with the driver's own loader:
tls_client <URL> --cert-store /path/to/bundle.pem -vv
```

### B2. Custom trust store empty or malformed

`custom_root_store_path` **replaces** the OS root store for that connection — it
is not additive. If you point it at a bundle that omits the public CA chain, a
previously-working connection starts failing with B1. The loader
(`sf_core/src/tls/client.rs`) rejects a file that parses to zero certificates, or
a DER it cannot add. Include **only CA certificates** — no end-entity certs, no
private keys. To keep trusting the public web PKI *and* add a private CA, build a
single bundle containing both.

### B3. Cross-signed intermediate chains

The builder produces **all** anchored candidate chains and the first clean one
wins. A chain revoked under one root but clean under another still passes
(authoritative behavior in the `crl_verifier.rs` tests). For intermittent
failures on cross-signed servers, check whether one signing root was recently
revoked or dropped from the trust store.

### B4. Hostname mismatch

The verifier rejects a cert whose SAN/CN doesn't match the server name; bypassed
only when `verify_hostname=false`.

**Diagnosis:** `openssl s_client -connect <host>:443` → check `subject` and
`subjectAltName`. IP-address connections require an **IP SAN**, not just a CN.

### B5. Expired certificate

WebPki checks `notBefore`/`notAfter` against the current time; client-side clock
skew causes false expiry.

**Resolution:** Sync the system clock — `timedatectl status` (Linux),
`w32tm /query /status` (Windows).

### B6. Chain settings

| Setting | Type | Default | Effect |
|---|---|---|---|
| `verify_certificates` | bool | `true` | `false` disables **all** cert + hostname validation (debug only) |
| `verify_hostname` | bool | `true` | `false` disables the hostname check only |
| `custom_root_store_path` | path | — | PEM bundle used **instead of** the system roots |

Source: `sf_core/src/tls/config.rs`, `sf_core/src/config/connection_config.rs`.
Full reference: [crl-tls-settings.md](crl-tls-settings.md).
