# Token cache: platform storage and the Linux file fallback

> Decision recorded: SNOW-3663604 · status: **Accepted risk**

## Summary

Token caching is **enabled by default on every platform** the driver supports.
On macOS the system Keychain is used; on Windows the Credential Manager; on
Linux the D-Bus Secret Service or kernel keyutils when available. On Linux
hosts where no system-backed store is reachable at runtime,
[`KeyringTokenCache::new`](../../sf_core/src/token_cache/keyring_cache.rs#L17-L21)
silently falls back to `FileTokenCache`: a `credential_cache_v2.json` file
inside an owner-only `0700` directory, with the file itself at `0600`.

## What the file may hold

The following `TokenType` values may be persisted to that file:

| Token type | Auth flow |
|---|---|
| `IdToken` | External-browser SSO |
| `MfaToken` | MFA |
| `OAuthAccessToken` | OAuth |
| `OAuthRefreshToken` | OAuth |
| `DpopBundledAccessToken` | DPoP-bound OAuth |

## Why caching is on by default

Disabling token caching entirely forces re-authentication on every new
connection. For MFA and SSO flows this means an interactive prompt or a
browser round-trip on every connection, which is unacceptable for applications
that open many short-lived connections. The default-on posture trades security
posture (resting tokens on disk) against usability for the common case.

## Protection in place

* **Directory**: created at mode `0700` (`FileTokenCache#create_subdir`) —
  accessible only by the owning user.
* **File**: created via `create_new_nofollow` at `0600`
  (`FileTokenCache#create_exclusive`) — readable and writable only by the
  owning user.
* **Read-time validation**: the open file descriptor is fstat-validated before
  any token is read (`FileTokenCache#validate_file_fd`, INV-2) — ownership and
  mode are checked on the fd, not on a preceding `stat` call, so TOCTOU is
  avoided.
* **Windows caveat**: the fstat/ownership check is `#[cfg(unix)]`-only; on
  Windows the file is created with plain `std::fs` calls and no mode
  enforcement (see the fail-open entry in the threat model's
  "Fail-open / fail-closed reachability" table).

## When the protection is insufficient (stops applying)

* The driver runs under a **shared UID** (e.g., a container where multiple
  tenants share the same OS user). DAC does not prevent a co-tenant with the
  same UID from reading the `0600` file.
* The host has **no system keystore** and is a **multi-tenant Linux machine**
  where lower-privileged users cannot be assumed to stay out of the owning
  user's home directory by other means.
* The host runs in a **container** where no D-Bus socket is available *and*
  the container image mounts home directories across tenants.

## How to opt out

Set the connection parameter:

```
CLIENT_STORE_TEMPORARY_CREDENTIALS = false
```

This disables token caching entirely. The driver will not persist any token to
disk or to the OS keystore. Every connection will require a fresh
authentication round-trip.

## References

* Implementation: `sf_core/src/token_cache/keyring_cache.rs` —
  [`KeyringTokenCache::new` (L17–21)](https://github.com/snowflakedb/drivers/blob/main/sf_core/src/token_cache/keyring_cache.rs#L17-L21)
* Fallback: `sf_core/src/token_cache/file_cache.rs`
* Threat-model invariant: `.security/threat-model.md` INV-47
* Jira: [SNOW-3663604](https://snowflakecomputing.atlassian.net/browse/SNOW-3663604)
