# FIPS compliance: one crypto module, one flag

Design note for the proposed FIPS 140-3 build of the universal driver. It
records the audit findings (2026-08-31), work already landed, and the target
architecture: one AWS-LC module for security-relevant cryptography and one
Cargo feature selecting its FIPS build. **The current `fips` build does not
meet that target or establish CMVP validation or a whole-driver FIPS claim.**

Status at a glance:

| Item | State |
| --- | --- |
| FIPS as the default build | **Decided: no.** The default links non-FIPS AWS-LC. |
| Crypto selection | **Team decision: one backend, one flag named `fips`.** The feature selects the AWS-LC FIPS build now; its name is not reserved for future compliance completion. Security-sensitive operations outside it are inventoried below. |
| OpenSSL | **Removed from production Rust crypto.** `openssl` remains a dev-dependency for independent fixture/reference tests; `openssl-probe` discovers CA locations without linking OpenSSL. |
| Distribution model | Snowflake-built and consumer-built native artifacts both supported. Snowflake can claim compliance only for artifacts and environments it attests; consumer builds are a supported escape hatch with consumer-owned provenance. |
| Validated module + certificate number | **Module decided: AWS-LC FIPS 3.6.0**, exactly pinned via `aws-lc-fips-sys = "=0.13.17"` and asserted at runtime in the FIPS smoke tests. Certificate / Security Policy applicability and supported deployment environments remain pending compliance confirmation; no public validated claim is established. |
| Phases 1–2 | Done and verified. |
| Phase 3 | F14 closed. Connection, storage and HTTPS CRL clients use the module provider. FIPS builds reject disabled certificate verification; the standard-build opt-out is unchanged. |
| Phases 4–6 | Most direct OpenSSL ports landed, including WIF attestation signing. AWS SDK SigV4 (F7), raw AES-ECB key transport (F4), SBOMs and release work remain. A `fips` build rejects 3DES private-key unwrap; a standard build still performs it outside the module. Phase 5 has the shipped-dependency deny gate and a release-artifact linkage check for ODBC and Node.js. |

## Module version decision

Ship **AWS-LC FIPS 3.6.0**, not 3.1.0. The optional direct
`aws-lc-fips-sys = "=0.13.17"` dependency is enabled by `fips`, so a
Cargo update cannot change the module without a manifest edit.
`tls::fips_tests::linked_module_version_matches_pin` checks the linked
module's `awslc_version_string()` against `EXPECTED_FIPS_MODULE_VERSION`
(`AWS-LC FIPS 3.6.0`); update that constant together with the manifest pin.
The existing `tls::fips_tests` CI filter runs the assertion, and
`CryptoModule` records the runtime version in its one-time identity log.
The `aws-lc-rs <1.18` bound excludes the AWS-LC FIPS 4.x transition and also
holds back rustls 0.23.44+, which requires aws-lc-rs 1.18.

The [3.1.0 dependency route](https://github.com/aws/aws-lc-rs/releases/tag/v1.15.3)
uses `aws-lc-fips-sys 0.13.11`, predating fixes for
[RUSTSEC-2026-0042](https://rustsec.org/advisories/RUSTSEC-2026-0042.html)
(conditional CRL-check bypass) and
[RUSTSEC-2026-0043](https://rustsec.org/advisories/RUSTSEC-2026-0043.html)
(AES-CCM timing). Version 0.13.17 includes those fixes. This is a security
maintenance decision, not proof that either advisory is reachable through
the driver or that the driver is vulnerability-free.

[Certificate #5314](https://csrc.nist.gov/projects/cryptographic-module-validation-program/certificate/5314)
and its [Security Policy](https://csrc.nist.gov/CSRC/media/projects/cryptographic-module-validation-program/documents/security-policies/140sp5314.pdf)
name the static 3.1.0 module and specific Amazon Linux 2023 tested environments.
They do not by themselves establish applicability to 3.6.0 or to Ubuntu,
macOS, or Windows packages. Certificate / Security Policy applicability for
3.6.0 remains pending compliance confirmation before any public "validated"
claim. Neither downgrading nor locally patching 3.1.0 automatically establishes
coverage. Module identity and FIPS-mode checks also do not close the separate
SigV4, key-transport, auxiliary TLS, wrapper-crypto, or release-provenance gaps.
FIPS builds already reject 3DES-encrypted and traditional encrypted PEM keys;
standard builds retain legacy support.

## Bottom line

**No shipped artifact has a complete FIPS claim today.** `fips` selects
AWS-LC FIPS for rustls and the driver's AWS-LC calls, including JWT signing,
AES stage encryption, DPoP, and approved-key decryption. HTTPS CRL downloads
now use the linked module, and FIPS builds reject `verify_certificates=false`
and `tls_skip_verify=true` (F14).
AWS SDK SigV4 signing still uses RustCrypto HMAC/SHA-256 (F7).
A standard build unwraps 3DES private keys with `des` outside AWS-LC; a
`fips` build rejects those keys. The stage key-wrap format uses raw
AES-ECB (F4). Selecting the `fips` feature does not establish CMVP validation,
certificate / Security Policy applicability, or a whole-driver FIPS claim.
F7 SigV4, F4 AES-ECB key wrap, certificate applicability, and per-artifact
SBOMs remain open.

The team decision is one source tree, one cryptographic module, and one flag:

- **without `fips`** — AWS-LC via `aws-lc-sys`. The default. No FIPS claim.
- **with `fips`** — selects AWS-LC FIPS via `aws-lc-fips-sys` today. The target
  remains security-relevant cryptography covered by an identified validation
  and Security Policy, with no reachable non-approved fallback. The feature
  name does not promise that end state.

### Why one flag rather than named profiles

An earlier revision of this note modelled the choice as two or three named
compile-time "profiles" (`standard`, `fips-aws-lc`, `fips-system-openssl`).
That is rejected, for a mechanical reason and a conceptual one.

Mechanically: Cargo features are *additive*. A choice between N named profiles
cannot be expressed in an additive system without something to reject the
combinations, which means `compile_error!` guards — and those guards fire under
`--all-features`, which the repo's own `.pre-commit-config.yaml` (clippy and
check) and `full_put_get_tests.yml` all pass. Making that work needs an
escape-hatch feature whose only purpose is to disarm the guards in exactly the
case you would want them, which is machinery in service of machinery.

Conceptually: there is only one backend. AWS-LC in FIPS mode and AWS-LC not in
FIPS mode are two builds of the same module, not two policies to choose
between. A single additive flag says that directly, needs no guards, and leaves
`--all-features` buildable.

The one combination that genuinely was incoherent — FIPS TLS alongside
statically linked unvalidated OpenSSL — stops existing when OpenSSL does. That
removal is the fix; a `compile_error!` was only ever a way to live with it.

## Why crates outside `sf_core` are affected

All production crypto is in `sf_core`; the wrappers marshal options and
delegate over protobuf/FFI. It is still a single Rust implementation problem.
The wrappers are in scope because they are the final native build roots and
therefore select, package, and attest the compiled artifact.

1. **Feature reachability.** `sf_core` is statically compiled into the ODBC,
   JDBC, Python, and Node.js native libraries. Each final build root must
   forward `fips` to `sf_core` or the flag is unreachable from anything that
   ships. The .NET package loads the standalone native `sf_core` artifact and
   must select the flag in that native build.
2. **Cargo feature unification.** Features are additive, so one dependency edge
   that re-enables a non-approved provider (ring via `reqwest/rustls-tls`, for
   instance) can contaminate a FIPS build. Dependency declarations must opt out
   of implicit defaults; this is what Phase 2's `-no-provider` and
   `oauth2 default-features = false` changes were for.
3. **Release and provenance.** Wheels, jars, ODBC installers, npm prebuilds and
   .NET native packages contain or load native artifacts. Module identity,
   SBOMs, signatures and build provenance attach to those final artifacts, not
   to `sf_core` in isolation.
4. **Wrapper-local findings.** JDBC base64-encodes private-key material into a
   non-zeroizable Java `String`, and the Python wheel's `cryptography`
   dependency may load a second OpenSSL in the process. Both remain in scope
   for a wrapper-level FIPS claim.

## Findings

Severity reflects impact on a credible FIPS claim, not exploitability.

| # | Severity | Finding | Where / status |
| --- | --- | --- | --- |
| F1 | Blocker | The original `fips` feature was unreachable from shipped binding artifacts | Binding manifests — fixed in Phase 1 |
| F2 | **Resolved** (was Blocker) | Shipped artifacts could statically link unvalidated OpenSSL 3.6.2 (`openssl-src 300.6.0+3.6.2`). `openssl` is now a `sf_core` dev-dependency, `vendored-openssl` is deleted from all four manifests, and no release workflow passes it. Verified 2026-09-18: `openssl-src` is absent from both `Cargo.lock` and `python/Cargo.lock.sdist`, and `cargo tree -e normal` finds no OpenSSL in `sf_core`, `odbc`, `jdbc_bridge`, `python_bridge` or `nodejs_bridge` | Resolved by removing OpenSSL outright, not by guarding the combination |
| F3 | Blocker | Direct JWT, private-key AES/PBKDF2, stage AES, DPoP and WIF attestation operations use AWS-LC; AWS SDK SigV4 still signs with RustCrypto outside the selected module. Legacy 3DES unwrapping runs outside the module on standard builds only; `fips` builds reject it. See the inventory below. | `sf_core` and `aws-sigv4` |
| F4 | High | Per-file keys are wrapped with raw AES-ECB, not an approved key-transport construction such as AES-KW/KWP. AWS-LC *can* perform ECB (`aws-lc-rs` exposes `OperatingMode::ECB`), so this is not an implementation blocker — it is a wire-format problem: running an unapproved construction inside a validated module does not make it approved. Shared Snowflake stage format; not unilaterally fixable here | `file_manager/encryption.rs` |
| F5 | High | A transitive reqwest feature can re-enable ring and its implicit provider fallback through feature unification | TLS dependency graph — fixed in Phase 2 |
| F6 | High | AWS Workload Identity previously bypassed shared TLS, proxy, root and CRL policy | `workload_identity/aws.rs` — fixed in Phase 2 |
| F7 | **High**, still open for SigV4 | **The ring-linking finding is resolved in the current normal graph.** The old sources were SDK legacy `rustls` TLS, `sigv4a` and `aws-config/sso`. The manifest now disables all three; disabling `sso` removes IAM Identity Center credentials from the default chain (while workload/explicit credentials remain). `ring` remains in `Cargo.lock` through the dev-only `rcgen` fixture path, not a shipped Rust dependency. **The driver's own AWS WIF `GetCallerIdentity` signer hashes and HMACs through `aws_lc_rs`.** Plain SigV4 still computes HMAC-SHA256/SHA-256 in `aws-sigv4` via RustCrypto for every S3 stage transfer; that SDK signer is not replaced by the local WIF change. | `aws-sigv4` stage transfers |
| F8 | **Resolved** (was Medium) | CSRF `state` and the PKCE code verifier now come from AWS-LC's DRBG via `oauth::random::token_b64url`, and the PKCE challenge digest from AWS-LC rather than RustCrypto. `oauth2` will only build a `PkceCodeChallenge` around a digest it computed itself, so the flow appends `code_challenge`/`code_challenge_method` directly — the same two parameters `set_pkce_challenge` emits. An end-to-end test asserts the challenge is the S256 digest of the verifier presented at the token endpoint, so the bypass cannot drift silently. A DRBG failure fails the login (`OAuthError::RandomGeneration`) rather than falling back. `rand` remains in the graph for retry/refresh jitter and a non-secret FFI handle tag — no security-relevant use | OAuth authorization-code flow |
| F9 | Classified, not eliminated | `md-5` computes Azure Content-MD5 and is also present in the SDK checksum graph; `sha1` is present in `aws-smithy-checksums`. These are checksum paths, not the driver-controlled authentication signer. Separately, `md-5` derives keys for legacy encrypted PEM in **standard builds only**; FIPS builds reject that format (F16). New MD5/SHA-1 usages still require call-site review, not a blanket crate ban. | Azure upload, S3 checksum dependencies, private-key loading |
| F10 | Low | A dead ring-only `rustls-webpki 0.102` dependency was present | Removed in Phase 1 |
| F11 | Partially addressed | `CryptoModule` logs the selected module name, runtime FIPS module version, and provider FIPS state; a FIPS smoke assertion verifies AWS-LC FIPS 3.6.0 against the exact sys-crate pin. The shipped-dependency deny gate rejects ring and OpenSSL crypto libraries. The FIPS artifact lane checks native linkage and TLS status through release-built ODBC and Node.js bindings. Certificate applicability, an SBOM for each artifact, and an attested release policy remain open. ASN.1 certificate and PKCS#8 parsing is not itself a cryptographic module operation (see inventory). | Repository-wide; release evidence remains open |
| F12 | Medium | AWS-LC FIPS has platform/toolchain limits (including previously observed GCC 15 and clang 20 failures with `aws-lc-fips-sys 0.13.12` and Windows ARM64 limitations). Revalidate build support and Security Policy scope for the exact locked `0.13.17` module; no alternate crypto backend covers an unsupported platform. | FIPS build infrastructure |
| F13 | Low | JDBC retains key bytes in a Java `String`; Python may load a second unvalidated OpenSSL | JDBC and Python wrappers |
| F14 | **Resolved** (was Blocker) | Verified connection and cloud-storage clients pass reqwest a config built with the module's provider on the CRL-disabled, enabled, and advisory paths. CRL-disabled default trust is native ∪ webpki roots; custom roots replace that set and extra roots extend it. CRL-enabled and advisory defaults use the native store only; extra roots there are native ∪ extra. HTTPS CRL downloads now use a module-backed config with native ∪ webpki roots, hostname verification and default protocols, without CRL checking to prevent recursion. FIPS builds reject `verify_certificates=false` and `tls_skip_verify=true` during settings parsing and reject hand-built insecure configs at the TLS builders; the reqwest insecure path is compiled only in standard builds. Snowflake `/telemetry/send` uses the verified connection client. IMDS and the local OpenTelemetry exporter are plain HTTP; platform-detection probes retain their fail-closed provider gate. | `tls/client.rs`, `tls/config.rs`, `crl/cache.rs` |
| F15 | Artifact gate added; graph/SBOM caveat remains | With `rustls/fips`, Cargo still resolves and compiles **both** `aws-lc-fips-sys` and `aws-lc-sys` (`0.13.17` and `0.38.0` in the current lockfile). `ci/fips/check_linked_crypto.py` checks that release ODBC and Node.js artifacts link the locked FIPS module, not non-FIPS AWS-LC, ring or unprefixed OpenSSL. Graph presence is not linked-module evidence; the shared FIPS CI matrix now checks artifacts, while per-release SBOMs remain open. | `rustls` feature wiring and release provenance; see Phase 5 |
| F16 | Format decision, not a general MD5 exemption | Two customer-supplied key formats load on a standard build and are rejected on a `fips` build. Traditional encrypted PEM (`Proc-Type: 4,ENCRYPTED`) requires an MD5-based `EVP_BytesToKey` KDF. PBES2 3DES-CBC is unwrapped with RustCrypto `des` on a standard build and rejected, with a conversion error, on a `fips` build. This differs from `md-5` use for nonsecurity upload checksums. The `des` crate stays in the graph for the standard build and for `pkcs8`'s `3des` parse feature. | `crypto/private_key.rs` |

## Shipped Rust crypto inventory and dependency policy

The inventory is the **Cargo graph**, not a binary attestation. At the current
lockfile, `cargo tree --locked --workspace --all-features -e normal,build`
includes the runtime and build dependencies of every workspace root, including
ODBC, JDBC, Python and Node native bindings and both module feature paths.
It omits `[dev-dependencies]`. The independent `deny.toml` uses the equivalent
`[graph] all-features = true, exclude-dev = true`, without restricting target
triples. On PRs, merges and main, `crypto-dependencies.yml` runs the same
`cargo deny --locked check bans` policy twice: against the root workspace
`Cargo.lock`, and against the reduced Python sdist workspace reconstructed from
`python/Cargo.toml.sdist`, `python/Cargo.lock.sdist` and the symlinks listed in
`scripts/python-sdist-cargo-lock.sh`. Both graph checks reject `ring`, the
OpenSSL library/binding family (`openssl`,
`openssl-sys`, `openssl-src`, `openssl-macros`), and `native-tls` as shipped
dependency paths. `cargo-deny` matches **exact crate names**; new differently
named crypto backends and native libraries still require review. No exception
or subtree skip permits a production `ring` or OpenSSL crypto dependency.

`openssl`/`openssl-sys` are reachable only through the `sf_core` dev fixture
dependency, and `ring` only through dev-only `rcgen`. Their lockfile entries
are expected: banning the full lockfile without excluding dev edges would
block the independent test fixtures without protecting shipped artifacts.
`openssl-probe` **is** in the normal graph through `rustls-native-certs`; it
finds OS CA-file locations, not OpenSSL cryptographic code, and is deliberately
not banned. `native-tls` is disallowed because it would restore a second TLS
backend on supported platforms. The two graph checks do not audit non-Rust
dependencies in a Python wheel or exact native libraries linked into a release
package; compare those with the release artifact separately.

`sf_core`'s direct `sha2` dependency is test-only: the OAuth PKCE tests use it
as an independent reference for AWS-LC digests. It remains in the shipped
graph transitively through `aws-sigv4`, `aws-smithy-checksums`, `oauth2` and
`pkcs5`; moving the direct dependency does not remove those implementations
or classify all their operations as approved.

| Crate / boundary | Shipped use and classification | Current disposition |
| --- | --- | --- |
| `aws-lc-rs`, `rustls`, `aws-lc-fips-sys` / `aws-lc-sys` | TLS and direct driver signing, RNG, hashing, AES and PBKDF2 are **security-sensitive**; the feature selects the module. The lockfile currently resolves `aws-lc-rs 1.16.1`, FIPS sys `0.13.17` and non-FIPS sys `0.38.0` when both feature paths are considered. | Allowed, but crate versions and `try_fips_mode()` prove neither the linked binary's module identity nor CMVP applicability. F14 is closed; F15 remains open as described above. |
| `hmac`, `sha2` through `aws-sigv4` and `aws-sdk-s3` | **Security-sensitive:** plain SigV4 HMAC/SHA-256 signs ordinary S3 requests regardless of `sigv4a`; the SDK owns this signer. | Allowed as an explicit external blocker, **not** an approved AWS-LC operation. A local WIF fix alone cannot close it. Do not blanket-ban SDK-required `hmac` or `sha2`. |
| AWS-LC HMAC/SHA-256 in `rest/snowflake/workload_identity/aws.rs` | **Security-sensitive:** the locally constructed AWS attestation uses `aws_lc_rs`. | Uses the selected module; SDK SigV4 request signing remains a separate external blocker. |
| AWS-LC SHA-256 in `crl/cache.rs`, `token_cache/mod.rs`, `file_manager/s3_transfer.rs` | **Identifiers, not message signatures:** CRL cache filenames, token-cache keys and S3 credential cache fingerprints. `tls/x509_utils.rs` additionally hashes issuer/subject DER for **revocation matching**; this is policy-sensitive selection, not a substitute signature verifier. | These driver call sites use `aws_lc_rs::digest`, not the transitive RustCrypto `sha2` crate. |
| `md-5`, `sha1`, `crc32fast` in checksum paths | **Non-authentication integrity checks:** Azure Content-MD5 is computed in `file_manager/encryption.rs`; `aws-smithy-checksums` brings MD5 and SHA-1 for S3 checksums, while `crc32fast` also enters via SDK event-stream framing and `flate2` compression. SDK checksums are distinct from its HMAC request signer. | Allowed for wire-compatible checksums. A normal dependency's presence does not certify that every invocation is nonsecurity; changes to call sites require review. |
| `md-5` in `crypto/private_key.rs::legacy_pem` | **Security-sensitive KDF:** MD5-based `EVP_BytesToKey` decrypts traditional encrypted PEM only under `cfg(not(feature = "fips"))`; the FIPS build rejects it with a specific error. | Standard-build compatibility exception, **not** a blanket FIPS MD5 exemption. A crate-wide ban would also break the unrelated Azure and SDK checksum paths. |
| `pkcs8`, `pkcs5`, `x509-parser`, `x509-cert`, `der-parser` | **Parsing:** DER, X.509 and PBES2 envelope/algorithm identifiers are decoded and checked, not signed/decrypted by these parsers in driver code. The `pkcs8/3des` feature transitively adds RustCrypto `aes`, `pbkdf2`, `scrypt`, `sha2` and `des` to the graph even though this driver calls AWS-LC for AES/PBKDF2 and rejects scrypt envelopes. | Allowed as parsing dependencies; graph/SBOM membership must be explained, not misreported as executed approved crypto. Review upstream feature/code changes before revising that classification. |
| `des`, `cbc` in `crypto/private_key.rs` | **Security-sensitive legacy key unwrap on a standard build only.** PBES2-derived keys come from AWS-LC PBKDF2. 3DES-CBC decrypt runs in RustCrypto when `fips` is off. A `fips` build returns `TripleDesEncrypted` and does not call `des`. | Standard-build compatibility. The crates remain in the manifest because that build calls them and `pkcs8`'s `3des` feature names the cipher on both builds. |

F4 is separate from this crate policy: stage file-key transport still uses raw
AES-ECB, and executing it inside AWS-LC does not turn the wire construction
into an approved key-wrap operation. Likewise, the selected
`aws-lc-fips-sys 0.13.17` vendors AWS-LC FIPS 3.6.0, while
[certificate #5314](https://csrc.nist.gov/projects/cryptographic-module-validation-program/certificate/5314)
and its [Security Policy](https://csrc.nist.gov/CSRC/media/projects/cryptographic-module-validation-program/documents/security-policies/140sp5314.pdf)
identify the static AWS-LC FIPS **3.1.0** module and prescribe installation
and tested environments. No certificate for the exact linked version, tested
platform, installation procedure, approved-service mapping or final-artifact
inventory is established here. F11 is only partly addressed:
`CryptoModule` logs the runtime FIPS module version as well as the module name
and TLS-provider status, and this gate prevents named crypto regressions;
release SBOMs, artifact provenance and compliance confirmation remain required.

## The feature contract

The team has named the public feature **`fips`**. It selects `aws-lc-fips-sys`
instead of `aws-lc-sys`; its absence selects the non-FIPS module. This name
describes compile-time module selection, not a reserved future feature or a
verified whole-artifact contract. Neither the feature nor the
`fips_build_enabled` status field establishes CMVP validation or whole-driver
FIPS compliance. The status field reports only whether the feature was enabled;
`tls_provider_is_fips` continues to report the linked TLS provider's FIPS state.

Properties that follow from it being a single additive feature:

- no `compile_error!` guards, no mutual exclusion, no escape-hatch feature;
- `--all-features` selects `fips`; the dependency gate resolves metadata
  without compiling it. Buildability and certificate applicability must be
  checked separately for each FIPS toolchain and artifact;
- every final native root forwards the same `fips` feature to `sf_core`;
- feature selection stays a separate axis from protobuf, JSON, CLI, test and
  performance-timing features;
- release commands name the feature explicitly and never rely on
  `--all-features`;
- `vendored-openssl` is deleted along with the OpenSSL dependency, so the
  incoherent FIPS-TLS-plus-vendored-OpenSSL artifact is now unbuildable by
  construction rather than by assertion (done; see F2).

Runtime configuration may *require* or *report* the compiled state, but must
never change it. A connection option can say "require FIPS" and fail when the
binary is a non-FIPS build; it cannot convert one into the other.

### Module ownership

`tls::crypto_module::CryptoModule` is the accessor for the selected TLS
provider. It is selected entirely at compile time, holds its own rustls
`CryptoProvider`, and supplies:

- the rustls provider handed to `ClientConfig::builder_with_provider` and
  `WebPkiServerVerifier::builder_with_provider`;
- the signature algorithms used for certificate-chain and CRL verification;
- the linked module's name and provider FIPS state for local diagnostics (not
  its exact version, certificate, or release attestation).

The direct AWS-LC calls for RNG, hashing, RSA/EC signing, AES and PBKDF2 are
selected by Cargo features, but do not go through this TLS accessor. SDK SigV4
and WIF attestation signing remain outside the module at this baseline.

It deliberately exposes no constructor taking an arbitrary provider. Which
module is in use is a compile-time fact; a runtime setter would reintroduce the
ambiguity the type exists to remove.

Holding its own provider — rather than reading `CryptoProvider::get_default()`
— is the point. The global install yields to an embedding application that got
there first, which is right for a global slot but would mean the module
carrying our traffic is decided by a startup race. Once every reqwest client is
built from an explicit rustls `ClientConfig`, the best-effort global install
(`ensure_crypto_provider()`) is retired.

## Removing OpenSSL

This was the bulk of the original port, and these production OpenSSL call
sites have been replaced:

| File | Ported operations | Remaining boundary |
| --- | --- | --- |
| `auth.rs`, `crypto/private_key.rs` | JWT RSA-SHA256 signing, PEM/DER key loading, AWS-LC PBKDF2 and AES decryption | 3DES unwrap and traditional encrypted PEM (MD5 KDF) load only without `fips`. |
| `config/private_key.rs` | Private-key DER/PEM decoding using the shared key loader | Parsing is not a cryptographic module service. |
| `file_manager/encryption.rs` | AWS-LC AES-CBC stage encryption, digest, RNG, and AES-ECB file-key transport | F4: raw AES-ECB is not an approved key-wrap construction despite executing inside AWS-LC. |
| `rest/snowflake/oauth/dpop.rs` | AWS-LC P-256 key generation, ECDSA and SHA-256 | SDK SigV4 and WIF attestation are separate signer paths. |

Sequencing mattered: **`vendored-openssl` could not be removed before the port
completed.** Release builds passed it to statically link OpenSSL, so deleting
the feature first would have made shipped artifacts depend on a host OpenSSL — a
deployment regression, and the opposite of the self-containment the AWS-LC
decision buys. The order followed was: port the four files, drop the `openssl`
dependency to dev-only, then delete `vendored-openssl`.

One consequence outlives the removal. `cargo build` does not compile
dev-dependencies, so shipped artifacts link no OpenSSL at all — but `cargo test`
still does, because the cert fixtures below need it. Test lanes therefore still
have to provide an OpenSSL (vcpkg on Windows, distro packages elsewhere, and
`openssl-libs` in the WIF runtime container, whose e2e binary now links it
dynamically instead of statically).

Cert-fixture generation in `crl/certificate_parser.rs`, `tls/x509_utils.rs` and
`tls/test_helpers.rs` is test-only. It may keep OpenSSL as a dev-dependency
indefinitely: it never enters a shipped artifact, and porting it buys nothing
for the compliance claim.

`file_manager/encryption.rs` deserves the most care of the four. It is
wire-format-compatible encryption of customer data, where an error means
corruption or silent weakening rather than a failed build. F4 also lives there.

## CRL contract

CRL behaviour is one shared policy implementation regardless of the flag. These
operations are backend-independent and stay in Rust:

- certificate and CRL DER parsing; distribution-point extraction;
- CRL HTTP fetching, retry, timeout and maximum-size enforcement;
- memory and disk caching; expiry and freshness policy;
- issuer-name and AKID/SKID matching; issuing-distribution-point policy;
- candidate-chain and trust-anchor selection; revoked-serial comparison;
- `Disabled`, `Enabled` and `Advisory` behaviour.

Only the cryptographic boundaries come from the module:

1. TLS handshake and certificate-chain signatures use the module's
   `WebPkiSupportedAlgorithms`.
2. Downloaded CRL signatures use the module's certificate-signature verifier.
3. An HTTPS CRL download uses a separate reqwest client with the module's
   provider, native ∪ webpki roots, hostname verification and default rustls
   protocols. CRL checks stay off on this client to prevent recursive
   validation. Plain `http://` distribution points perform no TLS.

The signature verifier must accept the CRL signature `AlgorithmIdentifier`,
issuer SPKI, exact DER-encoded `tbsCertList`, and signature bytes. It must:

- support the approved RSA PKCS#1, RSA-PSS and ECDSA combinations the
  certificate ecosystem requires;
- parse and enforce RSA-PSS hash, MGF1 hash, salt length and trailer
  parameters rather than trying multiple hashes;
- distinguish unsupported/non-approved algorithms from invalid signatures;
- reject Ed25519 or any algorithm the module's Security Policy does not
  approve;
- never retry verification through another implementation. An unsupported
  algorithm and an invalid signature are different outcomes, and neither is a
  reason to consult a second verifier.

## Distribution and compliance ownership

For Snowflake-built outputs, Snowflake owns the source revision, lockfile,
toolchain, module mapping, build provenance, SBOM, artifact signature, runtime
assertions, supported platform, and public claim.

For consumer-built outputs, Snowflake provides the exact source, lockfile,
supported build recipe, crypto inventory and runtime diagnostics. The consumer
owns toolchain and host provenance, the resulting native artifact, SBOM and
signing, validated-module applicability, deployment evidence, and any
packaging-introduced modules.

Release channels may publish only the platform combinations Snowflake chooses
to support. Consumer builds remain useful for platforms Snowflake does not
publish, but must not inherit a claim attached to a different binary.

## Remediation phases

### Phase 1 — Make the FIPS feature reachable (done)

- Added the passthrough to the four binding crates.
- Explicitly enabled AWS-LC FIPS for the coupled CRL verifier.
- Removed dead `rustls-webpki 0.102`.
- Added AWS-LC/rustls FIPS assertions.

### Phase 2 — Consolidate TLS on rustls/AWS-LC (done)

- Routed AWS Workload Identity through the shared reqwest transport.
- Installed the AWS-LC provider before every known client-construction path.
- Removed reqwest's ring fallback via `-no-provider` feature selection.
- Disabled `oauth2` defaults that re-enabled reqwest/ring through unification.

### Phase 3 — Establish the module boundary (in progress)

Landed:

- `CryptoModule`, holding the module's own rustls provider.
- Every rustls `ClientConfig`, `WebPkiServerVerifier`, `CrlServerCertVerifier`
  and `NoVerifyCertVerifier` that **this crate builds** takes that provider
  explicitly. Verified connection and cloud-storage clients now hand reqwest
  module-backed configs on the CRL-disabled, enabled, and advisory paths.
  CRL-disabled default trust is native ∪ webpki roots, with custom-root
  replacement and extra-root additions preserved. CRL-enabled and advisory
  defaults stay on the native store; extra roots there are native ∪ extra.
- HTTPS CRL downloads use a module-backed config with native ∪ webpki roots
  and no CRL checking, so they neither recurse nor depend on a host-installed
  provider, even before the first connection.
- FIPS settings and TLS builders reject disabled certificate verification.
  The insecure reqwest and diagnostic paths remain available only in standard
  builds; hostname-only opt-outs still use the module in both builds.
- `tls_provider_is_fips()` reports on the linked module's provider rather than
  the global slot. Named for the TLS provider deliberately: it is rustls's
  per-provider flag, not the artifact-level answer. `try_fips_mode()`, asserted
  by the `aws_lc_reports_fips_mode` test, checks whether the linked module
  entered FIPS mode; it does **not** establish its certificate applicability.
- The module's name (not exact vendored version or certificate) is logged once
  at initialization.

Remaining:

Snowflake `/telemetry/send` uses the verified connection client. IMDS and the
local OpenTelemetry exporter (`http://localhost:8318`) are plain HTTP.

F14 has no remaining insecure-client or HTTPS-CRL path. The best-effort global
install remains for the standard-build insecure client, plain-HTTP clients,
and platform-detection probes, whose FIPS provider gate is unchanged.

- Keep feature selection a separate axis from functional features.

### Phase 4 — Port crypto off OpenSSL (partly done)

- Port the four direct production files above to AWS-LC. **Done.** 3DES unwrap
  and MD5 traditional PEM remain standard-build compatibility; `fips`
  rejects both.
- Move CRL signature verification behind the module interface; enforce exact
  RSA-PSS parameters and the approved algorithm set.
- Remove ring from the shipped Rust graph. **Done:** SDK legacy `rustls`,
  `sigv4a` and `sso` are off; test-only `rcgen` still depends on ring.
- Move locally constructed WIF attestation HMAC/SHA-256 to AWS-LC; independently
  resolve plain SigV4 HMAC/SHA-256 inside `aws-sigv4` with the SDK owner (F7).
  The latter signs S3 requests and is not removed by disabling `sigv4a`.
- OAuth CSRF and PKCE randomness onto the module's DRBG (F8). **Done**, and the
  PKCE challenge digest moved to AWS-LC with it.
- Drop production `openssl`, then delete `vendored-openssl` (F2). **Done.**
- Use the agreed `fips` feature name now; boundary and validation evidence remain independent requirements.
- Keep RustCrypto `sha2`/`hmac` usage explicitly classified: identifiers and
  checksums differ from WIF/SigV4 signing; no crate-wide ban can express it.

F4 remains a protocol finding: executing raw AES-ECB inside a validated module
does not make it an approved key-transport construction.

### Phase 5 — Prove it (dependency gate added; release evidence open)

Run the complete behavioural suite separately with and without `fips`.
Never use a mixed `--all-features` build as whole-artifact FIPS evidence.

- runtime module mode and exact linked version match the selected build and an
  applicable certificate, Security Policy, tested environment and installation;
- every constructed `ClientConfig` uses the selected module, including when
  another provider was installed process-wide first;
- selected TLS versions, suites, groups and signature algorithms match the
  approved set;
- native artifact inspection checks release ODBC and Node.js binaries for the
  locked FIPS AWS-LC prefix and rejects non-FIPS AWS-LC, ring, unprefixed public
  OpenSSL API entry points and OpenSSL shared libraries, beyond the Cargo graph gate;
- ODBC `SFGetTlsStatus` (loaded via Python `ctypes`) and Node.js `getTlsStatus()`
  both report the FIPS provider and build flag through the inspected artifacts.
  Python/JDBC wrapper artifact work remains with their owning teams.

The `fips_artifacts` job uses the same Linux x64/ARM64, macOS x64/ARM64 and
Windows x64 matrix and shared toolchain action as `fips_smoke`. It builds with
`CARGO_PROFILE_RELEASE_STRIP=false`, retaining the release profile's thin LTO
and single codegen unit. Disabling final stripping preserves the shipped
linker's object selection while making Unix symbols available to `nm`.
OpenSSL detection uses explicit public API names rather than broad namespace
prefixes, which would misclassify AWS-LC's private static/inline helpers.
`readelf -d` on Linux and `otool -L` on macOS also reject `libssl`/`libcrypto`
dependencies. On Windows, `dumpbin /dependents` must name the locked FIPS
crypto DLL and no OpenSSL DLL, and `cargo rustc` emits a separate `/MAP` for
each final crate to check statically linked symbols too. Unique map paths
force fresh final links rather than reusing stale cached map evidence. The checker derives
the expected FIPS prefix from `Cargo.lock` and accepts arbitrary native
artifact paths, not only the two bindings in this job. Node's build script
opts into release with `SF_CORE_RELEASE=1`; its default stays debug.

F15's graph fact cannot be removed by disabling rustls defaults: the cached
`rustls 0.23.32` and `0.23.43` manifests both wire `fips` to `aws_lc_rs`,
which enables `aws-lc-rs/aws-lc-sys`, as well as enabling `aws-lc-rs?/fips`.
`0.23.43` is the newest compatible release under our `aws-lc-rs <1.18`
bound; the `0.23.44` manifest requires `aws-lc-rs 1.18`. Keeping the graph
entry is therefore intentional. The artifact check does not inventory all
RustCrypto operations or establish CMVP validation; final-artifact SBOMs and
release attestations are still required.

The `fips_smoke` CI lane runs offline TLS, crypto, authentication, OAuth
(DPoP, PKCE and DRBG), stage-encryption, CRL, AWS WIF and private-key unit
tests with `fips`, excluding three CRL unit tests that fetch public URLs.
Its AWS, Azure and GCP connectivity smoke checks the
TLS status API and a certificate-verified TLS 1.2/1.3 handshake with a FIPS
cipher suite; JWT login with an in-test AES-256-CBC PBES2-encrypted copy of the
account key; rejection of 3DES PKCS#8 and traditional encrypted PEM with
conversion guidance; and all 500,000 rows of a result with remote storage
chunks. The existing CRL-enabled SELECT and PUT/GET round-trip remain.
These checks establish selected-path coverage, not module validation or a
whole-artifact FIPS claim.

CRL matrix, for both builds: valid leaf succeeds; revoked leaf fails in
`Enabled` and `Advisory`; revoked intermediate fails; unavailable CRL differs
correctly between modes; invalid CRL signature fails; issuer and AKID/SKID
mismatch fail; trust-anchor fallback verifies the top-most intermediate CRL;
RSA PKCS#1, exact RSA-PSS and ECDSA CRLs verify when approved; unsupported
algorithms fail closed; HTTP and HTTPS distribution points exercise retry,
cache, size and timeout behaviour; HTTPS CRL download proves no recursive CRL
checking; and equivalent fixtures produce equivalent revocation outcomes with
and without `fips`, proving the port changed the backend and not the policy.

Release evidence additionally requires `try_fips_mode()`, an exact AWS-LC
module/version, certificate and Security Policy mapping, the pinned build
environment and the final artifact's linked-module inventory.

The standalone `cargo-deny check bans` gate now guards named shipped Rust
dependency paths (F11); CycloneDX SBOMs for attested release artifacts remain
to be generated and reviewed.

### Phase 6 — Ship and document

- Add explicit FIPS lanes to each supported artifact workflow.
- Publish a support matrix mapping platform, architecture, validated module,
  certificate, Security Policy, toolchain and artifact naming.
- Document exact consumer build commands for each final native root and state
  which provenance obligations transfer to the consumer.
- Expose structured status through every binding: module identity and version,
  FIPS active state, certificate/Security Policy identifier, and any
  initialization failure reason.
- Resolve JDBC key-material handling and determine whether Python can avoid a
  second bundled OpenSSL by moving DER conversion into `sf_core` (F13).
- Raise F4 with stage-protocol owners in parallel; document it as a known
  deviation until a coordinated protocol change exists.

## Toolchain and platform constraints

The previously tested `aws-lc-fips-sys 0.13.12` failed with GCC 15 and clang
20; the known working setup pinned GCC 13. The current lockfile instead
contains `0.13.17`, whose exact toolchain/environment still needs verification:

```bash
conda create -y -n ud-fips -c conda-forge 'gcc_linux-64=13' 'gxx_linux-64=13' \
  cmake make ninja go perl
export AWS_LC_FIPS_SYS_CC=$HOME/conda/envs/ud-fips/bin/x86_64-conda-linux-gnu-gcc
export AWS_LC_FIPS_SYS_CXX=$HOME/conda/envs/ud-fips/bin/x86_64-conda-linux-gnu-g++
```

The previously tested build used system CMake 3.31.x; CMake 4 rejected that
AWS-LC build's version range. Recheck this and Windows ARM64 for the exact
resolved module before claiming a supported platform.

Platform coverage is a single-backend problem: a platform that cannot build
`aws-lc-fips-sys` has no FIPS artifact, because there is no alternative backend
to cover it. Treat AWS-LC platform support as a release-blocking input when
choosing publish targets, and re-evaluate exclusions on each AWS-LC release
rather than treating today's as permanent.

## Rejected alternatives

### A system-OpenSSL FIPS backend

Routing crypto through a named system OpenSSL 3 FIPS provider would have bought
no pinned build toolchain, coverage on platforms where `aws-lc-fips-sys` does
not build (including Windows ARM64), and an answer for customers whose policy
names their OS vendor's module specifically.

It loses anyway:

1. **It does not survive the distribution model.** Snowflake publishes prebuilt
   artifacts. A host-resolved module means the thing Snowflake signs and the
   module the claim rests on are different artifacts, chosen at different times
   by different parties. The builder cannot attest what it did not select.
2. **A host upgrade silently changes the module** under a fixed driver binary,
   so a routine OS patch could invalidate a standing claim.
3. **The support matrix multiplies** across distribution × OpenSSL package
   version × certificate × ABI, and building against an arbitrary OpenSSL 3 is
   not support evidence.
4. **It needs an OpenSSL-backed rustls provider Snowflake would own** —
   `rustls-openssl` was the candidate, requiring an exact pin, an audit of its
   unsafe/provider boundary, and supply-chain review.
5. **A hybrid is worse than either.** AWS-LC for TLS plus system OpenSSL for
   application crypto puts two modules in one artifact, each needing separate
   inventory and attestation.

The cost accepted: F12 becomes the ceiling on FIPS platform coverage, and the
pinned GCC 13 toolchain is a standing build-infrastructure obligation until
AWS-LC's own support moves.

### Named compile-time profiles

See "Why one flag rather than named profiles" above. Two or three named
profiles required `compile_error!` guards that fired under `--all-features`,
plus an escape-hatch feature to disarm them — machinery that a single additive
flag makes unnecessary.

## Open questions for stakeholders

1. Which AWS-LC FIPS module version, validated certificate, and Security Policy
   will the `fips` build claim?
2. Which platform/architecture combinations will Snowflake publish and attest,
   given that `aws-lc-fips-sys` build support is the binding constraint and
   Windows ARM64 is currently excluded?
3. What exact public wording distinguishes Snowflake-built artifacts from
   consumer-built ones?
4. Who owns the coordinated stage-protocol change for F4?
5. If a customer mandates their OS vendor's OpenSSL module specifically, what is
   the answer? The rejection above assumes that case is rare enough to decline;
   confirm with the FedRAMP/gov account owners before it is load-bearing.
6. `aws-sigv4` computes plain SigV4 HMAC-SHA256 via RustCrypto on S3 requests.
   The local WIF `GetCallerIdentity` signer is AWS-LC and is not part of this
   question. Who owns moving the SDK signer to the selected module or
   documenting that independent deviation? Neither a dependency deny gate nor
   a local signer change closes it.
