# Token cache key format and file-based fallback (cross-driver contract)

> Language-agnostic specification. Every Snowflake driver that caches
> authentication tokens MUST produce byte-identical cache keys from the same
> inputs, so that the golden vectors in [§8](#8-golden-vectors) reproduce
> exactly in every language. This document is the single source of truth; driver
> PRs should link to it rather than restating the rules.
>
> Reference implementation: [`sf_core/src/token_cache/mod.rs`](../../sf_core/src/token_cache/mod.rs)
> (key construction and normalization) and
> [`sf_core/src/token_cache/file_cache.rs`](../../sf_core/src/token_cache/file_cache.rs)
> (file-based fallback). Platform storage and the security posture of the file
> fallback are covered in
> [`token-cache-platform-security.md`](./token-cache-platform-security.md).

## 1. Why this format exists

The original cache key was `<host>:<username>:<token-type>` (stored verbatim in
the OS keystore, or as `SHA256(<host>:<username>:<token-type>)` in the JSON file
fallback). It caused token collisions in two environments:

- **Multi-account**: the IdP host does not uniquely identify a Snowflake
  account. Several accounts can share one identity provider, so one account's
  token overwrote another's.
- **Multi-role**: the key had no role dimension. OAuth tokens are scoped to a
  role, so connecting with a new role overwrote the previously cached token.

The `v2` format fixes both by hashing a versioned, canonical JSON object that
includes the Snowflake account URL and (for OAuth) the role. The key is built
once and stored verbatim in **both** the OS keystore and the file fallback.

All three changes are required together: fixing the host without adding the role
still leaves the multi-role collision, and vice versa. Drivers must converge on
this single, strict key format.

## 2. Key format

```
SnowflakeTokenCache.v2.<TokenType>.<sha256hex(keyData)>
```

Four dot-separated segments:

| Segment | Value |
|---|---|
| Prefix | The literal `SnowflakeTokenCache`. |
| Version | The literal `v2`. The version tracks the **key format**, not the file name (see [§11](#11-migration-and-compatibility)). |
| Token type | A PascalCase token-type string (see [§3](#3-token-types)). Lives in the readable prefix — **not** inside `keyData` — so keystore/keyring tooling can identify and purge a specific token class without decoding the hash. |
| Hash | **Lowercase** hex SHA-256 of the canonical JSON bytes of `keyData` (see [§5](#5-canonical-json-and-hashing)). |

Example:

```
SnowflakeTokenCache.v2.MfaToken.10c5dde84bb8f584c0df06ea826d418c4f580e08f9db10187c0cb5e2a732a0d6
```

## 3. Token types

The token-type segment is one of these exact PascalCase strings. Note the
casing: `Oauth`, not `OAuth`.

| Token type | Auth flow | `keyData` shape |
|---|---|---|
| `IdToken` | External-browser SSO (ID token) | MFA/ID (2 fields) |
| `MfaToken` | MFA | MFA/ID (2 fields) |
| `OauthAccessToken` | OAuth access token | OAuth (4 fields) |
| `OauthRefreshToken` | OAuth refresh token | OAuth (4 fields) |
| `DpopBundledAccessToken` | DPoP-bound OAuth access token | OAuth (4 fields) |

Drivers whose internal enums use different spellings (e.g. `MFATOKEN`,
`MFAToken`, `OAUTH_ACCESS_TOKEN`, `USERNAME_PASSWORD_MFA`) MUST map to the exact
strings above when building the key. The token type a driver does not implement
is simply never produced; only `DpopBundledAccessToken` is used as a test
literal in the golden vectors and need not exist as a real flow in every driver.

## 4. `keyData` — two flow-specific shapes

`keyData` is a JSON object whose fields depend on the authentication flow. It
**never** contains the token type.

### 4.1 OAuth flows (`OauthAccessToken`, `OauthRefreshToken`, `DpopBundledAccessToken`)

OAuth tokens are scoped to a Snowflake account, an identity provider, a user,
and a role, so all four dimensions enter `keyData`:

| Field | Value |
|---|---|
| `idp` | Normalized IdP token-endpoint URL (see [§4.3](#43-idp-is-the-full-token-endpoint-url)). |
| `role` | Normalized role name (empty string when no role is configured). |
| `snowflake` | Normalized Snowflake server URL. |
| `username` | Normalized Snowflake username. |

Sorted field order in the canonical JSON: `idp`, `role`, `snowflake`,
`username`.

### 4.2 MFA and ID token flows (`MfaToken`, `IdToken`)

MFA and external-browser ID-token flows always target the Snowflake host
directly and never embed a role in the authentication call, so `idp` and `role`
are **absent** from `keyData`:

| Field | Value |
|---|---|
| `snowflake` | Normalized Snowflake server URL. |
| `username` | Normalized Snowflake username. |

Sorted field order in the canonical JSON: `snowflake`, `username`.

Including `idp` or `role` here would change the hash on every connection and
cause a permanent cache miss.

### 4.3 `idp` is the full token-endpoint URL

`idp` is the **full token-endpoint URL**, not the bare hostname. Customers who
run IdP endpoints that differ only by port or path (for example a Prod vs. Dev
tenant on the same host) stay disambiguated. Passing only the hostname
reintroduces collisions. For Snowflake-native OAuth, `idp` is the Snowflake
token-request URL.

### 4.4 Field population by flow

| Flow | `idp` | `role` | `snowflake` | `username` |
|---|---|---|---|---|
| OAuth (auth code / refresh / DPoP) | token-endpoint URL | role, or `""` | Snowflake server URL | username |
| MFA | *absent* | *absent* | Snowflake server URL | username |
| External-browser ID token | *absent* | *absent* | Snowflake server URL | username |

## 5. Canonical JSON and hashing

The JSON bytes that are hashed MUST be byte-for-byte identical across all
drivers:

1. **Compact**: no spaces after `:` or `,`, no newlines, no indentation.
2. **Keys sorted lexicographically** by Unicode code point, ascending. Both
   shapes are already in sorted order as listed in [§4](#4-keydata--two-flow-specific-shapes),
   but the serializer must enforce the sort rather than relying on it — many
   JSON libraries serialize in insertion order, not sorted order.
3. **Standard JSON string escaping**: `"` becomes `\"`, `\` becomes `\\`,
   control characters are escaped. No non-ASCII escaping beyond what standard
   JSON requires.
4. Serialize to **UTF-8 bytes**, apply **SHA-256**, and render the digest as
   **lowercase** hex.

Hashing happens **exactly once**, when the key is built. Both storage backends
(OS keystore and file fallback) receive and store the already-built key string
verbatim. No backend re-hashes the key it is given. (A prior defect in some
drivers applied a second SHA-256 inside the file backend — that must not
happen.)

Language notes (all equivalent, each must yield the same bytes):

- Sort keys explicitly whenever the serializer preserves insertion order.
- Render the digest in lowercase; some hex helpers emit uppercase.
- Validate the token type maps to the exact PascalCase string in [§3](#3-token-types).

Pseudocode:

```text
# Step 1 — Normalize all inputs before calling build_cache_key.
# Callers supply already-normalized fields; build_cache_key only serializes and hashes.
snowflake_norm = normalize_url(snowflake)
username_norm  = normalize_identifier(username)
idp_norm       = normalize_url(idp)          # "" for MFA/ID; non-empty required for OAuth
role_norm      = normalize_identifier(role)  # "" when no role is configured

# Step 2 — Validate normalized values before storing.
# Reject (do not hash) if snowflake_norm or username_norm is empty, or if
# idp_norm is empty for an OAuth token type.

# Step 3 — Build the cache key from already-normalized fields.
function build_cache_key(token_type, idp_norm, role_norm, snowflake_norm, username_norm):
    if token_type in {OauthAccessToken, OauthRefreshToken, DpopBundledAccessToken}:
        keyData = {
            "idp":       idp_norm,
            "role":      role_norm,
            "snowflake": snowflake_norm,
            "username":  username_norm,
        }
    else:  # MfaToken, IdToken
        keyData = {
            "snowflake": snowflake_norm,
            "username":  username_norm,
        }
    json   = compact_json_with_sorted_keys(keyData)       # UTF-8
    digest = lowercase_hex(sha256(json))
    return "SnowflakeTokenCache.v2." + token_type + "." + digest
```

## 6. Normalization

Normalization makes semantically-equal inputs produce the same key while keeping
genuinely-different inputs distinct.

### 6.1 `normalize_url` — applies to `idp` and `snowflake`

Operating on the **raw URL string** (not a value produced by a URL parser):

1. Strip the scheme: remove everything up to and including the first `://`.
2. Drop the query string (everything from the first `?`).
3. Drop the fragment (everything from the first `#`).
4. Strip optional userinfo (`user:pass@`) **from the authority only**. The
   authority ends at the first `/`; an `@` that appears before that `/` is a
   userinfo delimiter and the authority is replaced with the text after it. An
   `@` that appears **after** the first `/` is part of the path and is
   preserved. (Query and fragment are removed first, so an `@` that lived only
   in a query string is never mistaken for a userinfo delimiter.)
5. Trim every trailing `/`.
6. **Lowercase** the entire remainder (host + optional `:port` + optional
   `/path`).

Important details:

- **Explicit default ports are preserved.** An explicitly-written `:443` on an
  HTTPS URL stays in the key. This is why the raw string is parsed directly:
  most URL libraries normalize an explicit default port away, which would change
  the hash. If a driver uses a URL parser, it MUST re-add an explicitly-written
  default port and preserve the original path encoding.
- **Trailing slashes.** The reference trims *all* trailing slashes from the
  remainder, so `host/oauth/` and `host/oauth` collapse to the same value
  (`host/oauth`). This is intentional: the two denote the same endpoint, and all
  drivers must agree. Do not implement a "root-only" trailing-slash rule — it
  would diverge from the reference and break parity.
- **Userinfo scope.** Strip `@` only within the authority. A path such as
  `host/oauth/@handle/token` must keep `@handle/token`. A whole-string "cut at
  first `@`" is a defect that truncates such paths.

Examples:

```text
https://login.microsoftonline.com:443/tenant-id/oauth2/v2.0
  -> login.microsoftonline.com:443/tenant-id/oauth2/v2.0

https://myorg-myaccount.privatelink.snowflakecomputing.com
  -> myorg-myaccount.privatelink.snowflakecomputing.com

https://host.example.com                 (implicit default port)
  -> host.example.com

https://user:pass@host.example.com/Path
  -> host.example.com/path

https://idp.example.com/oauth/token/      (trailing slash trimmed)
  -> idp.example.com/oauth/token

https://host.example.com/oauth/@handle/token
  -> host.example.com/oauth/@handle/token
```

### 6.2 `normalize_identifier` — applies to `username` and `role`

- If the value contains **any** double-quote character (`"`), return it
  **verbatim** — quoted identifiers are case-sensitive in SQL and carry the
  user's intent exactly, including lowercase letters and spaces inside and
  outside the quotes.
- Otherwise, **lowercase the entire value** — unquoted Snowflake identifiers are
  case-insensitive, so lowercasing yields a stable canonical form.

Examples:

```text
USER@DOMAIN.COM                       -> user@domain.com
"First Last"@domain.com               -> "First Last"@domain.com   (verbatim: contains ")
"Analyst Role":north_america:prod     -> "Analyst Role":north_america:prod   (verbatim)
```

> The quote test is whole-value: the presence of a single `"` anywhere makes the
> entire value verbatim (it is **not** a per-character "uppercase outside
> quotes" transform). Lowercase content outside the quotes is preserved.

## 7. Validation

- `snowflake` MUST be non-empty — reject (error) otherwise.
- `username` MUST be non-empty — reject (error) otherwise.
- `idp` MUST be non-empty for OAuth flows; it is allowed to be empty for MFA/ID
  flows (absent by design).
- `role` MAY be empty for OAuth when no role is configured.
- Legacy separator-injection guards (rejecting `;` or `:` in inputs) are removed
  — hashing makes them unnecessary.

## 8. Golden vectors

Every driver MUST reproduce both keys exactly. Add unit tests that assert them.

### Vector A — OAuth flow

Raw inputs (pre-normalization). The token type `DpopBundledAccessToken` is a
test literal; pass it directly without requiring a real DPoP flow:

```text
token_type : DpopBundledAccessToken   (key prefix only — not in keyData)
idp        : https://login.microsoftonline.com:443/tenant-id/oauth2/v2.0
role       : "Analyst Role With Spaces":north_america:prod:readonly
snowflake  : https://myorg-myaccount.privatelink.snowflakecomputing.com
username   : "First Last"@long-corporate-domain.example.com
```

After normalization (`First Last` and `Analyst Role With Spaces` keep their case
because the whole value contains `"`; URL fields are lowercased and keep `:443`):

```text
idp        : login.microsoftonline.com:443/tenant-id/oauth2/v2.0
role       : "Analyst Role With Spaces":north_america:prod:readonly
snowflake  : myorg-myaccount.privatelink.snowflakecomputing.com
username   : "First Last"@long-corporate-domain.example.com
```

Canonical JSON (compact, sorted keys, 4 OAuth fields, no `token_type`):

```json
{"idp":"login.microsoftonline.com:443/tenant-id/oauth2/v2.0","role":"\"Analyst Role With Spaces\":north_america:prod:readonly","snowflake":"myorg-myaccount.privatelink.snowflakecomputing.com","username":"\"First Last\"@long-corporate-domain.example.com"}
```

Expected key:

```
SnowflakeTokenCache.v2.DpopBundledAccessToken.741b6d66d252666d6821bfd19e0151511cf4efdaaeba2b3c87673aa4de6d2c0b
```

### Vector B — MFA flow

Raw inputs:

```text
token_type : MfaToken   (key prefix only — not in keyData)
snowflake  : https://myorg-myaccount.privatelink.snowflakecomputing.com
username   : "First Last"@long-corporate-domain.example.com
```

After normalization:

```text
snowflake  : myorg-myaccount.privatelink.snowflakecomputing.com
username   : "First Last"@long-corporate-domain.example.com
```

Canonical JSON (compact, sorted keys, 2 MFA/ID fields, no `idp`/`role`/`token_type`):

```json
{"snowflake":"myorg-myaccount.privatelink.snowflakecomputing.com","username":"\"First Last\"@long-corporate-domain.example.com"}
```

Expected key:

```
SnowflakeTokenCache.v2.MfaToken.10c5dde84bb8f584c0df06ea826d418c4f580e08f9db10187c0cb5e2a732a0d6
```

### Dimension-isolation properties to test

- Same IdP + different Snowflake host -> different OAuth keys.
- Same host/user + different role -> different OAuth keys.
- MFA and OAuth for the same user/host -> different keys (different prefix and
  field set).
- Different token-type prefix -> different keys by construction.

## 9. File-based fallback

When no OS keystore is reachable, the driver stores tokens in a JSON file. The
same final key string from [§2](#2-key-format) is the map key; the token value
is stored verbatim with no additional hashing.

### 9.1 File location

The cache **directory** is resolved in this priority order:

1. `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR` — used as-is; it MUST already be a
   directory. A set but non-directory value is an immediate error, not a
   fall-through to the next option.
2. `$XDG_CACHE_HOME/snowflake` — `$XDG_CACHE_HOME` MUST exist; the `snowflake`
   subdirectory is created.
3. `$HOME/.cache/snowflake` — `$HOME` MUST exist; `.cache` and `snowflake` are
   created as needed.

An empty string for any of these variables is treated as unset and the next
option is tried. If none resolves, the file fallback is unavailable (an error
is raised, not a silent miss). Any directory the driver creates is made with
mode `0700` (owner-only) on Unix.

The cache **file name** defaults to `credential_cache_v2.json` and can be
overridden with `$SF_TEMPORARY_CREDENTIAL_CACHE_FILE_NAME`. The file name is
independent of the key-format version: the `v2` in the key prefix does not change
the file name, and the file name does not change when the key format revs.

### 9.2 File format

A single JSON object mapping each cache key to its token value:

```json
{
  "tokens": {
    "SnowflakeTokenCache.v2.MfaToken.10c5dde8…": "<token-value>",
    "SnowflakeTokenCache.v2.OauthAccessToken.…": "<token-value>"
  }
}
```

- Keys are stored **verbatim** — exactly the `build_cache_key` output, never a
  hash-of-a-hash.
- Values are the raw token strings (UTF-8).
- The reference serializes this as pretty-printed JSON; the exact whitespace of
  the file is not part of any cross-driver contract (only the key strings are).

### 9.3 Permissions and integrity (Unix)

- The file is created atomically with `O_EXCL` + `O_NOFOLLOW` at mode `0600`.
- Before any read or write, the already-open file descriptor is validated with
  `fstat` (not a path `stat`, to avoid TOCTOU):
  - **Regular file** — symlinks and special files are rejected (`O_NOFOLLOW`
    yields `ELOOP`, mapped to an "irregular file type" error).
  - **Ownership** — the file's UID must equal the current process UID, otherwise
    the operation fails.
  - **Mode** — the permission bits must be exactly `0600`; any other mode
    (e.g. `0644`) is refused for **both** read and write, and the offending
    file is left untouched.

### 9.4 Windows caveat

On Windows the file is created and written with the platform's standard file
APIs; the `fstat`/ownership/mode enforcement above is Unix-only. See the
security posture doc for the fail-open implications.

### 9.5 Concurrency and locking

- Every `set` / `get` / `delete` acquires an **exclusive OS-level advisory
  lock** (`flock` on Unix, `LockFileEx` on Windows) before touching the file.
- The lock is held on a sidecar file named `<cache-file>.lck` (created at
  `0600`). The `.lck` file persists on disk but holds no lock once the handle is
  closed; this is harmless and avoids a delete/recreate race.
- Lock acquisition retries with **decorrelated jitter** backoff (the reference
  uses up to 20 attempts, 100 ms base delay; each retry sleeps a random duration
  in `[base, min(3 × previous, 16 × base)]`). If the budget is exhausted, the
  operation fails rather than proceeding unlocked.
- On Unix, writes reuse the open file descriptor: seek to start, truncate,
  write, flush — no separate temp-file rename. On Windows the reference uses
  `fs::write` on the path directly; neither platform uses a temp-file rename.

### 9.6 Operation semantics

| Operation | Behavior |
|---|---|
| **set** | Lock, open-or-create the file, insert/overwrite the entry under the key, write back. |
| **get** | Lock, read the file strictly; return the value for the key, or "not found" if the key, file, or valid JSON is absent. |
| **delete** | Lock, open the file, remove the entry; report whether it existed. Writing back is skipped if nothing changed. |

Parsing leniency:

- A missing cache file is a cache miss, not an error.
- On the read path, empty content is a cache miss (`None`) with no log;
  invalid JSON is logged and treated as no token.
- On the write path, empty content is replaced by a fresh empty cache with no
  log; invalid JSON is logged and replaced, so a corrupt file self-heals on the
  next write.

## 10. Single hashing point

The final key is built once by `build_cache_key`. Neither storage backend
re-hashes it. When auditing a driver, confirm there is no leftover secondary
hashing inside the file or keystore backend, and that both backends store the
identical string.

## 11. Migration and compatibility

No reader looks up the original v1 keys or any earlier v2 variant (the
superseded form used a 5-field `keyData` that embedded `token_type`, uppercase
URL/identifier normalization, and `SCREAMING_SNAKE_CASE` token types). Those
older entries simply become orphaned; the next connection transparently
re-authenticates and writes a fresh entry in the current format. Active cleanup
of orphaned entries is out of scope. The cache file name stays
`credential_cache_v2.json` across key-format revisions.

## 12. Definition of done (per driver)

- [ ] Golden vector A (OAuth) passes byte-exact.
- [ ] Golden vector B (MFA) passes byte-exact.
- [ ] Key is `SnowflakeTokenCache.v2.<TokenType>.<lowercase-hex>` (four segments).
- [ ] `token_type` never appears inside `keyData`.
- [ ] OAuth `keyData` has exactly `idp`, `role`, `snowflake`, `username` (sorted);
      MFA/ID `keyData` has exactly `snowflake`, `username` (sorted).
- [ ] `idp` is the full token-endpoint URL, not the hostname only.
- [ ] `normalize_url` lowercases, preserves an explicit `:443`, strips userinfo
      from the authority only, and strips trailing slashes.
- [ ] `normalize_identifier` returns quoted values verbatim and lowercases
      unquoted values.
- [ ] Both backends store the built key verbatim; hashing happens exactly once.
- [ ] File fallback uses `credential_cache_v2.json`, `0700` dir / `0600` file,
      fd-based validation, and locking.
- [ ] Dimension-isolation tests pass.
