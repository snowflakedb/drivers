# Stage & cloud storage

PUT/GET file transfers, result-chunk downloads, large parameter bindings,
credential vending, and per-provider troubleshooting. Part of the
[troubleshooting deep-dive](../index.md); the
[Troubleshooting Runbook](../../troubleshooting-runbook.md) **Login OK,
stage/PUT/GET fails** row (Appendix A) routes here.

---

## Network prerequisites — firewall & DNS

Cloud-storage access (PUT, GET, result-chunk downloads) requires the client to
reach S3, Azure Blob, or GCS endpoints **directly**. These endpoints are separate
from `*.snowflakecomputing.com` and are blocked by default in many corporate
firewalls.

**This is the most common root cause of stage-related failures.** Before
investigating credentials, driver configuration, or server-side issues, confirm
network reachability.

### Step 1 — obtain the required endpoint list

```sql
-- Public endpoint deployments:
SELECT * FROM TABLE(SYSTEM$ALLOWLIST());

-- PrivateLink deployments (use in addition to, or instead of, the above):
SELECT * FROM TABLE(SYSTEM$ALLOWLIST_PRIVATELINK());
```

This returns every hostname the driver needs: the Snowflake control plane, CRL
distribution points, an OCSP-cache entry, and the cloud-storage hostnames for
your account's stage region.

> The allowlist output lists an **OCSP cache** entry because the function is
> shared across clients. The Universal Driver performs **CRL-based** revocation
> only and does not use OCSP — see [crl-revocation.md](crl-tls/crl-revocation.md).
> You still need the CRL distribution points reachable when `crl_check_mode` is
> `ENABLED`/`ADVISORY`.

If `ENABLE_INTERNAL_STAGES_PRIVATELINK` is set on the account, stage traffic
(PUT/GET and result-chunk downloads) also routes through the PrivateLink
endpoint — `SYSTEM$ALLOWLIST_PRIVATELINK()` then includes the internal-stage
hostnames. See [privatelink.md](privatelink.md) for the two PrivateLink scopes.

### Step 2 — test each storage endpoint

```sh
# S3 (port 443):
curl -v --max-time 10 https://<bucket>.s3.<region>.amazonaws.com/

# Azure Blob:
curl -v --max-time 10 https://<account>.blob.core.windows.net/

# GCS:
curl -v --max-time 10 https://storage.googleapis.com/
```

What to watch for:

- **DNS failure** — `Could not resolve host` means the client's DNS cannot reach
  the cloud provider's resolver. Stage hostnames are dynamically allocated and
  cannot be covered by static hosts-file entries — ensure the client uses the
  cloud provider's DNS (AWS Route 53, Azure DNS, GCP Cloud DNS).
- **Connection refused / timeout** — TCP is blocked before the TLS handshake. A
  firewall rule is preventing outbound HTTPS (443) to the storage endpoint. Not a
  driver issue.
- **TLS handshake or certificate error** — the endpoint is reachable but TLS is
  failing; see [cert-chain.md](crl-tls/cert-chain.md) and
  [tls-handshake.md](crl-tls/tls-handshake.md).
- **HTTP 4xx / 403** — connectivity is fine; the error is credential- or
  permission-related (below).

---

## When cloud storage is accessed

Three distinct paths trigger cloud-storage I/O:

| Trigger | Provider | Credentials |
|---|---|---|
| `PUT` / `GET` SQL command | S3, Azure, or GCS | STS / SAS / bearer in the `StageInfo` response |
| Result chunks exceeding the memory threshold | Same as the session's stage | Presigned URLs in the query response |
| Large parameter bindings | `@SYSTEM$BIND/{uuid}` | Session-scoped, auto-provisioned |

### Execution flow

```
perform_put_get_transfer()          sf_core/src/apis/database_driver_v1/query.rs
  └─ extract StageInfo from query response
       StageInfo.location_type → LocationType enum   sf_core/src/file_manager/types.rs
       dispatch: sf_core/src/file_manager/mod.rs
         ├─ LocationType::S3     → s3_transfer.rs
         ├─ LocationType::Azure  → azure_transfer.rs
         └─ LocationType::Gcs    → gcs_transfer.rs

Large bindings: upload_csv_bindings()   sf_core/src/stage_binding.rs
```

---

## Credential vending

Credentials arrive embedded in the Snowflake query response in the `StageInfo`
struct (`sf_core/src/file_manager/types.rs`):

| Provider | Credential type |
|---|---|
| S3 | STS temporary keys (`aws_key_id`, `aws_secret_key`, `aws_token`) |
| Azure | SAS token |
| GCS | Bearer token or presigned URL |

Each provider has a refresher implementing the `StageInfoRefresher` trait. When
cloud storage rejects vended credentials as expired (403/401), the driver
**re-issues the original PUT/GET SQL** to obtain fresh credentials and retries.
Successful refreshes are cached for **~10 minutes** so concurrent transfers
coalesce onto one refresh instead of hammering the server.

---

## Proxy & the transfer clients

Cloud-storage transfers do **not** use a dedicated proxy setting — they inherit
the connection's proxy configuration:

- An explicit `proxy_host` / `proxy_port` **is** applied to PUT/GET transfers.
- `HTTP_PROXY` / `HTTPS_PROXY` env vars are honored **only when
  `use_proxy_env=true`** (same policy as the main REST client).

So `no_proxy` matters for transfers only when you are relying on env-var proxying
(`use_proxy_env=true`) — with an explicit `proxy_host`, transfers route through
it directly. This differs from the CRL-fetch and cloud-metadata clients, which
ignore `proxy_host` and always auto-detect env proxy. See the full matrix in
[proxy-tls.md](crl-tls/proxy-tls.md#which-clients-honor-the-proxy).

---

## Downloaded-file permissions (GET)

On Unix, a GET download writes its output (`.part`) file **owner-only (`0600`)**
by default (`create_output_file`, `sf_core/src/file_manager/mod.rs`). Setting
`unsafe_file_write=true` reverts to the process umask (typically `0644`). The
setting is **Unix-only** — ignored on Windows.

If a downloaded file has unexpectedly tight permissions, that is the secure
default, not a bug; only relax it with `unsafe_file_write=true` when a downstream
consumer genuinely needs group/other read.

---

## S3 transfers

Upload & download: `sf_core/src/file_manager/s3_transfer.rs` (SDK: `aws-sdk-s3`
v1.x + `aws-sdk-sts`).

**403 AccessDenied:**
- The STS session credentials have expired. The driver refreshes automatically;
  if it isn't:
  1. Check whether the ~10-minute refresh window is holding back a re-vend (look
     for refresh/coalesce lines in DEBUG logs).
  2. Confirm the warehouse/role has permission to vend STS credentials for the
     stage.

**"Could not connect to the endpoint URL":**
- Check whether the stage uses a VPC endpoint or PrivateLink — the presigned URL
  may contain a hostname reachable only from inside a VPC
  ([privatelink.md](privatelink.md)).
- If you proxy via env vars (`use_proxy_env=true`), verify `no_proxy` does **not**
  exclude the S3 endpoint hostname (see
  [Proxy & the transfer clients](#proxy--the-transfer-clients)).

**Checksum mismatch on download:**
- Enable DEBUG logging to see per-chunk ETags; a single corrupt chunk invalidates
  the result set.
- Re-run the query — chunk downloads are not retried automatically on checksum
  failure.

---

## Azure transfers

Upload & download: `sf_core/src/file_manager/azure_transfer.rs`.

**403 AuthorizationFailure:**
- SAS token expired. Should auto-refresh; if not, check the refresh logs.
- SAS token tied to an IP range: if the client IP changes mid-transfer, the SAS
  is invalidated.

**Slow uploads to Azure:**
- Azure Blob has block-size limits. Large files are split into blocks; check
  whether the block-size configuration matches the storage-account tier.

---

## GCS transfers

Upload & download: `sf_core/src/file_manager/gcs_transfer.rs`.

**401 on presigned URL:**
- Presigned GCS URLs expire quickly (typically 1 hour). If the query took a long
  time to return, the presigned URLs in the response may already be expired.
- Resolution: re-run the query immediately; the new response carries fresh URLs.

---

## Large parameter bindings

When binding values exceed the inline threshold, `upload_csv_bindings()`
(`sf_core/src/stage_binding.rs`) uploads a CSV to `@SYSTEM$BIND/{uuid}` before
sending the query; the request's `bindStage` field points at this file.

**Symptom: bind upload fails, query never reaches the server.**
1. Check TLS / network errors for the stage upload (same path as a `PUT`).
2. Verify the role has `CREATE STAGE` / write access on `@SYSTEM$BIND`.

This surfaces to the caller as a **stage** error on the query path — see also the
bind-stage symptom in [query-execution.md](query-execution.md#symptom-bind-stage-upload-fails-before-the-query-runs).

---

## Result-chunk downloads

Large result sets overflow to cloud storage. Chunks arrive as presigned URLs in
the query response (`sf_core/src/rest/snowflake/query_response.rs`); the download
path is `sf_core/src/chunks/` (`get_chunk_data`). Concurrency and memory budget
are shared with the result-fetch machinery documented in
[query-execution.md](query-execution.md#result-fetch)
(`CLIENT_PREFETCH_THREADS`, `CLIENT_MEMORY_LIMIT`).

### Symptom: result retrieval fails after the query already succeeded

**Error patterns:** `Max retry reached for the download of chunk#N`,
`connection refused` / `connection reset` on chunk fetch, or fetch succeeding for
small results but failing for large ones.

This profile — the query executes and returns a response, but result retrieval
then fails — almost always means the client cannot reach the cloud-storage
endpoint. The query ran server-side; the result chunks live in S3/Azure/GCS; the
driver cannot download them.

**Diagnosis:**
1. Follow [Network prerequisites](#network-prerequisites--firewall--dns). Run
   `SYSTEM$ALLOWLIST()` and verify the storage endpoints are reachable.
2. If the error names a specific hostname, test it directly with `curl`.
3. If it's intermittent (works sometimes): presigned URLs have a short expiry. If
   the wrapper fetches long after `execute()`, the URLs may have expired — re-run
   and fetch immediately.
4. If it's user-specific on the same driver/account: check for per-user firewall
   rules or network segments (VPN split-tunnel, per-user proxy assignments).

**Resolution:** Add the cloud-storage endpoints to the firewall allow list. Use
`SYSTEM$ALLOWLIST()` as the authoritative source — hardcoding IP ranges is
unreliable because cloud-provider IPs change.

---

## Quick reference

| Setting / parameter | Default | Effect |
|---|---|---|
| `CLIENT_PREFETCH_THREADS` | 4 | Concurrent chunk-download threads |
| `CLIENT_MEMORY_LIMIT` | 1536 (MB) | In-flight chunk memory budget |
| `unsafe_file_write` | `false` | `true` → GET downloads use umask perms instead of owner-only `0600` (Unix-only) |
| Stage-credential refresh window | ~10 min | Successful re-vends coalesce within this window |

---

Related: [query-execution.md](query-execution.md) ·
[privatelink.md](privatelink.md) ·
[proxy-tls.md](crl-tls/proxy-tls.md) ·
[cert-chain.md](crl-tls/cert-chain.md) · up to the
[deep-dive index](../index.md) · [Runbook](../../troubleshooting-runbook.md)
