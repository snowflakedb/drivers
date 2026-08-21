# Troubleshooting deep-dive index

Subsystem deep-dives for the Snowflake Universal Core Architecture (the Rust
core `sf_core` and its language wrappers). These pages sit **below** the
[Troubleshooting Runbook](../troubleshooting-runbook.md) — start there for the
task-oriented flow (capture logs → run the connection diagnosis → read the
symptom → action table) and follow its links down into the pages here when you
need subsystem detail.

If you don't yet know which subsystem owns a symptom, the router's
[Appendix A — Symptom → action](../troubleshooting-runbook.md#appendix-a--symptom--action)
is the coarse triage table; [architecture.md](../architecture.md) maps the
component graph.

**Looking up a specific error string or code?** Use the
[keyword → page index](keyword-index.md) — an agent-first table mapping exact
error tokens (`390114`, `612`, "no anchored chains", `UnsatisfiedLinkError`, …) to
the owning page.

## Using these pages (human or agent)

1. Enter from the router's symptom table or from a cross-link, not by guessing a filename.
2. Inside a page, match the **Symptom** heading, then follow **Diagnosis** steps in order — they run cheapest-first.
3. Source references are **file paths only** (no line numbers — they rot on every commit). Confirm the symbol still exists before advising a code change.
4. A wrapper-specific symptom (JNI load, Python import, ODBC DSN) starts in the wrapper page; if the root cause is in `sf_core`, the page cross-links to the core section.

## Core

| Page | Covers |
|---|---|
| [core/authentication.md](core/authentication.md) | Login flows and authenticator values, key-pair/JWT, PAT, OAuth family, external-browser/Okta SSO, workload identity; session-token expiry & refresh; token cache on disk. |
| [core/crl-tls.md](core/crl-tls.md) | Index into the TLS/CRL cluster below. |
| [core/crl-tls/tls-handshake.md](core/crl-tls/tls-handshake.md) | TLS handshake failures; cipher-suite and protocol-version window. |
| [core/crl-tls/cert-chain.md](core/crl-tls/cert-chain.md) | Certificate-chain errors, hostname, expiry, custom trust store. |
| [core/crl-tls/proxy-tls.md](core/crl-tls/proxy-tls.md) | Corporate-proxy interception; which clients honor the proxy and which don't. |
| [core/crl-tls/crl-revocation.md](core/crl-tls/crl-revocation.md) | CRL revocation modes, fetch errors, disk cache, slow first connection. |
| [core/crl-tls/crl-tls-settings.md](core/crl-tls/crl-tls-settings.md) | Full TLS / CRL / proxy settings reference. |
| [core/query-execution.md](core/query-execution.md) | Sync/async query paths, request/query IDs, retries and 503s, result fetch. |
| [core/stage-cloud-storage.md](core/stage-cloud-storage.md) | PUT/GET transfers, cloud-storage allowlist and firewall prerequisites, proxied transfers, downloaded-file permissions. |
| [core/privatelink.md](core/privatelink.md) | PrivateLink scopes, private DNS, cert differences, and the CRL-over-port-80 gap. |
| [core/tls-client-tool.md](core/tls-client-tool.md) | The standalone `tls_client` diagnostic binary. |
| [core/logging-diagnostics.md](core/logging-diagnostics.md) | Troubleshooting-mode capture and per-wrapper native logging (defers to [docs/logging](../logging/logging-architecture.md)). |

## Wrappers

| Page | Covers |
|---|---|
| [wrappers/python.md](wrappers/python.md) | PyO3 native module import, per-CPython build/ABI, trust store, timeouts, Arrow → pandas/numpy. |
| [wrappers/jdbc.md](wrappers/jdbc.md) | JNI bridge load, logger selection (JUL default / SLF4J), `TRACING` file logging. |
| [wrappers/odbc.md](wrappers/odbc.md) | `sf.odbc.ini`, DSN keys, PROXY aliases, Driver Manager tracing. |
| [wrappers/nodejs.md](wrappers/nodejs.md) | **Private preview — stub.** Wrapper is under heavy change; wrapper-specific troubleshooting is deferred. Use the core pages and Runbook. |
| [wrappers/dotnet.md](wrappers/dotnet.md) | **Private preview — stub.** Wrapper is under heavy change; wrapper-specific troubleshooting is deferred. Use the core pages and Runbook. |
