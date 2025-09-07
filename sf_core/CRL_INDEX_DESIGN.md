## Parsed CRL Index: Low-Overhead Revocation Checks

### Current State (working, but expensive)

- The TLS verifier integrates CRL checks through `CrlCache::check_revocation(cert_der, issuer_der)`.
- The cache stores raw CRL bytes by URL (disk + in-memory LRU of bytes) and verifies CRL signatures.
- For each revocation check, we parse the CRL and scan the revoked list to look up the certificate serial.
- This is correct and passes E2E, but re-parsing is costly for large CRLs and repeated lookups.

### Goal

- Provide O(1) revocation lookups with minimal re-parsing and bounded memory, while preserving disk caching and signature verification.

### Public API (unchanged)

- `CrlCache::check_revocation(cert_der: &[u8], issuer_der: Option<&[u8]>) -> Result<RevocationOutcome, RevocationError>`
  - The cache remains the revocation oracle. Internals are free to evolve.

### Internal Model

- `ParsedCrl` (one per CRL URL), stored in an in-memory LRU keyed by URL:
  - `url: String`
  - `this_update: DateTime<Utc>`
  - `next_update: Option<DateTime<Utc>>`
  - `akid_key_id: Option<Vec<u8>>` (CRL AKID, for AKID/SKID checks)
  - `issuer_name_raw: Vec<u8>` (or canonical form) for issuer-subject equality checks
  - `signature_verified: bool`
  - `serial_index: HashSet<SerialKey>` where `SerialKey = Box<[u8]>` (normalized big-endian serial without leading zeros)
  - optional (future): `revoked_reason_time: Option<HashMap<SerialKey, (Option<Reason>, Option<DateTime<Utc>>)>>`

### Build Path (on load/fetch)

1. Load raw DER via existing disk/network paths.
2. Verify CRL signature once:
   - Canonical TBS-CRL via `x509-cert` + `ring`; fallback `openssl` for RSASSA-PSS params.
   - Enforce issuer-subject match, AKID/SKID linkage, no delta CRL, and critical extension policy.
3. Stream-parse revoked entries to build `serial_index`:
   - Iterate `TBSCertList.revokedCertificates` with `der-parser` to avoid materializing the full vector.
   - Normalize each serial to big-endian bytes and insert into `serial_index`.
4. Insert `ParsedCrl` into the parsed-LRU (drop in-memory DER; keep DER only on disk).
5. Keep existing half-life background refresh. The refresh rebuilds `ParsedCrl` and atomically swaps it into the LRU.

### Lookup Path

- `check_revocation(cert, issuer)`:
  - Extract CRL Distribution Points (HTTP-only) and the cert serial.
  - For each URL:
    - `get_parsed(url)`: from parsed-LRU or build as above.
    - Validate issuer linkage against `ParsedCrl` metadata. If mismatched → skip.
    - If `signature_verified == false` → `NotDetermined` (policy can fail-closed upstream).
    - Check `serial_index.contains(serial)`:
      - true → `Revoked`
      - false → test next URL
  - Aggregate: any Revoked → Revoked; any Checked and none revoked → NotRevoked; otherwise → NotDetermined.

### Disk Sidecar Index (optional, phase 2)

- Emit `<digest>.idx` next to the CRL with:
  - Version, URL digest, this/nextUpdate, AKID, issuer hash, count, and compact serial encoding.
- On load: validate metadata and hydrate the `serial_index` directly (skip DER parsing).
- Fallback to rebuilding if mismatch/corrupt.

### Memory & Performance

- LRU by parsed entry count; configurable capacity. Each entry holds only metadata + serial set.
- Consider `ahash::AHashSet` for reduced overhead; Bloom filter pre-check is an optional future optimization.
- For extremely large CRLs, the streaming parser minimizes peak memory during index build.

### Policy & Errors

- Errors unify under `RevocationError` (CRL/OCSP/Policy/Internal).
- TLS verifier continues to interpret outcomes by mode:
  - Enabled: fail on Revoked or error.
  - Advisory: log and allow on error/indeterminate; fail only on Revoked.

### Concurrency

- Keep per-URL single-flight locks to ensure one builder per URL.
- Background refresh builds a new `ParsedCrl`, writes disk, then swaps into the LRU.

### Telemetry/Logging

- Log parsed-LRU hit/miss, disk hit/miss, entries indexed, signature verification status, and half-life refresh scheduling.

### Phasing

- Phase 1 (implement now): `ParsedCrl` + parsed-LRU + streaming index build + single-flight + outcome aggregation.
- Phase 2 (optional): sidecar index read/write for warm starts.
- Phase 3 (optional): Bloom filter pre-check for huge CRLs.

### Compatibility

- Public API for revocation remains unchanged.
- Existing disk cache and network logic remain intact.
- This design does not alter caller behavior; it only reduces CPU/memory overhead internally.


