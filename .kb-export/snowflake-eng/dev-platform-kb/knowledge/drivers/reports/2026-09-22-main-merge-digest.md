# Snowflake Drivers (`snowflakedb/drivers`) — main merge digest

**Report period:** 2026-09-15 through 2026-09-22 (UTC)  
**Branch:** `main`  
**Head commit:** [`3d1804efb`](https://github.com/snowflakedb/drivers/commit/3d1804efbb091fd3bac8496c55cdc4f28c13d370) — *2026-09-22 07:15 UTC*  
**Commits merged to main:** 133  
**Source repository:** https://github.com/snowflakedb/drivers  

This digest summarizes what landed on `main` during the week. The Universal Driver (UD) stack is organized around **`sf_core`** (Rust), with language wrappers for **ODBC**, **JDBC**, **Python**, **Node.js**, and **.NET**.

---

## Executive summary

The week was dominated by **parity hardening** and **shared test expansion**, not green-field features:

1. **ODBC shared E2E coverage (SNOW-4036831)** — Large batches of Gherkin/shared scenarios now run against ODBC (PUT/GET, binding, geo types, timestamps, autocommit, session edge cases). Excel/Power Query replay tests saw final unskips (SNOW-3560627).
2. **PUT/GET resilience (SNOW-3838715, SNOW-4065116)** — S3 multipart and GCS resumable uploads resume only the failed part/chunk after token expiry; S3 client reuse and unsigned-payload PUT optimizations reduce transfer overhead.
3. **JDBC Auto-Connection (SNOW-3887958, 7 commits)** — Multi-part rollout: URL parsing, profile mapping, `getPropertyInfo`, SQLException mapping, and option resolution at the JDBC boundary.
4. **Node.js migration quality** — New Vitest E2E suites for numeric types, intervals, time, auth-browser flows; async batch fetch awaited in the bridge; cancel E2E hardened against server race `000605`.
5. **Python connector polish** — Native Arrow `DictCursor` rows, `executemany` accepts generators/iterators, restored connection parameters, array-bind fallback when `SYSTEM$BIND` stage is disabled.
6. **Reliability fixes** — ODBC key-pair heap corruption on Windows x64/Azure (SNOW-4108925), TLS handshake retries on connection reset (SNOW-4109090), JDBC/Python array-bind inline JSON retry (SNOW-4010544).

---

## New features and enhancements

### Cross-driver / `sf_core`

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Connection usability | SNOW-4115554, SNOW-4017511 | `connection_is_usable` / single-lock usability reporting; client app identity preserved when reusing `ConnectionConfig`; `validate_session_token` wrapper preset for Node.js parity with legacy deserialize flows. |
| Session serialization | SNOW-3927630 | Config to serialize backend session operations per connection. |
| Token caching default | SNOW-3990032 | Token caching enabled by default; MFA flows send both caching parameters. |
| TLS / cold start | SNOW-4138694 | Drop AWS-LC CPU jitter entropy source; avoid loading TLS trust store twice when unnecessary. |
| Telemetry | SNOW-3511564, SNOW-3989998 | Python telemetry client closes with connection; `CERT_REVOCATION_CHECK_MODE` in login `CLIENT_ENVIRONMENT`. |
| Proxy | SNOW-4109376 | Honor HTTPS hops to HTTP proxy. |

### PUT / GET / stage transfers

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Resumable uploads | SNOW-3838715 | S3 multipart + GCS resumable uploads retry only failed part/chunk after credential/token expiry (not full file restart). |
| Client efficiency | SNOW-4065116 | Reuse one `S3Client` per PUT/GET batch; send S3 PUT bodies without aws-chunked CRC32 trailers where applicable. |
| SQL surface | SNOW-4027518 | PUT/GET via `SQLPrepare` + `SQLExecute` (fixes server error `000007`). |
| Compression options | SNOW-3704972, SNOW-3704976 | `PUT_COMPRESSLV` gzip level and `PUT_TEMPDIR` for AUTO_COMPRESS temp files. |
| JDBC parity | SNOW-2881858 | GCS transfer SQLState / vendor-code parity with legacy JDBC. |
| GCP config | SNOW-4133517 | `GCS_USE_DOWNSCOPED_CREDENTIAL` E2E and GCP PUT/GET coverage. |

### JDBC

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Auto-Connection | SNOW-3887958 (series 1/9–6/9, `getPropertyInfo`) | Parses auto URLs, validates account hosts, maps profiles, resolves options, preserves suppressed failures on `SQLException` mapping. |
| Array bind fallback | SNOW-4010544 | Retry array-bind execute with inline JSON when `SYSTEM$BIND` stage is disabled. |
| Reference pin | — | JDBC reference driver bumped to **4.3.4**. |
| ArchUnit | SNOW-3735420 | Tighter JDBC ArchUnit exception-boundary rules. |

### ODBC

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Shared E2E | SNOW-4036831 | Broad shared-scenario coverage: DATE/TIME, timestamps, geo types, FILE/binary LOB, PUT/GET, parameter binding, autocommit, leftover query/session cases; remaining shared features covered. |
| Type info / binding | SNOW-3990021, SNOW-3990013, SNOW-3990009 | `SQLGetTypeInfo` TIMESTAMP `COLUMN_SIZE` default 29; ISO8601 `T` separator in TIMESTAMP_TZ CHAR parser; reject `SQL_DEFAULT_PARAM` (-5) with SQLSTATE `07S01`. |
| Performance | — | Fast-path DATE/TIME/TIMESTAMP `SQL_C_CHAR` kernel writes; direct-write BINARY hex in block fetch; batched CHAR write improvements. |
| Params / DSN | NO-SNOW | Deprecated DSN keys (`DEFAULT_VARCHAR_SIZE`, `DEFAULT_BINARY_SIZE`, logging keys, driver-manager keys) registered as ignored. |
| HTTP retries | SNOW-3990028 | Accept max HTTP retries; document unsupported retry params. |
| Catalog IRD | SNOW-4039377 | PK/FK catalog strings use IRD `SQL_WVARCHAR`. |
| Excel replay | SNOW-3560627 | Final unskips for Excel replay suite; PowerQuery navigator TIMESTAMP `COLUMN_SIZE` BD fix. |

### Python

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Arrow / cursor | SNOW-3926501 | Native-arrow `DictCursor` rows. |
| `executemany` | SNOW-4017505 | Accept generators and iterators. |
| Config restore | SNOW-4017511 | Restored missing connection parameters. |
| Array bind | SNOW-4010544 | Inline JSON retry when bind stage disabled. |
| Session cache | SNOW-3104303 | Parse `ALTER SESSION SET` into session cache (**Python-only** path). |
| Snowpark / pandas | SNOW-3390251, SNOW-3521138 | Quote Parquet field names in `write_pandas` COPY; tests for silent INTERVAL DAY TO SECOND overflow on pandas fetch. |
| Connections file | SNOW-4109376 | Parse `SNOWFLAKE_CONNECTIONS` as TOML (Snowpark `Session.builder` fix). |

### Node.js

| Theme | Tickets / commits | What changed |
| --- | --- | --- |
| Data type E2E | — | Vitest E2E for NUMBER, int, time, interval, numeric precision loss. |
| Bridge | SNOW-4109070 | Await async batch fetch in Node.js bridge. |
| Cancel | SNOW-4115633 | Harden cancel E2E against `000605` race. |
| Auth | SNOW-3996212 | External browser auth reference tests. |
| Errors | SNOW-3894966 | Hide napi status from codeless Node errors. |
| .NET | SNOW-4017940 | PAT E2E tests for .NET driver. |

---

## Bug fixes and reliability

| Area | Ticket | Summary |
| --- | --- | --- |
| ODBC / Windows | SNOW-4108925 | Fix key-pair connect **heap corruption** on Windows x64 Azure. |
| Network | SNOW-4109090 | Retry public TLS handshake GETs on connection reset. |
| ODBC binding | SNOW-4082445 | `SQL_ATTR_ROW_BIND_TYPE` bugfix (documented). |
| ODBC intervals | SNOW-4108929 | Fix INTERVAL DAY-TIME fetch for several subtypes. |
| ODBC Windows CI | SNOW-3734900 | Fix `SQLGetDescField` test hang on Windows. |
| ODBC config errors | NO-SNOW | Report error-class SQLSTATEs for client-local config rejection. |
| Auth tests | SNOW-4027777 | Mint fresh TOTP per MFA E2E connect (avoid reused passcodes). |
| Okta | SNOW-3990025 | Surface native Okta authenticator-request rejection reason. |
| PUT/GET ODBC | SNOW-4036831 | GET of unmatched staged path returns **empty result set** (not error). |
| JDBC describe | SNOW-4072351 | Forward bind values to describe-only `PREPARE`. |
| RowStatement | SNOW-4143089 | `RowStatement.getSqlText` from execute-time SQL. |

---

## Parity, behavioral differences, and test infrastructure

- **Shared test definitions** (`tests/definitions/`) continue to drive cross-driver scenarios; ODBC gained the largest share of new shared E2E wiring this week (SNOW-4036831).
- **Behavioral differences (BD)** registry remains the contract for intentional old-vs-new driver differences (`BehaviorDifferences.yaml` per driver). Notable maintenance: remove duplicated fixed BD entries; PowerQuery/Excel replay alignment for TIMESTAMP metadata.
- **JDBC Auto-Connection** is an in-flight multi-PR program (7 commits this week); expect more profile/URL edge cases until the series completes.
- **Reference drivers:** JDBC pin at 4.3.4; Node.js continues retiring `_old-driver-reference` tests as Vitest E2E lands (`BCR_LOG.md` tracks API behavior changes).

---

## Gaps and open risks

These are **not regressions** but active parity or product gaps called out in-repo:

| Gap | Owner / signal | Notes |
| --- | --- | --- |
| Node.js variant parsing | `nodejs/BCR_LOG.md` | JSON/XML variant path still slow, uses `eval()`, heavy deps; BCR suggests user-provided parsers like other drivers. |
| Node.js `RowStatement` API | BCR | `getNumRows` / `getQueryId` semantics vs TypeScript types; fetchRows vs streamRows typing split planned. |
| FLOAT special values | Node E2E `it.todo` | NaN/inf behavior blocked on upstream bug; documented in BCR. |
| `jsTreatIntegerAsBigInt` | BCR “Future” | May change post-UD; precision-loss behavior documented. |
| ODBC REAL / FLOAT CI | PR #1320 (May 2026) | Six REAL-type ODBC tests marked **flaky** on `ubuntu-x64-aws-json` (precision); still excluded from blocking CI until fixed. |
| MAP type ODBC decode | `sf_core/CHANGELOG.md` | Structured MAP may still need ODBC-specific decode arm (Python/JDBC paths improved in core). |
| JDBC Auto-Connection | SNOW-3887958 | Series explicitly multi-part (1/9–6/9 observed this week); remaining commits likely still open. |
| OCSP vs CRL | `BehaviorDifferences.yaml` | Revocation via CRL configuration; OCSP not implemented in UD path. |
| Python-only session cache | SNOW-3104303 | `ALTER SESSION SET` parsed from SQL only on Python; other wrappers use server-echoed params (intentional divergence). |

---

## CI, packaging, and developer experience

Recent **merged PRs (2026-05-20 – 2026-05-28)** visible via GitHub (69 PRs) — useful context when this digest’s commit window is ahead of PR metadata indexing:

| Label (approx.) | Count | Highlights |
| --- | ---: | --- |
| `sf_core` | 19 | S3 `UNSIGNED-PAYLOAD` on PutObject (gap 15); Duo `EXT_AUTHN_DUO_METHOD` push default; protoc download hardening. |
| `odbc` | 13 | `SQL_ATTR_MAX_ROWS` client-side only (no SQL LIMIT injection); WinODBC trace → replay tool; OpenSSL removed from ODBC **tests**. |
| `documentation` | 12 | `SECURITY.md`; Node E2E migrations; merge-queue scope docs. |
| `jdbc` | 10 | Private key auth, heartbeat, PAT E2E, statement release, JAR-bundled native lib, version constants. |
| `ci` | 9 | Merge queue scope reduction; Slack failure alerts; ODBC Mac `.pkg` artifacts; separate ODBC package workflow. |

**This week on `main` (commits):** additional CI work includes running `sf_params_spec` tests on CI, ODBC changelog sync from release branch, and continued deprecation of legacy DSN keys in the param registry.

---

## Suggested follow-ups for driver consumers

1. **Stage uploads/downloads:** Rely on resumable S3/GCS behavior after token expiry; verify client timeout/retry settings (`max_http_retries` on ODBC).
2. **JDBC Auto-Connection:** Test connection strings with multiple profiles before production rollout; watch for SQLException mapping changes.
3. **Python Snowpark:** Ensure `SNOWFLAKE_CONNECTIONS` is valid TOML; use new DictCursor Arrow path where applicable.
4. **Node.js:** Plan for BCRs around variant parsing and connection serialize/deserialize strictness.
5. **ODBC Excel/PQ:** Replay-generated tests are now largely unskipped — use them when validating BI tool traces.

---

## How this report was produced

- Git log on `main`: `git log main --since=2026-09-15`
- Merged PR metadata (when indexed): `gh pr list --repo snowflakedb/drivers --state merged`
- In-repo changelogs: `sf_core/CHANGELOG.md`, `nodejs/BCR_LOG.md`, `odbc/CHANGELOG.md`

**Next report:** advance window to commits after `3d1804efb` on the following `main` push trigger.
