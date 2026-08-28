> Derived-from: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993 · generated 2026-08-19 · sources: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993
> Regenerate when: a connection-parameter validation path changes, a new bypass parameter is added, or a mechanism described below is refactored

## What supplies input, and what validates it

This is a client library — its inputs are the calling application's connection
parameters and the Snowflake service's own HTTP responses, not network requests
from the outside world.

**The calling application, via connection parameters.** Entry points:
`sf_core/src/apis/database_driver_v1/connection.rs#connection_set_option` /
`#connection_set_options`. Validated by `sf_core/src/apis/database_driver_v1/validation.rs#resolve_options`,
which resolves each key against the canonical parameter registry
(`sf_params_spec`), rejects unknown/conflicting keys, and type-checks stringly-typed
values (ODBC/JDBC connection strings are inherently string-only) against the
registry's declared `ValueType`. `validation.rs#validate_connection_seed_write`
additionally rejects writes wrong-scoped for the connection's lifecycle state.
Confirmed production call sites: `nodejs_bridge/src/connection.rs:36`,
`odbc/src/api/connection.rs:531,551`.

**The Snowflake service, via HTTP responses.** Validated by (a) TLS peer
authentication of the responding server before any response bytes are trusted
(`sf_core/src/tls/client.rs`, on by default — see the TLS section below for the
caller-configurable exception), and (b) strict serde-typed deserialization of
the response body against fixed struct shapes (e.g.
`sf_core/src/rest/snowflake/query_response.rs`,
`sf_core/src/rest/snowflake/auth.rs`) — malformed or unexpected shapes fail to
parse rather than being accepted.

**An unvalidated path exists in the same source struct, but is not reachable
from any production surface.** `sf_core/src/apis/database_driver_v1/connection.rs#Connection::set_option`
(doc comment: "Convenience setter for tests and direct call sites") inserts
directly into `connection_seed` with no call to `resolve_options`. Every bridge
crate (`python_bridge`, `nodejs_bridge`, `jdbc_bridge`, `odbc`) and all of
`sf_core/src` outside `tests/` were checked for `.set_option(` call sites — the
only ones found are in test files. Bridges only ever reach connections through
`DatabaseDriverV1`'s handle-based async API, which does go through validation.

**One control is enforced downstream of this repository entirely.** DPoP
proof-of-possession for the OAuth Authorization Code login flow: the driver
constructs a DPoP JWK/proof and attaches it to the login request
(`sf_core/src/rest/snowflake/mod.rs:698-704`), but the driver does not verify the
proof itself — per the code comment, the server validates it statelessly
against the thumbprint (`jkt`) already embedded in the access token (RFC 9449).
This is a standard-mandated check the Snowflake server enforces on every
request using this login method — a compromised or malicious driver build
cannot forge a request that passes it without possessing the private key,
regardless of what the driver claims in the request body.

## What is trusted internally: mechanisms

**Local filesystem — config/credential-file permission gate, Unix.**
`sf_core/src/config/toml_loader.rs#check_file_permissions` — the whole function
body is `#[cfg(unix)]`-gated. A file writable by group or other
(`mode & 0o022 != 0`) is rejected before parsing; a file readable by group or
other is a warn-only log, suppressible via
`SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE`. The gate itself is
skippable only via an explicit `unsafe_skip_file_permissions_check` /
`unsafe_skip_config_file_permissions_check` flag, and doing so logs
(`tracing::info!`) — never silent. Called from `toml_loader.rs#load_toml_file`,
`sf_core/src/config/private_key.rs:143`, `sf_core/src/config/ini_loader.rs:92`
(the same gate covers config and credential files, not just `connections.toml`).
The check is stat-then-open, not fd-decided: it stats the path
(`check_file_permissions`), then `load_toml_file` opens it separately — the
token cache (below) avoids this pattern by deciding on the open file descriptor
instead.

**OS keystore with file-based fallback.** Primary:
`sf_core/src/token_cache/keyring_cache.rs` (service `snowflake_credential_cache`).
Fallback: `sf_core/src/token_cache/file_cache.rs` — directory created at mode
`0o700` (`#create_subdir`), lock file at `0o600` (`FileLock#open_lock_file`),
new cache files created via `crate::fs_lock::create_new_nofollow` with mode
`0o600` (`#create_exclusive`). Reads validate the open file descriptor directly
— `#validate_file_fd` (fstat + uid check, remediates a too-open mode) and
`#open_existing` (`O_NOFOLLOW`, `ELOOP` on a symlink target) — deciding on the
fd rather than a preceding stat, avoiding the TOCTOU shape the config-file gate
above has. `#validate_file_fd`/`#open_existing` are, like the config gate,
Unix-only.

**TLS verification, and its runtime bypass.** Defaults are secure:
`verify_hostname: true, verify_certificates: true`
(`sf_core/src/tls/config.rs#from_settings`). All three of
`tls_skip_verify`, `verify_certificates`, and `verify_hostname` are ordinary
runtime connection parameters resolved through the same `Settings`-map path as
every other connection parameter — any calling application can set them to
disable certificate, hostname, and CRL-revocation checking on a real
connection. `tls_skip_verify` short-circuits both the certificate and hostname
flags regardless of their individual values. The resulting client is built with
`.danger_accept_invalid_certs(true).danger_accept_invalid_hostnames(true)`
(`sf_core/src/tls/client.rs:52-65`). This is not compile-time or build-mode
gated — no `cfg(feature = ...)` exists around this logic, and the `fips` Cargo
feature changes only the linked crypto backend, not this behavior
(`.github/workflows/test-rust-core.yml:417-419`). The runtime warns
(`tracing::warn!`, `sf_core/src/tls/config.rs:312-316`,
`sf_core/src/tls/client.rs:53,98`) but does not refuse. The same helper builds
the TLS client used for cloud storage transfers
(`sf_core/src/file_manager/azure_transfer.rs:1107`,
`sf_core/src/file_manager/gcs_transfer.rs:1142`,
`sf_core/src/tls/aws_http_client.rs:56-57` — the last also sets
`redirect::Policy::none()`), so the same caller-supplied bypass reaches
S3/GCS/Azure traffic, not only the account-host REST client.
A separate, narrowly-cfg-gated `TlsConfig::insecure()` constructor
(`sf_core/src/tls/config.rs:291-300`) is genuinely test-only — reachable in
production code only from `sf_core/src/config/rest_parameters.rs:206`, itself
inside
`#[cfg(any(test, feature = "test-utils"))]` — and is a different, narrower
mechanism from the three connection parameters above.

**Presigned and chunk-download URLs are provenance-restricted, not
host-validated.** `Chunk.url` (`sf_core/src/rest/snowflake/query_response.rs#Chunk`)
is populated exclusively by serde deserialization of the authenticated
query-response body — there is no other code path that can set it. The driver
does not independently check the host or scheme of a chunk or presigned URL
before fetching it (`sf_core/src/chunks/mod.rs:362,407,419-422`); trust in the
URL's destination rests entirely on the authenticity of the response it arrived
in, which is the TLS/hostname verification described above.

**Proxy.** `sf_core/src/tls/config.rs#ProxyConfig`. Environment-variable
proxies are ignored unless `use_proxy_env` is explicitly set
(`sf_params_spec/src/lib.rs:219-221`, default `false`); explicit connection
config overrides env. Storage transfers pass the proxy config explicitly
(`sf_core/src/file_manager/azure_transfer.rs:1109-1114`,
`sf_core/src/file_manager/gcs_transfer.rs:1145-1149`), so they honor the
same setting rather than silently bypassing it. Proxy password is held as
`SensitiveString`.

**CRL (certificate revocation).** `sf_core/src/crl/config.rs#CertRevocationCheckMode`
defaults to `Disabled`. When `Enabled` or `Advisory`, download is capped at
`DEFAULT_CRL_DOWNLOAD_MAX_SIZE_BYTES` (20 MiB, `sf_core/src/crl/config.rs:7`).
A confirmed-revoked certificate is rejected regardless of mode, including
`Advisory` — the check that lets `Advisory` proceed on an unresolved CRL check
only fires when no chain came back revoked
(`sf_core/src/tls/crl_verifier.rs#verify_server_cert`). CRL signatures are
verified against the issuer before use
(`sf_core/src/tls/x509_utils.rs#verify_crl_signature`, called from
`sf_core/src/crl/cache.rs:742,749`).

**OAuth loopback callback listener.** Always binds a loopback interface
regardless of configured host — `sf_core/src/rest/snowflake/oauth/loopback_server.rs#bind`
(`127.0.0.1` default; `::1` only for an explicit IPv6 redirect hint).
The callback is authenticated by matching the returned `state` against the
generated value, constant-time
(`sf_core/src/rest/snowflake/oauth/authorization_code.rs:600-601`).

**Native Okta and OAuth IdP host handling — two different postures for two
different flows.** Native Okta origin-pins the IdP URL and fails closed on
mismatch: `sf_core/src/rest/snowflake/native_okta.rs#validate_idp_urls`,
`#url_origin_matches`. OAuth's `authorization_url`/`token_url` overrides go
through a bare `url::Url::parse` with no allowlist
(`sf_core/src/rest/snowflake/oauth/authorization_code.rs#resolve_authorize_url`,
`#resolve_token_url`) — this asymmetry is recorded in `threat-model.md`.
The external-browser SSO flow's `sso_url` is server-response-supplied (fetched
from the account host, `POST /session/authenticator-request`) and is validated
only for launch-safety before being opened —
`sf_core/src/rest/snowflake/browser.rs#validate_browser_url` requires `https`,
rejects shell/argv metacharacters and control bytes, and never routes through a
shell interpreter (WSL uses a standalone argv element, not `cmd` —
`sf_core/src/rest/snowflake/browser.rs#wsl_launch_candidates`, see SNOW-3649282).
This check establishes
launch safety, not host identity: it does not confirm the `sso_url`'s host
matches an expected IdP.

**Workload Identity Federation host gate.**
`sf_core/src/rest/snowflake/workload_identity/mod.rs#ensure_allowed_host` runs
before any provider-specific attestation is fetched
(`sf_core/src/rest/snowflake/mod.rs:726`), checked against a suffix-anchored
allowlist (`sf_core/src/rest/snowflake/workload_identity/host_allowlist.rs#is_snowflake_host_for_workload_identity`,
suffixes `snowflakecomputing.com`/`.cn`/`.mil`,
`sf_core/src/rest/snowflake/workload_identity/host_allowlist.rs:21-25`). Fails
closed: an unparseable URL or a host with no match is rejected identically to
an explicitly disallowed host. The `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES`
environment variable is additive-only — read only from the process
environment, never from connection configuration — and cannot narrow or
disable the check (`sf_core/src/rest/snowflake/workload_identity/host_allowlist.rs`
module doc). AWS and GCP attestation audience is pinned to
`snowflakecomputing.com`
(`sf_core/src/rest/snowflake/workload_identity/aws.rs:38`,
`sf_core/src/rest/snowflake/workload_identity/gcp.rs:27`).

## What is enforced outside this repository — plain facts, not this repo's controls

**The public mirror.** `snowflakedb/drivers` receives commits via Copybara on a
daily schedule (`ci/mirroring/copy.bara.sky` — publish policy dated
2026-06-18). Decrypting the GPG-encrypted `.github/secrets/*.gpg` bundle
requires `PARAMETERS_SECRET`, which the mirror never receives — GitHub Actions
secrets, Vault, Jenkins, and 1Password are all internal-network-only — so the
bundle is inert on the public side without that passphrase.

**A TLS-terminating corporate proxy can see all driver traffic.** Whether that
is acceptable, and supplying the proxy's CA trust, is the customer's own policy
decision (`NOMIRROR/Security/security_guidelines.md:652-653`). This driver
cannot detect or prevent a proxy from terminating and re-establishing TLS.

**Instance-metadata service integrity.** The integrity of the cloud instance-
metadata service and the process environment that points at it is the cloud
provider's and the OS's responsibility, not this driver's
(`NOMIRROR/Security/security_guidelines.md:897-899`). WIF trusts the
platform-provided identity; it verifies the *destination* the attestation is
sent to (see the host gate above), not the metadata service itself.
