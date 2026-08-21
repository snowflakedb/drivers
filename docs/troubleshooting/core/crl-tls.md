# CRL and TLS — index

TLS and certificate-revocation troubleshooting, split into focused sub-pages.
Part of the [troubleshooting deep-dive](../index.md); the
[Troubleshooting Runbook](../../troubleshooting-runbook.md) **TLS / certificate
error** row routes here.

> **Revocation is CRL-only.** The core validates certificate revocation with
> **CRLs**; it performs **no OCSP revocation validation** (a deliberate
> divergence from the older drivers' versions). Don't look for OCSP settings — there are
> none.

| Topic | Page |
|---|---|
| TLS handshake failures; protocol-version window; cipher-suite verification | [crl-tls/tls-handshake.md](crl-tls/tls-handshake.md) |
| Certificate-chain errors (no anchored chains, hostname, expiry, custom trust store) | [crl-tls/cert-chain.md](crl-tls/cert-chain.md) |
| Proxy interception; which driver clients honor the proxy | [crl-tls/proxy-tls.md](crl-tls/proxy-tls.md) |
| CRL revocation modes, fetch errors, disk cache, slow first connection | [crl-tls/crl-revocation.md](crl-tls/crl-revocation.md) |
| Full TLS / CRL / proxy settings reference | [crl-tls/crl-tls-settings.md](crl-tls/crl-tls-settings.md) |

For the standalone `tls_client` diagnostic binary used throughout these pages,
see [tls-client-tool.md](tls-client-tool.md). For PrivateLink-specific TLS/CRL
behavior see [privatelink.md](privatelink.md).
