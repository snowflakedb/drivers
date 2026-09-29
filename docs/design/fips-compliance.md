# FIPS compliance: one crypto module, one flag

Design note for making the universal driver capable of producing FIPS 140-3
compliant builds. It captures the audit findings (2026-08-31), the work already
landed, and the agreed architecture: **all cryptography runs on AWS-LC, and a
single Cargo feature decides whether the linked AWS-LC is the FIPS-validated
build.**

Status at a glance:

| Item | State |
| --- | --- |
| FIPS as the default build | **Decided: no.** The default links non-FIPS AWS-LC. |
| Crypto selection | **Decided: one backend, one flag.** AWS-LC for every security-relevant operation; the `fips` feature selects `aws-lc-fips-sys` instead of `aws-lc-sys`. No named profiles, no runtime backend switch, no provider plugin ABI. |
| OpenSSL | **Decided: removed.** Not a second backend, not a fallback, not vendored. Four production call sites remain to port (below). |
| Distribution model | Snowflake-built and consumer-built native artifacts both supported. Snowflake can claim compliance only for artifacts and environments it attests; consumer builds are a supported escape hatch with consumer-owned provenance. |
| Validated module + certificate number | **Open.** The AWS-LC FIPS module version, certificate, Security Policy, and supported platform list still need to be fixed and stated. |
| Phases 1–2 | Done and verified. |
| Phase 3 | In progress. The crypto-module boundary has landed; retiring the global provider install remains. |
| Phases 4–6 | Not started. Port the remaining crypto off OpenSSL, prove it, ship it. |

## Bottom line

**No shipped artifact is FIPS-capable today.** The current `fips-tls` feature
makes AWS-LC FIPS reachable through the binding crates and covers rustls TLS
plus the coupled AWS-LC CRL signature verifier. Non-TLS operations — JWT
signing, private-key parsing, stage file encryption, DPoP — still run on
OpenSSL. The feature is named for exactly that scope; it becomes `fips` when
the port below lands and the name is true.

The end state is one source tree, one cryptographic module, and one flag:

- **without `fips`** — AWS-LC via `aws-lc-sys`. The default. No FIPS claim.
- **with `fips`** — AWS-LC via `aws-lc-fips-sys`, the validated build, for TLS
  and non-TLS crypto alike. One module in the artifact, and no reachable
  non-approved fallback.

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
| F3 | Blocker | Crypto coverage is incomplete: JWT signing, private-key parsing, stage encryption, DPoP, SigV4 and OAuth randomness bypass the validated module | `sf_core` crypto call sites — the Phase 4 port |
| F4 | High | Per-file keys are wrapped with raw AES-ECB, not an approved key-transport construction such as AES-KW/KWP. AWS-LC *can* perform ECB (`aws-lc-rs` exposes `OperatingMode::ECB`), so this is not an implementation blocker — it is a wire-format problem: running an unapproved construction inside a validated module does not make it approved. Shared Snowflake stage format; not unilaterally fixable here | `file_manager/encryption.rs` |
| F5 | High | A transitive reqwest feature can re-enable ring and its implicit provider fallback through feature unification | TLS dependency graph — fixed in Phase 2 |
| F6 | High | AWS Workload Identity previously bypassed shared TLS, proxy, root and CRL policy | `workload_identity/aws.rs` — fixed in Phase 2 |
| F7 | **High**, partially closed | Two separable problems. **(a) ring is linked.** The 2026-09-15 note attributed this to `aws-sdk-s3/sigv4a` alone; that was incomplete. ring had three gated sources: the `rustls` feature on `aws-sdk-s3`/`aws-sdk-sts` (which expands to `aws-smithy-http-client/legacy-rustls-ring` and dragged in hyper 0.14 and a *second*, older rustls 0.21), `aws-sdk-s3/sigv4a`, and `aws-config/sso`. The first two are removed: the build now carries one rustls (0.23, aws-lc) instead of two, and measured ring symbols fall **221 → 175**. Note `default-https-client` is *not* a ring path — it maps to `rustls-aws-lc` — and is deliberately kept. **The residual 175 is accepted:** `aws-config/sso` hard-requires `dep:ring`, and dropping it (verified to reach 0) would remove IAM Identity Center credentials from the SDK default chain, which WIF ambient discovery and platform detection both rely on. Decision 2026-09-18: keep SSO, record ring as a known deviation. Cargo features cannot be negated, so this cannot be split per-build. **(b) SigV4 HMAC is outside the module.** Plain SigV4 (`aws-sigv4/src/sign/v4.rs`) computes HMAC-SHA256/SHA-256 via RustCrypto `hmac`/`sha2` **unconditionally**, so the scope is every S3 stage transfer, not just WIF. Not fixable from this repo — see open question 7 | `workload_identity/aws.rs` **and** all `aws-sdk-s3` traffic |
| F8 | **Resolved** (was Medium) | CSRF `state` and the PKCE code verifier now come from AWS-LC's DRBG via `oauth::random::token_b64url`, and the PKCE challenge digest from AWS-LC rather than RustCrypto. `oauth2` will only build a `PkceCodeChallenge` around a digest it computed itself, so the flow appends `code_challenge`/`code_challenge_method` directly — the same two parameters `set_pkce_challenge` emits. An end-to-end test asserts the challenge is the S256 digest of the verifier presented at the token endpoint, so the bypass cannot drift silently. A DRBG failure fails the login (`OAuthError::RandomGeneration`) rather than falling back. `rand` remains in the graph for retry/refresh jitter and a non-secret FFI handle tag — no security-relevant use | OAuth authorization-code flow |
| F9 | Medium | `md-5` and `sha1` remain in the AWS checksum dependency graph; actual use must be classified and constrained. Confirmed present as normal (non-dev) deps in the `fips-tls` graph as of 2026-09-15, together with `crc32fast` | S3 transfer dependencies |
| F10 | Low | A dead ring-only `rustls-webpki 0.102` dependency was present | Removed in Phase 1 |
| F11 | Low | No module identity report, SBOM policy, or dependency-deny guard exists | Repository-wide |
| F12 | Medium | `aws-lc-fips-sys` has platform and toolchain limitations, including Windows ARM64 and current GCC/Clang failures. With a single backend there is no alternative, so this bounds which platforms can have a FIPS build at all | `fips` build infrastructure |
| F13 | Low | JDBC retains key bytes in a Java `String`; Python may load a second unvalidated OpenSSL | JDBC and Python wrappers |
| F14 | Blocker | TLS verification algorithms and CRL signature verification called AWS-LC directly rather than through the selected module, and rustls configs read the process-global provider | `tls/crl_verifier.rs`, `tls/client.rs` — **partially addressed in Phase 3; still open.** Every rustls config this crate builds itself now takes the module's provider explicitly, which closes the verification-algorithm half. The second half stands: only the CRL-enabled paths hand reqwest a config (`use_preconfigured_tls`). On the default `CertRevocationCheckMode::Disabled` path reqwest still builds its own config and resolves `CryptoProvider::get_default()`, so "we own the module that verifies the connection" holds only when a customer turns CRL on. Closing it means giving every traffic-carrying client a module-backed config, which narrows the trust anchors (see Phase 3 Remaining) and needs its own change |
| F15 | Low (build/inventory only) | `rustls/fips` transitively enables `aws-lc-rs/aws-lc-sys`, so a FIPS build compiles **both** AWS-LC modules. Verified 2026-09-15 that this does **not** contaminate the artifact: `libaws_lc_sys-*.rlib` is never passed to the linker (aws-lc-rs omits `extern crate aws_lc_sys` under `fips`), the binary holds 1558 `aws_lc_fips_0_13_12_*` symbols and **0** `aws_lc_0_38_0_*`, and the two archives' exported symbol sets are disjoint with 3824/3824 non-FIPS exports prefixed. Cost is wasted build time plus an SBOM that lists two crypto modules. Not fixable in `sf_core`; file upstream with rustls | `rustls` feature wiring (0.23.32–0.23.45) |
| F16 | Resolved (decision taken) — **FIPS builds only** | Traditional encrypted PEM keys (`RSA PRIVATE KEY` + `Proc-Type: 4,ENCRYPTED`, from `openssl rsa -aes256`) load in **standard builds** and are refused in **`fips-tls` builds**. Their KDF is `EVP_BytesToKey`, which is MD5-based and definitional to the format; AWS-LC exposes no MD5, so a FIPS build cannot read them under any reading of the rules. Distinct from the 3DES arm, where PBKDF2-HMAC-SHA is approved and SP 800-131A permits TDEA unwrapping for legacy use. **Decision (2026-09-15): parity for default artifacts wins.** Refusing everywhere dropped a working format from every shipped artifact to protect a claim only the FIPS artifact makes; a standard build asserts no FIPS compliance, so MD5-unwrapping a key its owner already holds compromises nothing. Implemented behind `cfg(not(feature = "fips-tls"))`; only the KDF is MD5, the bulk decryption still runs in AWS-LC (AES) / `des` (3DES). **This is the only place a build flag changes which keys load rather than only which module does the work** — so the FIPS-build error says so explicitly, to keep "wrong build" distinguishable from "broken key". Residual: still worth a release note for FIPS-build users | `crypto/private_key.rs::legacy_pem`; raised by both review bots on PR #1346 |

## The feature contract

One public Cargo feature governs cryptography:

- **`fips`** (currently `fips-tls`) — links `aws-lc-fips-sys` and asserts FIPS
  mode. Its absence links `aws-lc-sys`.

Properties that follow from it being a single additive feature:

- no `compile_error!` guards, no mutual exclusion, no escape-hatch feature;
- `--all-features` stays buildable. Note it already requires the pinned GCC 13
  toolchain, because `fips` pulls `aws-lc-fips-sys` — true on `main` today, not
  a consequence of this work;
- every final native root forwards the same name, e.g.
  `python_bridge/fips -> sf_core/fips`;
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

`tls::crypto_module::CryptoModule` is the single accessor for the linked
module. It is selected entirely at compile time, holds its own rustls
`CryptoProvider`, and owns:

- the rustls provider handed to `ClientConfig::builder_with_provider` and
  `WebPkiServerVerifier::builder_with_provider`;
- the signature algorithms used for certificate-chain and CRL verification;
- FIPS-mode state and module identity for reporting;
- and, as Phase 4 lands: random bytes, hashing/HMAC, RSA and EC key
  import/generation/signing, and symmetric encryption.

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

This was the bulk of the remaining work, and the port is complete. Production
OpenSSL use was four files:

| File | Operations | AWS-LC replacement | Gap |
| --- | --- | --- | --- |
| `auth.rs` | RSA private-key PEM parse (incl. passphrase), SHA-256 public-key fingerprint, RSA-SHA256 JWT signing | `aws_lc_rs::digest`, `signature::RsaKeyPair` | Encrypted PEM has no `aws-lc-rs` equivalent; the `jwt 0.16` crate is OpenSSL-bound via `PKeyWithDigest` |
| `config/private_key.rs` | `PKey` private-key parsing | `signature::RsaKeyPair::from_der` after PEM→DER decode | PEM/PKCS#8 decode needs a separate crate |
| `file_manager/encryption.rs` | AES-128/256-CBC streaming (64 KiB chunks), AES-ECB key wrap, digest, RNG | `aws_lc_rs::cipher` (`OperatingMode::CBC`/`ECB`), `digest`, `rand` | Chunked streaming API needs checking against `aws-lc-rs`'s one-shot/streaming split |
| `rest/snowflake/oauth/dpop.rs` | P-256 keygen, ECDSA signing, SHA-256, BigNum JWK coordinate extraction | `signature::EcdsaKeyPair`, `digest` | Coordinates come from the uncompressed point encoding rather than BigNum |

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
3. HTTPS CRL downloads use an explicitly configured TLS client from the same
   module, without recursively enabling CRL checks.

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
  explicitly. That is half of F14; the rest is listed under Remaining, because
  reqwest only receives one of those configs on the CRL-enabled paths.
- `tls_provider_is_fips()` reports on the linked module's provider rather than
  the global slot. Named for the TLS provider deliberately: it is rustls's
  per-provider flag, not the artifact-level answer. `try_fips_mode()`, asserted
  by the `aws_lc_reports_fips_mode` test, stays the source of truth for "is the
  module in approved mode".
- Module identity logged once at initialization.

Remaining:

- **Close F14** by giving the *default* (CRL-disabled) connection path a
  module-backed config too. Today it is the only shape where reqwest builds its
  own config and resolves the process-global provider, so the module boundary
  is real only for customers who enable CRL. Blocked on trust-anchor scope:
  reqwest is configured with both `rustls-tls-native-roots-no-provider` and
  `rustls-tls-webpki-roots-no-provider`, so its default trust set is native
  roots ∪ webpki roots, while the configs this module builds use native roots
  only. Switching the default path silently narrows every connection's trust
  anchors, which needs its own change and its own testing.
- Convert the auxiliary reqwest clients (telemetry, CRL fetch, IMDS) to
  explicit rustls configs, then retire `ensure_crypto_provider()` and the
  process-global install entirely.
- Keep feature selection a separate axis from functional features.

### Phase 4 — Port crypto off OpenSSL

- Port the four production files in the table above onto AWS-LC.
- Move CRL signature verification behind the module interface; enforce exact
  RSA-PSS parameters and the approved algorithm set.
- SigV4 HMAC/SHA-256 off RustCrypto, and the ring path removed or contained
  (F7). **Partially done:** the `rustls`/`sigv4a` ring paths are gone (221 → 175
  symbols); the `aws-config/sso` residual is an accepted deviation and the
  RustCrypto HMAC needs upstream movement.
- OAuth CSRF and PKCE randomness onto the module's DRBG (F8). **Done**, and the
  PKCE challenge digest moved to AWS-LC with it.
- Drop the `openssl` dependency, then delete `vendored-openssl` (F2). **Done.**
- Rename `fips-tls` to `fips` once — and only once — coverage is complete.
- Leave `sha2` only where an operation is provably non-security-relevant, such
  as a cache filename, and record that classification at the call site so
  dependency audits do not read it as a bypass.

F4 remains a protocol finding: executing raw AES-ECB inside a validated module
does not make it an approved key-transport construction.

### Phase 5 — Prove it

Run the complete behavioural suite separately with and without `fips`. Never
use one mixed `--all-features` build as evidence.

- runtime module identity equals the compiled feature;
- every constructed `ClientConfig` reports the expected FIPS state;
- selected TLS versions, suites, groups and signature algorithms match the
  approved set;
- negative tests prove a missing or conflicting preinstalled provider fails
  before any network traffic;
- native dependency inventory proves OpenSSL and ring are absent;
- all wrappers build and report the same module status.

CRL matrix, for both builds: valid leaf succeeds; revoked leaf fails in
`Enabled` and `Advisory`; revoked intermediate fails; unavailable CRL differs
correctly between modes; invalid CRL signature fails; issuer and AKID/SKID
mismatch fail; trust-anchor fallback verifies the top-most intermediate CRL;
RSA PKCS#1, exact RSA-PSS and ECDSA CRLs verify when approved; unsupported
algorithms fail closed; HTTP and HTTPS distribution points exercise retry,
cache, size and timeout behaviour; HTTPS CRL download proves no recursive CRL
checking; and equivalent fixtures produce equivalent revocation outcomes with
and without `fips`, proving the port changed the backend and not the policy.

`fips` evidence additionally includes `try_fips_mode()`, AWS-LC module identity
and version, the certificate and Security Policy identifiers, and the pinned
build environment that produced the artifact.

Add `cargo-deny` dependency policy checks and generate CycloneDX SBOMs for
attested release artifacts.

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

`aws-lc-fips-sys 0.13.12` fails on GCC 15 and clang 20 in the currently tested
environment. The known working setup pins GCC 13:

```bash
conda create -y -n ud-fips -c conda-forge 'gcc_linux-64=13' 'gxx_linux-64=13' \
  cmake make ninja go perl
export AWS_LC_FIPS_SYS_CC=$HOME/conda/envs/ud-fips/bin/x86_64-conda-linux-gnu-gcc
export AWS_LC_FIPS_SYS_CXX=$HOME/conda/envs/ud-fips/bin/x86_64-conda-linux-gnu-g++
```

Keep system CMake 3.31.x on `PATH`; CMake 4 rejects the version range the
current AWS-LC build uses. Windows ARM64 remains blocked.

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
6. Encrypted-PEM private keys have no `aws-lc-rs` equivalent. Which crate takes
   that job, and does it need to be in the validated boundary at all — key
   *decryption* at rest is arguably not a security-relevant operation for the
   claim, but that needs a written position rather than an assumption.
7. `aws-sigv4` computes SigV4's HMAC-SHA256 through RustCrypto unconditionally,
   on every stage transfer, and `aws-config/sso` keeps `ring` in the artifact.
   Neither is fixable here. Does Snowflake raise these with the AWS SDK team, or
   are they written up as known deviations in the Security Policy mapping? This
   is the last open item standing between the current build and a complete
   crypto-coverage claim.
