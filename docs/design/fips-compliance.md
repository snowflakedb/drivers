# FIPS compliance: one crypto module, one flag

One AWS-LC module for security-relevant cryptography, and one Cargo feature
that selects its FIPS build.

| Item | State |
| --- | --- |
| FIPS as the default build | **Decided: no.** The default links non-FIPS AWS-LC. |
| Crypto selection | **One backend, one flag named `fips`.** The feature selects the AWS-LC FIPS build. |
| OpenSSL | **Removed from production Rust crypto.** `openssl` remains a dev-dependency for independent fixture/reference tests; `openssl-probe` discovers CA locations without linking OpenSSL. |
| Module | **AWS-LC FIPS 3.6.0**, pinned with `aws-lc-fips-sys = "=0.13.17"` and asserted at runtime in the FIPS smoke tests. [Certificate #5314](https://csrc.nist.gov/projects/cryptographic-module-validation-program/certificate/5314) names the static 3.1.0 module. Applicability of that certificate to this 3.6.0 build is an open mapping. |
| Evidence | ODBC and Node.js FIPS CI libraries have a candidate CycloneDX bundle. It covers the unstripped CI library. Installer SBOMs and RelEng attestation of the signed bytes remain open. |

## Module version

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

[Certificate #5314](https://csrc.nist.gov/projects/cryptographic-module-validation-program/certificate/5314)
and its [Security Policy](https://csrc.nist.gov/CSRC/media/projects/cryptographic-module-validation-program/documents/security-policies/140sp5314.pdf)
name the static 3.1.0 module and specific Amazon Linux 2023 tested environments.
They do not by themselves establish applicability to 3.6.0 or to Ubuntu,
macOS, or Windows packages. Neither downgrading nor locally patching 3.1.0
establishes that coverage.

## Exceptions

`fips` selects AWS-LC FIPS for rustls and the driver's AWS-LC calls,
including JWT signing, AES stage encryption, DPoP, and approved-key
decryption. HTTPS CRL downloads use the linked module. FIPS builds reject
`verify_certificates=false` and `tls_skip_verify=true`.

These operations stay outside that selection:

- AWS SDK SigV4 signing uses RustCrypto HMAC/SHA-256 on every S3 stage transfer.
- A standard build unwraps 3DES private keys with `des` outside AWS-LC. A `fips` build rejects those keys. Traditional encrypted PEM (MD5 KDF) loads only on a standard build.
- The stage key-wrap format uses raw AES-ECB.
- Azure Content-MD5 and SDK checksums use `md-5` and `sha1` as non-authentication integrity checks.

`sha2`, `hmac`, `md-5`, and `des` stay in the graph for those call sites. A
crate-wide ban does not match that classification.

## One flag

The feature is **`fips`**. It selects `aws-lc-fips-sys` instead of
`aws-lc-sys`. Its absence selects the non-FIPS module.

`is_fips` is true when the binary was built with `fips` and the linked TLS
provider is in FIPS mode. A standard build returns false. A `fips` build
whose provider did not come up in FIPS mode also returns false.

- **without `fips`** — AWS-LC via `aws-lc-sys`. The default.
- **with `fips`** — AWS-LC FIPS via `aws-lc-fips-sys`.

Cargo features are additive, so the flag has no `compile_error!` guards, no
mutual exclusion, and no escape-hatch feature. `--all-features` selects
`fips`. Every final native root forwards the same `fips` feature to
`sf_core`. Release commands name the feature explicitly.

Runtime configuration may require or report the compiled state, and it does
not change it. A connection option can fail when the binary is a non-FIPS
build. It cannot convert one build into the other.

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
