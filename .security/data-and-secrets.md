> Derived-from: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993 · generated 2026-08-19 · sources: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993
> Regenerate when: a new credential-acquisition mechanism is added, or a CI secret-delivery path changes

## What sensitive data this driver holds

User-supplied password, PAT, private key and passphrase, OAuth client secret,
and MFA passcode; server-issued session and master tokens, plus cached
`IdToken`/`MfaToken`/`OAuthAccessToken`/`OAuthRefreshToken`
(`sf_core/src/token_cache/mod.rs#TokenType`); infrastructure credentials — proxy
password, cloud stage credentials, presigned URLs, and `query_stage_master_key`
(`sf_core/src/file_manager/types.rs:390`), which unwraps the per-file
client-side-encryption keys (`sf_core/src/file_manager/encryption.rs:117,226`);
and all customer result data and query text in transit.

All of the above are held as `Sensitive<T>` / `SensitiveString`
(`sf_core/src/sensitive.rs`) where the driver's own types carry them —
zeroized on drop, redacted in `Debug`/`Display`. The wrapper's `Serialize`
implementation delegates to the inner type rather than redacting it; see
`threat-model.md` INV-8 for the acceptance this rests on.

## Where secrets come from at runtime

### How the driver itself receives credentials

Six mechanisms, not one:

1. **Connection parameters from the calling application** —
   `sf_core/src/config/connection_config.rs#build_auth_config`. The primary
   path: whatever the embedding DBAPI/JDBC/ODBC connection string or config
   object supplies.
2. **Config files on disk** — TOML (`sf_core/src/config/toml_loader.rs#load_toml_file`)
   and INI (`sf_core/src/config/ini_loader.rs#load_ini_files`, for
   `sf.odbc.ini`). Both gated by the permission check described in
   `trust-boundaries.md`.
3. **OS keystore with file-based fallback** — see `trust-boundaries.md` for the
   full mechanism.
4. **External-browser SSO** — `sf_core/src/config/connection_config.rs#AuthConfig::ExternalBrowser`.
   The driver opens a local redirect-URI listener; the user's browser completes
   the IdP flow; the resulting token is cached via mechanism 3.
5. **Workload Identity Federation** —
   `sf_core/src/rest/snowflake/workload_identity/mod.rs#create_attestation`,
   dispatching to per-cloud-provider modules. See `trust-boundaries.md` for the
   host gate that runs first.
6. **SPCS auto-injected environment** — `SNOWFLAKE_RUNNING_INSIDE_SPCS`, and,
   only when that flag is set, `SNOWFLAKE_ACCOUNT`/`SNOWFLAKE_HOST`/
   `SNOWFLAKE_DATABASE`/`SNOWFLAKE_SCHEMA`
   (`sf_core/src/env_vars.rs:11-18`). Inside Snowpark Container Services, the
   platform itself injects connection identity into the container's
   environment, not the calling application.

### How the project's own CI/build receives secrets

Three independent systems, not one:

1. **GitHub Actions** — `.github/secrets/parameters_<cloud>[_local].json.gpg`,
   decrypted by `scripts/decode_secrets.sh` using a `PARAMETERS_SECRET` GPG
   passphrase expected pre-set as a CI environment variable. A local-dev-only
   fallback reads it from 1Password (`op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password`)
   when neither `GITHUB_ACTIONS` nor `BUILD_NUMBER` is set. A fork-contributor
   escape hatch (`PARAMETERS_JSON_<CLOUD>`, plaintext repo secret) bypasses GPG
   entirely for forks that cannot hold `PARAMETERS_SECRET`.
2. **Buildkite** — `PARAMETERS_SECRET` is referenced in
   `.buildkite/pipelines/ci-tests/pipeline.yml:18,370`, but the actual source
   is a Vault-backed Buildkite plugin mapping a Vault path to that environment
   variable at pipeline start.
3. **Jenkins** — its own Credentials Store, distinct from both GitHub's and
   Vault's native surfaces: AWS access key/secret for internal Docker
   registry/S3 access; HashiCorp Vault `role_id`/`secret_id` fed into an
   internal broker ("SnowIdentity") that decrypts cloud-storage credentials for
   test-account provisioning — a second, independent integration with the same
   Vault instance Buildkite uses; two differently-scoped GitHub App tokens; a
   file credential for perf-metrics upload config; and its own Jenkins
   credential ID feeding the same `decode_secrets.sh`/`PARAMETERS_SECRET`
   mechanism GitHub Actions uses — so that one script is invoked from at least
   two of the three CI systems, each sourcing the passphrase differently.

WIF end-to-end test fixtures
(`ci/wif/parameters/{parameters_wif.json,rsa_wif_aws_azure,rsa_wif_gcp}.gpg`)
are decrypted with the same `PARAMETERS_SECRET` per `ci/test_wif.sh`'s header
comment.

**Confirmed absent:** `.circleci`, `.gitlab-ci.yml`, and `azure-pipelines.yml` do
not exist in this repository.
