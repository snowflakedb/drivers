# TLS handshake failures & cipher-suite verification

TLS client construction, handshake failures (A1–A5), and cipher-suite
verification (Section E).

Related: [cert-chain.md](cert-chain.md) · [proxy-tls.md](proxy-tls.md) · [crl-revocation.md](crl-revocation.md) · [crl-tls-settings.md](crl-tls-settings.md) · up to [CRL/TLS index](../crl-tls.md) · [Runbook](../../../troubleshooting-runbook.md)

---

## How the TLS client is built

Client construction lives in `sf_core/src/tls/client.rs`; the CRL-aware verifier
in `sf_core/src/tls/crl_verifier.rs`:

```
create_tls_client_with_proxy()          tls/client.rs
  ├─ verify_certificates=false ──────── accepts invalid certs AND invalid hostnames (see A4)
  ├─ crl_check_mode=DISABLED ────────── default TLS path
  │    optional: custom_root_store_path (replaces system roots)
  │    optional: verify_hostname=false
  └─ crl_check_mode=ENABLED|ADVISORY ── CRL-aware verifier
       WebPkiServerVerifier (chain + hostname) + CRL revocation
       rustls ClientConfig … .with_no_client_auth()
```

Crypto provider: `rustls` with `aws-lc-rs`, installed once at startup. All three
production client builders finish with `.with_no_client_auth()` — the driver
never presents a client certificate (see the mTLS note in
[proxy-tls.md](proxy-tls.md)).

---

## TLS handshake failures

**Entry: connection fails before any HTTP response; the error mentions "tls",
"ssl", "handshake", "certificate", or "verify".** The `tls_client` binary
classifies these as the `"certificate"` error type
(`sf_core/src/bin/tls_client.rs`).

### A1. Crypto provider unavailable

The driver installs `aws-lc-rs` as the default rustls provider at startup. A
failure here is a packaging/linking defect, not a configuration issue — report
it as a driver bug.

### A2. TLS protocol-version window

The driver negotiates **TLS 1.2 and 1.3**. The window is configurable per
connection with `min_tls_version` / `max_tls_version` (accepted values `tls12`,
`tls13`); `max_tls_version` must be ≥ `min_tls_version` or the connection is
rejected up front. Anything below TLS 1.2 is never offered — Snowflake requires
TLS 1.2 minimum, which is correct server behavior.

**Diagnosis:**
```sh
openssl s_client -connect <account>.snowflakecomputing.com:443 -tls1_2 </dev/null
# Should succeed. If it fails, the client OS TLS stack is too old or a policy disabled TLS 1.2.
```

**Resolution:** If the failure is a version error, fix the client OS TLS stack
or the policy that disabled TLS 1.2 — don't narrow `max_tls_version`. Only touch
`min_tls_version`/`max_tls_version` when a network policy specifically requires
pinning the window (parsing/validation in `sf_core/src/tls/config.rs`).

### A3. Signature algorithm not supported

rustls/aws-lc-rs accepts a fixed set of signature schemes
(`sf_core/src/tls/crl_verifier.rs`). SHA-1 signatures and P-521 curves are **not**
in the set and cause a handshake failure.

#### Step 1 — capture the negotiated cipher and signature algorithm

```sh
echo Q | openssl s_client -connect <account>.<region>.snowflakecomputing.com:443 2>&1 \
  | grep -E "Protocol|Cipher is|Peer signing digest|Peer signature type"
```

Healthy Snowflake endpoint (TLS 1.2):
```
New, TLSv1.2, Cipher is ECDHE-RSA-AES256-GCM-SHA384
    Protocol  : TLSv1.2
Peer signing digest: SHA256
Peer signature type: rsa_pss_rsae_sha256
```

`rsa_pss_rsae_sha256` / `SHA256` are in rustls's supported set. If you instead
see `sha1WithRSAEncryption` or `ecdsa-with-SHA1`, that is the A3 failure.

#### Step 2 — check the signature algorithm against rustls's supported set

| ID | Name | In rustls/aws-lc-rs? |
|--------|------------------------------|----------------------|
| 0x0401 | rsa_pkcs1_sha256 | yes |
| 0x0804 | rsa_pss_rsae_sha256 | yes ← typical |
| 0x0805 | rsa_pss_rsae_sha384 | yes |
| 0x0403 | ecdsa_secp256r1_sha256 | yes |
| 0x0503 | ecdsa_secp384r1_sha384 | yes |
| 0x0203 | ecdsa_sha1 | **no** → A3 |
| 0x0201 | rsa_pkcs1_sha1 | **no** → A3 |

**Resolution:** Not driver-configurable. If the server (or an intermediary)
mandates an algorithm in the "no" row, file a driver defect. A proxy that
re-terminates TLS with a supported algorithm can unblock it temporarily.

### A4. `verify_certificates=false` also disables hostname verification

When `verify_certificates=false`, the client accepts **both** invalid
certificates **and** invalid hostnames, regardless of `verify_hostname`
(`sf_core/src/tls/client.rs`). Do not assume hostname checking survives when cert
verification is off. This is a debugging-only escape hatch — never use it in production code.

### A5. Diagnostic tool — `tls_client`

```sh
tls_client <URL> --verbose            # connectivity + TLS trace
tls_client <URL> -vv                  # + proxy config, root-store loading
tls_client <URL> --insecure           # baseline: disable all verification
tls_client <URL> --crl-mode enabled   # reproduce with CRL enforcement
tls_client <URL> --result-file r.json # machine-readable result
```

Build instructions and the full flag reference: [tls-client-tool.md](../tls-client-tool.md).

---

## Cipher-suite verification

**Entry: cert chain and proxy are confirmed clean, but TLS still fails — suspect
a cipher-suite mismatch or a weak cipher being offered/rejected.** Rule out proxy
MITM (server cert is genuine, not a proxy re-sign) and trust-store issues first.

### E1. List cipher suites the server offers

```sh
sslscan <account>.snowflakecomputing.com
```

### E2. Rate a cipher via ciphersuite.info

Take the **negotiated** cipher from E3 and look it up on
[ciphersuite.info](https://ciphersuite.info):

| Badge | Meaning |
|---|---|
| `Recommended` | Strong, modern — not the problem |
| `Secure` | Acceptable for TLS 1.2+ |
| `Weak` | Deprecated/marginal; may be rejected by strict policy |
| `Insecure` | Known-broken; rejected by rustls / aws-lc-rs |

**Practical rule:** Snowflake servers always offer strong cipher suites. A
`Weak`/`Insecure` cipher in the negotiated handshake almost always means a
client-side intermediary (proxy, VPN terminator, SSL inspector) is constraining
the cipher list. Investigate the client network path, not the server.

### E3. Observe the negotiated cipher during the real handshake

```sh
openssl s_client -connect <account>.snowflakecomputing.com:443 -tls1_2 </dev/null 2>/dev/null \
  | grep -E "^(New|Cipher|Protocol|Peer|Server)"
```

**If `openssl s_client` succeeds but the driver fails**, the negotiated cipher is
likely outside rustls/aws-lc-rs's supported set. The driver supports:
- **TLS 1.3:** `TLS_AES_256_GCM_SHA384`, `TLS_AES_128_GCM_SHA256`, `TLS_CHACHA20_POLY1305_SHA256`
- **TLS 1.2:** ECDHE suites with AES-GCM or ChaCha20; **no RSA key exchange** (no forward secrecy)

**If the negotiation uses a TLS 1.2 RSA key-exchange cipher** (no `ECDHE`
prefix) — typically a proxy/load balancer presenting a downgraded cipher list —
rustls refuses the handshake. Snowflake endpoints always support ECDHE; if it is
absent, a network intermediary is constraining the cipher list on the client
side.

### E4. Capture the handshake at the wire (advanced)

When E1–E3 are inconclusive — or you suspect an intermediary is silently
rewriting the offered cipher list — capture the raw handshake instead of trusting
`openssl s_client`'s summary. The ClientHello and ServerHello travel in the clear
even on TLS 1.3, so the offered suites, the selected suite, the negotiated
version, and the SNI are all visible. (The certificate is encrypted under TLS 1.3,
so inspect certs with `openssl s_client` or the tools in
[cert-chain.md](cert-chain.md), not the capture.)

```sh
# 1. Capture only the Snowflake handshake to a pcap, then reproduce the failure.
sudo tcpdump -i any -s0 -w handshake.pcap \
  "host <account>.<region>.snowflakecomputing.com and port 443"
# …run the failing connection in another shell, then Ctrl-C tcpdump.

# 2. Decode the two hello messages.
tshark -r handshake.pcap \
  -Y 'tls.handshake.type == 1 || tls.handshake.type == 2' -V \
  | grep -E "Handshake Type|Cipher Suite|Server Name|Version"
```

- **ClientHello** (`tls.handshake.type == 1`) lists every suite the client
  *offered*. If ECDHE suites are missing here, something on the client replaced the
  driver's list before it left the host.
- **ServerHello** (`tls.handshake.type == 2`) shows the single suite the server
  *selected*, plus the negotiated version.

To turn the hex cipher IDs a capture shows into names — and to confirm what your
local OpenSSL can offer — enumerate them locally:

```sh
openssl ciphers -V 'ECDHE' | head   # e.g. 0xC0,0x2F  ->  ECDHE-RSA-AES128-GCM-SHA256
```

`openssl ciphers -V` reflects the **local OpenSSL** build, not the driver's set —
the driver uses rustls/aws-lc-rs, whose supported suites are listed in **E3**; use
it only to decode capture output and as a client-capability sanity check. Feed the
*negotiated* suite back into **E2** (ciphersuite.info): a `Weak`/`Insecure` result
seen on the wire is the signature of a client-side intermediary constraining the
handshake.
