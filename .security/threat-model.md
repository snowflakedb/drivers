> Derived-from: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993 · generated 2026-08-19 · sources: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993
> Regenerate when: an invariant changes status, a new bypass or accepted-risk decision is recorded, or the systems-and-trust-boundaries table gains or loses a system

## The system

The Snowflake universal driver: a client-side shared library embedded in
customer applications, not a service. A Rust core (`sf_core/`) exposes a C-ABI
and protobuf surface consumed by seven language wrappers (`docs/architecture.md:61,106-107`).

- **Maintained by** `@snowflake-eng/snow-drivers-warsaw`, code owner of every
  file (`.github/CODEOWNERS:17`).
- **Driven by** the host application's own calls to the public API. There is
  no scheduler, listener, or ambient trigger.
- **Changed by** anyone able to merge to `main` or a `release/*` branch. Merge
  is effectively publication: it puts the change on the public mirror
  `snowflakedb/drivers` on the next daily Copybara run
  (`NOMIRROR/security-signoff.md:3-7`; `ci/mirroring/copy.bara.sky:6-7,12-13`).
  Two gates: GitHub-native code-owner approval, and the `security-signoff`
  commit status, which requires an `APPROVED` review from a security partner
  other than the author on the PR's current head commit, and only when the
  `security-signoff-required` label is present
  (`NOMIRROR/security-signoff.md:15-24,31-36`). The roster is read from the
  PR's base branch, so a PR cannot edit the list governing its own sign-off
  (`security-signoff.md:38-42`).
- **Runs with** the host application's full ambient authority — filesystem,
  network, environment, and process memory. A driver bug is the embedding
  application's bug (`NOMIRROR/Security/security_guidelines.md:276-282`).
- **Blast radius of a compromised release**: this library executes inside
  every customer application that embeds it, and there are no forced upgrades
  — old versions remain in the wild indefinitely
  (`NOMIRROR/Security/security_guidelines.md:269-274`). A compromised release
  is arbitrary code execution in customer processes, at their own update
  cadence.
- **Not a confinement boundary.** The driver runs with the host application's
  authority and is not trusted to confine it — it can behave least-privilege,
  but cannot enforce anything against the app embedding it
  (`NOMIRROR/Security/security_guidelines.md:276-282`).
- **A local co-tenant on the same host is in scope.** Much of what this
  document describes — the Unix file-permission gate, the loopback-only OAuth
  listener with `state` authentication, the `0600` token caches — exists
  specifically to resist a lower-privileged local principal, not only a remote
  network attacker.
- **Reads and treats as trustworthy:** the host application's API arguments
  and process environment; `connections.toml`/`config.toml` after the
  permission gate; the OS keystore; stage credentials and presigned URLs
  issued by the Snowflake backend over the authenticated session.
- **Explicitly not trusted:** server-supplied download filenames, server-
  supplied hostnames, and cert-embedded CRL distribution-point URLs.
- **Holds:** user-supplied password / PAT / private key and passphrase / OAuth
  client secret / MFA passcode; server-issued session and master tokens, plus
  cached `IdToken`/`MfaToken`/`OAuthAccessToken`/`OAuthRefreshToken`
  (`sf_core/src/token_cache/mod.rs#TokenType`); infrastructure credentials —
  proxy password, cloud stage credentials, presigned URLs, and
  `query_stage_master_key`, which unwraps the per-file client-side-encryption
  keys (`sf_core/src/file_manager/types.rs#EncryptionMaterial`,
  `sf_core/src/file_manager/encryption.rs#build_encryptor`,
  `#decrypt_ciphertext_to_writer`); and all customer result
  data and query text in transit. `Sensitive<T>` (`sf_core/src/sensitive.rs`)
  zeroizes on drop and redacts `Debug`/`Display` output, but its
  `Serialize` implementation delegates to the inner type — a full struct that
  reaches a log or telemetry sink unredacted would expose the value (see
  Invariants, INV-8).

## Systems and trust boundaries

| System | Role here | Crosses in | Crosses out | Trusted for |
|---|---|---|---|---|
| Local filesystem | Config/key storage, result spool, log sink, on-disk token-cache fallback | `connections.toml`/`config.toml`, private key files, server-supplied download names | Written results, logs, cached tokens | Integrity of config/credential files before parsing — that a file the driver reads has not been made writable by a lower-trust local principal |
| OS keystore + file fallback | Primary secret store for session/OAuth tokens; falls back to an owner-only file when no keystore exists | Tokens to store | Tokens on retrieval | Confidentiality of cached tokens at rest, within the keystore's own access scope |
| Snowflake backend REST API | Primary control channel: login, session, query, results | Credentials, query text/params, connection params | Session/master tokens, results, server config | Endpoint authenticity (TLS chain + hostname); honesty of the session parameters and IdP endpoints it returns |
| Cloud provider blob storage (S3/GCS/Azure stage) | PUT/GET file transfer to the account's cloud stage | Stage credentials/presigned URLs from Snowflake, file bytes (PUT) | Uploaded bytes, downloaded bytes (GET) | Endpoint authenticity only — content confidentiality is provided by client-side encryption, not by this boundary |
| Proxy | Customer/enterprise HTTP(S) proxy all driver egress may transit | All outbound requests | Proxied responses | Not being silently interposed via environment variables when the caller has not opted in |
| Third-party revocation egress (CRL) | Fetches CRLs from CA-operated distribution-point hosts named in the presented cert | CRL distribution-point URL (cert-embedded, untrusted) | CRL bytes, cached CRL (disk) | Integrity of revocation data; availability of the CA's CRL endpoint is explicitly not trusted |
| Identity providers (Snowflake IdP, Native Okta, OAuth) + OAuth loopback listener | Authenticates the user | Authorize URLs, callback params, tokens | Token/authorize requests to IdP hosts | IdP host authenticity (varies by flow — see `trust-boundaries.md`); authenticity of the local callback caller |
| Web browser + system shell | Opens the IdP-supplied URL in the user's system browser | IdP authorize/SSO URL (untrusted) | — (fire-and-forget process spawn) | Nothing — this is a command-execution boundary the URL must be safe to cross, not a source of any guarantee |
| Telemetry egress (Snowflake account host) | In-band, session-gated operational telemetry | — | Telemetry payloads to `/telemetry/send` | Receiving only approved operational metadata |
| Cloud instance-metadata / WIF | Mints a cloud-issued identity token, forwarded to Snowflake for verification | — (ambient IMDS/metadata credentials) | SigV4-signed `GetCallerIdentity` (AWS) or OIDC token request (Azure/GCP) | Attestation-target host authenticity and audience binding |
| CI credential delivery (GitHub Actions, Vault-backed Buildkite plugin, Jenkins Credentials Store + SnowIdentity) | Three independent secret stores supplying build/test credentials | Build/test requests | Decrypted secrets into the CI runner | Confidentiality of `PARAMETERS_SECRET` and CI database credentials |
| Public mirror `snowflakedb/drivers` | Copybara-mirrored public copy of this repository | Copybara-pushed commits | Inbound PR-import metadata | Receiving only non-secret content |
| Supply chain (crates.io, PyPI, npm, Maven, NuGet) | Third-party dependencies | Package downloads | — | Byte-identity of pinned dependencies only — not their behavior |

## Invariants

| ID | Invariant | Control | Status |
|---|---|---|---|
| INV-1 | Download destination path is contained | `sf_core/src/file_manager/mod.rs#safe_download_file_name`, `#resolve_validated_output_path` | Holding |
| INV-2 | Unix token-cache file descriptor is fstat-validated before use | `sf_core/src/token_cache/file_cache.rs#validate_file_fd` | Holding |
| INV-3 | Unix config/credential file rejected before parse if group/other-writable | `sf_core/src/config/toml_loader.rs#check_file_permissions`, called from `#load_toml_file`, `private_key.rs:143`, `ini_loader.rs:92` | Holding |
| INV-4 | Permission-gate skip requires an explicit flag, and the skip is logged | `sf_core/src/config/toml_loader.rs#check_file_permissions` | Holding |
| INV-5 | OAuth token cache key is scoped to IdP + account + user + role | `sf_core/src/token_cache/mod.rs#serialize_cache_key` | Holding |
| INV-6 | MFA/ID-token and OAuth cache keys cannot cross-read each other | `sf_core/src/token_cache/mod.rs#build_cache_key` | Holding |
| INV-7 | `Sensitive<T>` is zeroized on drop and redacted in Debug/Display | `sf_core/src/sensitive.rs#Sensitive` | Holding |
| INV-8 | `Sensitive<T>`'s `Serialize` impl exposes the credential value unredacted | `sf_core/src/sensitive.rs#Sensitive` | Accepted risk (`docs/logging/logging-guidelines.md` and reviewer discipline — assumes no full struct containing a `Sensitive<T>` field reaches a log or telemetry sink; stops applying if a serialized struct is logged directly; relies on code-owner review and the logging guidelines) |
| INV-9 | TLS chain and hostname verification are on by default | `sf_core/src/tls/config.rs#TlsConfig::from_settings` | Holding |
| INV-10 | Disabling TLS verification requires an explicit connection parameter, and the runtime warns when it is set | `sf_core/src/tls/config.rs#TlsConfig::from_settings`, `sf_core/src/tls/client.rs:53,98` | Accepted risk (caller-supplied connection parameters — assumes only the embedding application sets its own connection parameters; stops applying if a lower-privileged local principal can influence connection configuration; relies on the embedding application's own config management, and on INV-3 keeping an unauthorized local principal out of `connections.toml`) |
| INV-11 | A confirmed-revoked certificate (end-entity or any chain link) is rejected regardless of check mode, including Advisory | `sf_core/src/tls/crl_verifier.rs#verify_server_cert` | Holding |
| INV-12 | CRL signature is verified against the issuer before use | `sf_core/src/tls/x509_utils.rs#verify_crl_signature`, called from `sf_core/src/crl/cache.rs:742,749` | Holding |
| INV-13 | Advisory mode allows the connection when the CRL check could not reach a definite non-revoked verdict (no revoked chain, but validation otherwise failed or was inconclusive) | `sf_core/src/tls/crl_verifier.rs#verify_server_cert` | Accepted risk (`sf_core/src/crl/config.rs#CertRevocationCheckMode` — assumes customers who need fail-closed behavior select `Enabled`; stops applying if a customer with an `Advisory` requirement needed fail-closed semantics; relies on the customer's own mode selection) |
| INV-14 | Revocation checking is off by default | `sf_core/src/crl/config.rs#CertRevocationCheckMode` (`#[default] Disabled`) | Accepted risk (`NOMIRROR/Security/security_guidelines.md:186-191,269-274` — a revocation fetch leaks connection metadata to the CA; assumes customers requiring revocation checking explicitly enable `crl_check_mode`; stops applying if that metadata leak becomes unacceptable to a customer who has enabled it anyway; relies on the customer's own mode selection) |
| INV-15 | CRL download is capped at a 20 MiB default | `sf_core/src/crl/config.rs#DEFAULT_CRL_DOWNLOAD_MAX_SIZE_BYTES`, `#from_settings` | Holding |
| INV-16 | Environment-variable proxies are ignored unless `use_proxy_env` is set; proxy password is `SensitiveString`; storage transfers pass proxy config explicitly | `sf_core/src/tls/config.rs#ProxyConfig`, `sf_params_spec/src/lib.rs#USE_PROXY_ENV`, `azure_transfer.rs#create_azure_client`, `gcs_transfer.rs#create_gcs_client` | Holding |
| INV-17 | OAuth callback `state` is matched constant-time | `sf_core/src/rest/snowflake/oauth/authorization_code.rs#run_interactive_flow` | Holding |
| INV-18 | OAuth loopback listener always binds a loopback address regardless of configured host | `sf_core/src/rest/snowflake/oauth/loopback_server.rs#bind` | Holding |
| INV-19 | Browser-launch URL validation is an unbypassable backstop: https-only, no shell metacharacters | `sf_core/src/rest/snowflake/browser.rs#validate_browser_url`, `#open_url` | Holding |
| INV-20 | The IdP URL is never routed through a shell; WSL uses a standalone argv element | `sf_core/src/rest/snowflake/browser.rs#wsl_launch_candidates` | Holding |
| INV-21 | WIF attestation is sent only to an allowlisted host, checked before any credential fetch, and fails closed | `sf_core/src/rest/snowflake/workload_identity/mod.rs#ensure_allowed_host`, called at `sf_core/src/rest/snowflake/mod.rs#auth_request_data` | Holding |
| INV-22 | The WIF allowlist cannot be narrowed by connection configuration; the environment-variable override is additive-only | `sf_core/src/rest/snowflake/workload_identity/host_allowlist.rs#ALLOWED_SUFFIXES` | Holding |
| INV-23 | AWS and GCP WIF attestation audience is pinned to `snowflakecomputing.com` | `sf_core/src/rest/snowflake/workload_identity/aws.rs#SNOWFLAKE_AUDIENCE`, `gcp.rs#SNOWFLAKE_AUDIENCE` | Holding |
| INV-24 | Every bypass path (permission-gate skip, TLS verification disable, WIF allowlist extension) logs on the bypass path | `toml_loader.rs#check_file_permissions`, `tls/config.rs#TlsConfig::from_settings`, `host_allowlist.rs` | Holding |
| INV-25 | The GPG-encrypted end-to-end credential test bundle is published to the public mirror | `ci/mirroring/copy.bara.sky:19,25-30` | Accepted risk (`ci/mirroring/copy.bara.sky:19` — dated policy decision, 2026-06-18; assumes `PARAMETERS_SECRET` never reaches the public mirror; stops applying if that passphrase is ever exposed on the public side; relies on GitHub Actions/Vault/Jenkins/1Password all being internal-network-only) |
| INV-26 | A keyword net flags mirror-bound content that may disclose internal-only information | `NOMIRROR/security-signoff.md` | Accepted risk (`NOMIRROR/security-signoff.md` — assumes the net's keyword coverage matches what is actually published to the mirror; stops applying if the publish policy changes which directories are published without the net being updated; relies on the net's own maintenance) |
| INV-27 | `Cargo.lock` pins the exact bytes of every Rust dependency | `Cargo.lock` | Holding (narrow: verifies dependency bytes, not dependency behavior) |
| INV-28 | Staged data is encrypted client-side (CSE) before upload only when the stage response includes encryption material; SSE stages (no encryption material) upload unencrypted client-side and rely on server-side encryption | `sf_core/src/file_manager/mod.rs#preprocess_file_before_upload` (conditional gate), `sf_core/src/file_manager/encryption.rs#build_encryptor` (CSE encryptor), `sf_core/src/rest/snowflake/query_response.rs#to_file_upload_data` (SSE omits `encryption_material`) | Holding |
| INV-32 | Token-cache fallback from OS keystore to file cache | `sf_core/src/token_cache/keyring_cache.rs#new` (fallback decision), `sf_core/src/token_cache/file_cache.rs` (fallback target) | Holding |
| INV-33 | PKCE (S256) is applied to the OAuth authorization-code flow unless explicitly disabled | `sf_core/src/rest/snowflake/oauth/authorization_code.rs#run_interactive_flow`; default `false` pinned by `sf_core/src/config/rest_parameters.rs#test_oauth_disable_pkce_defaults_false_when_setting_omitted_with_real_idp_params` | Holding |
| INV-36 | CI skill-eval subprocess environment is allowlisted | `ci/skill_eval_runner.py#build_claude_env` | Holding |
| INV-37 | Native Okta IdP URLs are origin-pinned and fail closed on mismatch | `sf_core/src/rest/snowflake/native_okta.rs#validate_idp_urls`, `#url_origin_matches` | Holding |
| INV-39 | OAuth response fields `tokenUrl`/`ssoUrl` are deserialized but never acted on | `sf_core/src/rest/snowflake/auth.rs#AuthResponseMain` | Holding |
| INV-41 | Unix config file readable by group/other logs a warning, suppressible via an explicit flag | `sf_core/src/config/toml_loader.rs#check_file_permissions` | Holding |
| INV-42 | File token-cache directory and files are created with restrictive Unix permissions (0700/0600), validated on the open file descriptor | `sf_core/src/token_cache/file_cache.rs#create_subdir`, `#open_lock_file`, `#create_exclusive` | Holding |
| INV-43 | The permission-gate opt-out exists, and is disclosed | `sf_core/src/config/toml_loader.rs#check_file_permissions`; `NOMIRROR/Security/security_guidelines.md:99,111,438,464,551-552` | Accepted risk (`NOMIRROR/Security/security_guidelines.md:99,111,438,464,551-552` — disclosed five times in the driver's own security guidance; assumes the embedding application deliberately chose the opt-out and understands the consequence; stops applying if the opt-out is set without the caller's knowledge; relies on the opt-out itself being logged, INV-24) |
| INV-46 | Chunk-download and presigned stage-transfer URLs are sourced exclusively from the authenticated Snowflake REST response, never from any other input | `sf_core/src/rest/snowflake/query_response.rs#Chunk` | Holding (downstream of INV-9/INV-10: if TLS/hostname verification is bypassed, the response's own authenticity — and therefore this guarantee — is bypassed with it) |
| INV-44 | Instance-metadata service integrity is the cloud provider's and OS's responsibility, not this driver's | `NOMIRROR/Security/security_guidelines.md:897-899` | Architectural risk (`NOMIRROR/Security/security_guidelines.md:897-899` — assumes the cloud provider's link-local metadata service and the OS's process-environment isolation hold; stops applying if either is compromised; relies on the cloud provider and the OS — remediation is outside this repository) |
| INV-45 | A TLS-terminating corporate proxy can observe all driver traffic; accepting that is the customer's own policy decision | `NOMIRROR/Security/security_guidelines.md:652-653` | Architectural risk (`NOMIRROR/Security/security_guidelines.md:652-653` — assumes the customer has made an informed decision about their own proxy and supplied its CA trust; stops applying if a proxy is interposed without the customer's knowledge; relies on the customer's own proxy policy and CA trust management — remediation is outside this repository) |

## Fail-open / fail-closed reachability

| Control | Direction | Reachable in normal operation? |
|---|---|---|
| Telemetry server opt-in (`sf_core/src/apis/database_driver_v1/connection.rs#connection_init_inner`) | Fails open | Yes — absence of the server's `CLIENT_TELEMETRY_ENABLED` parameter enables telemetry |
| CRL `Advisory` mode, undetermined status | Fails open | Yes — a CRL distribution point being slow or unreachable is routine |
| CRL `Enabled` mode, undetermined status | Fails closed | — |
| Confirmed revocation, any mode | Fails closed | — |
| WIF host allowlist | Fails closed | — |
| Browser-launch URL validation | Fails closed | — |
| Native Okta IdP origin pin | Fails closed | — |
| Config permission gate on Windows | Fails open, always | Every Windows deployment — the function returns `Ok(())` having checked nothing, since its body is `#[cfg(unix)]`-gated |
| Token file-cache permission/ownership validation on Windows | Fails open, always | Every Windows deployment — `sf_core/src/token_cache/file_cache.rs#create_subdir`, `#open_existing_cache`, `#create_empty_cache`, `#read_cache`, `#write_cache` each take a `#[cfg(not(unix))]` branch that creates/reads/writes the cache file with plain `std::fs` calls and no fstat/ownership/mode check — the Unix-only enforcement in `#validate_file_fd` (INV-2) has no Windows counterpart |
| `security-signoff` on a merge-queue group | Fails open, argued sound in `NOMIRROR/security-signoff.md` | A PR cannot be queued until its own checks are green; revoking clearance after queueing is untested |
