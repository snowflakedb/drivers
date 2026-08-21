# Keyword → page index (agent router)

Agent-first lookup. Match the **exact error string, error code, or symptom
keyword** you have in the left column, then open the linked page and jump to the
quoted section. This complements — it does not replace — the task-oriented
[Troubleshooting Runbook](../troubleshooting-runbook.md) (start there for the
*capture logs → diagnose → triage* flow) and the [deep-dive index](index.md)
(browse by subsystem).

**Conventions.** Links are **page-level**: in-page anchors and source line numbers
rot on every commit, so each entry gives you the page **plus the heading text to
search for** inside it (search the quoted string). Quoted strings are unique
prefixes of a real heading. All pages are under `docs/troubleshooting/`. If
nothing matches, use the [deep-dive index](index.md) or
[architecture.md](../architecture.md) to find the owning subsystem, then re-route.

---

## Authentication & session

| Match this (error / code / keyword) | Go to |
|---|---|
| `401`, "Incorrect username or password", bad credentials | [authentication.md](core/authentication.md) → "Symptom: 401" |
| key-pair / JWT auth fails, "JWT token is invalid", `JWT_TOKEN_INVALID_EXPIRATION_TIME`, JWT expired in transit / proxy latency | [authentication.md](core/authentication.md) → "Symptom: key-pair (JWT) auth fails" |
| PAT rejected, "Programmatic access token is invalid or expired" | [authentication.md](core/authentication.md) → "Symptom: PAT rejected" |
| OAuth token rejected, `390301` | [authentication.md](core/authentication.md) → "Symptom: OAuth token rejected" |
| MFA passcode not accepted | [authentication.md](core/authentication.md) → "Symptom: MFA passcode not accepted" |
| external-browser / Okta SSO fails; browser opens then login fails; account identifier with an **underscore** + SSO | [authentication.md](core/authentication.md) → "Symptom: external-browser / Okta SSO" |
| "Account not found" / wrong endpoint | [authentication.md](core/authentication.md) → "Account not found" |
| "Session expired", token refresh, `390111` (session gone), `390112`, `390114`, "Authentication token has expired", "session no longer exists", master token expired | [authentication.md](core/authentication.md) → "Session expiry & renewal" |
| heartbeat / keep-alive stopped, idle session dropped | [authentication.md](core/authentication.md) → "Keeping an idle session alive" |
| keep-alive on but a query still times out; `STATEMENT_TIMEOUT_IN_SECONDS`; warehouse auto-suspend / auto-resume while the session stays alive | [authentication.md](core/authentication.md) → "Symptom: keep-alive is on but a query still times out" |
| on-disk token cache, cached-token reuse problems | [authentication.md](core/authentication.md) → "On-disk token cache" |

## TLS handshake & certificates

| Match this | Go to |
|---|---|
| TLS handshake failure; "certificate", "ssl", "verify" | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "TLS handshake failures" |
| crypto provider unavailable | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "A1. Crypto provider unavailable" |
| TLS protocol-version mismatch / version too old | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "A2. TLS protocol-version window" |
| signature algorithm not supported | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "A3. Signature algorithm not supported" |
| `verify_certificates=false` also disables hostname verification | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "A4." |
| cipher-suite mismatch, weak cipher rejected, `sslscan`, ciphersuite.info | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "Cipher-suite verification" |
| capture the TLS handshake on the wire; `tcpdump` / `tshark` / Wireshark; `openssl ciphers -V`; offered vs selected cipher suite | [tls-handshake.md](core/crl-tls/tls-handshake.md) → "E4. Capture the handshake at the wire" |
| "no anchored chains", unknown issuer, cert-chain error | [cert-chain.md](core/crl-tls/cert-chain.md) → `B1. "no anchored chains"` |
| custom trust store empty/malformed (`custom_root_store_path`) | [cert-chain.md](core/crl-tls/cert-chain.md) → "B2. Custom trust store" |
| cross-signed intermediate chain | [cert-chain.md](core/crl-tls/cert-chain.md) → "B3. Cross-signed" |
| hostname mismatch | [cert-chain.md](core/crl-tls/cert-chain.md) → "B4. Hostname mismatch" |
| expired certificate | [cert-chain.md](core/crl-tls/cert-chain.md) → "B5. Expired certificate" |

## CRL / revocation

| Match this | Go to |
|---|---|
| slow first connection (10–60 s), fast on subsequent attempts | [crl-revocation.md](core/crl-tls/crl-revocation.md) → "CRL-0. Slow first connection" |
| certificate revoked | [crl-revocation.md](core/crl-tls/crl-revocation.md) → "CRL-2. Certificate revoked" |
| CRL fetch error, network/timeout fetching a CRL | [crl-revocation.md](core/crl-tls/crl-revocation.md) → "CRL-3. Fetch errors" |
| stale / expired cached CRL | [crl-revocation.md](core/crl-tls/crl-revocation.md) → "CRL-4." |

## Proxy

| Match this | Go to |
|---|---|
| connection fails through a corporate proxy | [proxy-tls.md](core/crl-tls/proxy-tls.md) → "Which clients honor the proxy" |
| `HTTP_PROXY` / `HTTPS_PROXY` env var not being picked up | [proxy-tls.md](core/crl-tls/proxy-tls.md) → "C2. Two input forms" |
| TLS interception by a corporate MITM proxy | [proxy-tls.md](core/crl-tls/proxy-tls.md) → "C4." |
| `NO_PROXY` not honored | [proxy-tls.md](core/crl-tls/proxy-tls.md) → "C7." |
| `ProxyBuild` error | [proxy-tls.md](core/crl-tls/proxy-tls.md) → "C6." |
| full TLS / CRL / proxy settings reference | [crl-tls-settings.md](core/crl-tls/crl-tls-settings.md) |

## Query execution & results

| Match this | Go to |
|---|---|
| query hangs, never returns, async poll timeout | [query-execution.md](core/query-execution.md) → "Symptom: query hangs, no response" |
| error 612, `AsyncPollResultNotFound` | [query-execution.md](core/query-execution.md) → "Symptom: error 612" |
| multi-statement returns wrong / missing child-result count | [query-execution.md](core/query-execution.md) → "Symptom: multi-statement returns the wrong number" |
| intermittent 503s; retries / backoff behavior | [query-execution.md](core/query-execution.md) → "Retries & 503s" |
| `SfError` classification / taxonomy | [query-execution.md](core/query-execution.md) → "SfError taxonomy" |
| bind-stage upload fails before the query runs | [query-execution.md](core/query-execution.md) → "Symptom: bind-stage upload fails" |
| OOM / out-of-memory on a large result set | [query-execution.md](core/query-execution.md) → "Symptom: OOM on a large result set" |
| slow / low-throughput large result set (not memory-bound) | [query-execution.md](core/query-execution.md) → "Symptom: slow (not memory-bound)" |
| chunk download fails mid-result | [query-execution.md](core/query-execution.md) → "Symptom: chunk download fails mid-result" |
| Arrow IPC parse error, result type mismatch | [query-execution.md](core/query-execution.md) → "Symptom: Arrow IPC parse error" |
| rows disappear after re-executing the same statement | [query-execution.md](core/query-execution.md) → "Symptom: rows disappear" |
| same DML ran twice / duplicate rows / duplicate side effects; idempotent DML, `MERGE`, `IF NOT EXISTS`, `WHERE NOT EXISTS` | [query-execution.md](core/query-execution.md) → "Symptom: the same DML appears to run twice" |
| legitimately long-running query (not hung); submit-and-poll vs one blocking call; `retry_timeout` does not bound query runtime | [query-execution.md](core/query-execution.md) → "Symptom: a legitimately long-running query" |

## Stage / cloud storage (PUT / GET)

| Match this | Go to |
|---|---|
| PUT/GET fails; S3 / Azure / GCS upload or download error | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "S3 transfers" / "Azure transfers" / "GCS transfers" |
| result retrieval fails after the query already succeeded; max retry on chunk download | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "Symptom: result retrieval fails after the query already succeeded" |
| stage credential expired mid-transfer | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "Credential vending" |
| DNS resolution fails for a stage / storage endpoint | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "Network prerequisites" |
| large parameter bindings upload | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "Large parameter bindings" |
| downloaded file has unexpected permissions (GET) | [stage-cloud-storage.md](core/stage-cloud-storage.md) → "Downloaded-file permissions" |

## PrivateLink

| Match this | Go to |
|---|---|
| `.privatelink.` host resolves to a **public** IP; private DNS not applied | [privatelink.md](core/privatelink.md) → "DNS is the most common failure" |
| PrivateLink connects but CRL / cert fetch fails | [privatelink.md](core/privatelink.md) → "CRL fetches still need public port 80" |

## Logging & diagnostics

| Match this | Go to |
|---|---|
| no log output; "logging is on but there's no file"; log level / path config | [logging-diagnostics.md](core/logging-diagnostics.md) → "log_path gates the core file layer" |
| OTLP / OpenTelemetry export not sending | [logging-diagnostics.md](core/logging-diagnostics.md) → "Forward-looking / not-yet-wired" |
| what is redacted / safe to share externally | [logging-diagnostics.md](core/logging-diagnostics.md) → "What is never in the logs" |
| reproduce a TLS failure with no wrapper involved | [tls-client-tool.md](core/tls-client-tool.md) |

## Wrapper-specific

### Python
| Match this | Go to |
|---|---|
| `Couldn't load core driver dependency (sf_core_python)`, `cannot import name 'sf_core_python'`, core extension won't import, ABI mismatch | [python.md](wrappers/python.md) → "Symptom: `OperationalError: Couldn't load core driver dependency" |
| `OSError: libssl.so.X: cannot open shared object file` | [python.md](wrappers/python.md) → "Symptom: OSError: libssl.so" |
| TLS error despite `REQUESTS_CA_BUNDLE` / `SSL_CERT_FILE` set (they are ignored) | [python.md](wrappers/python.md) → "Symptom: TLS/certificate errors despite" |
| Arrow → pandas / numpy, `ArrowInvalid`, type error | [python.md](wrappers/python.md) → "Symptom: Arrow→pandas conversion" |
| `fetchall()` returns `[]` | [python.md](wrappers/python.md) → "Symptom: `fetchall()` returns" |
| read `cursor.sfqid` / query ID, `cursor.description` column metadata, `ResultMetadata`, `describe()`, `execute_async` / `get_results_from_sfqid` | [python.md](wrappers/python.md) → "Reading result metadata and the query ID" |
| `DatabaseError: Connection is closed` (errno 250002), `InterfaceError: Cursor is closed` (errno 252006) | [python.md](wrappers/python.md) → "Connection is closed" |

### JDBC
| Match this | Go to |
|---|---|
| `UnsatisfiedLinkError`, JNI load, wrong architecture, shaded jar | [jdbc.md](wrappers/jdbc.md) → "Symptom: `UnsatisfiedLinkError`" |
| no driver logs appear (`loggerImpl` / `TRACING` / SLF4J); note **there is no `sf.logLevel`** | [jdbc.md](wrappers/jdbc.md) → "Symptom: no driver logs appear" |
| TLS error and the JVM truststore has no effect (rustls, not JSSE) | [jdbc.md](wrappers/jdbc.md) → "Symptom: TLS/certificate errors, and the JVM truststore has no effect" |
| JDBC Arrow OOM / off-heap (direct) memory; `--add-opens=java.base/java.nio`; `InaccessibleObjectException`; `OutOfMemoryError: Direct buffer memory`; `-XX:MaxDirectMemorySize` | [jdbc.md](wrappers/jdbc.md) → "Arrow result delivery" |
| JDBC proxy properties (`proxyHost` / `nonProxyHosts`), `queryTimeoutSeconds`, `allowUnderscoresInHost`, JDBC connection-property behavior | [jdbc.md](wrappers/jdbc.md) → "Connection properties: JDBC-specific behavior" |

### ODBC
| Match this | Go to |
|---|---|
| "Data source name not found"; `odbcinst.ini` / DSN; "Can't open lib" | [odbc.md](wrappers/odbc.md) → "Symptom: \"Data source name not found\"" |
| only a return code (`SQL_ERROR`) — need the real message | [odbc.md](wrappers/odbc.md) → "Symptom: a call fails but you only see the return code" |
| which INI holds driver settings; `sf.odbc.ini` location / not found; `SF_ODBC_INI`; minimal `odbc.ini` / DSN example | [odbc.md](wrappers/odbc.md) → "The driver's own settings file" |
| wide-string / `SQLWCHAR` text garbled, truncated, mojibake, extra NUL bytes; `DriverManagerEncoding`; UTF-16 vs UTF-32; unixODBC vs iODBC | [odbc.md](wrappers/odbc.md) → "Symptom: wide-string calls return garbled" |
| password with `;` in connection string; brace-quote a value; `duplicate key` / `unterminated brace` invalid connection string | [odbc.md](wrappers/odbc.md) → "Quoting connection-string values" |
| configure a DSN on Windows; `odbcad32.exe` / ODBC Data Source Administrator; password not saved in DSN | [odbc.md](wrappers/odbc.md) → "Configuring a DSN on Windows" |

### Node.js
| Match this | Go to |
|---|---|
| any Node.js wrapper symptom | [nodejs.md](wrappers/nodejs.md) — **private-preview stub**; wrapper-specific troubleshooting deferred, use the core pages |

### .NET
| Match this | Go to |
|---|---|
| any .NET wrapper symptom | [dotnet.md](wrappers/dotnet.md) — **private-preview stub**; wrapper-specific troubleshooting deferred, use the core pages |

## Don't know the subsystem?

| Match this | Go to |
|---|---|
| "what component owns this error?" — no idea where to start | [architecture.md](../architecture.md), then the [deep-dive index](index.md) |

---

Up: [deep-dive index](index.md) · [Troubleshooting Runbook](../troubleshooting-runbook.md)
