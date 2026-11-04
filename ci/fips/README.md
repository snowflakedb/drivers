# FIPS candidate evidence

`release_evidence.py` writes one bundle for a native library built with the `fips` feature. `fips_artifacts` in `.github/workflows/test-rust-core.yml` does this for the ODBC library and the Node.js addon after `check_linked_crypto.py` passes.

The bundle is a GitHub Actions candidate. It hashes the unstripped library this job inspected. Release Engineering later signs and notarizes ODBC installers, and GPG-signs JDBC during publication, which changes customer bytes. A candidate attestation does not cover those published files. Python wheels are the exception today: Jenkins publishes the candidate bytes unchanged and adds a detached signature.

## Bundle

| File | Contents |
| --- | --- |
| `sbom.cdx.json` | CycloneDX 1.5 for the bridge crate, with `--features fips` and without `--all-features`. `aws-lc-fips-sys` stays at the exact `sf_core` pin. An `AWS-LC FIPS` component carries the runtime version from `EXPECTED_FIPS_MODULE_VERSION`. `aws-lc-sys` stays in the BOM when Cargo resolves it, with `snowflake:aws-lc-sys` saying why it is not linkage evidence. |
| `linkage.txt` | `check_linked_crypto.py` output for the same file. |
| `dumpbin.txt`, `linker-map.txt` | Windows only. |
| `hashes.json` | SHA-256 of the native file, the SBOM, and the FIPS module that file loads. Windows records the DLL named by `dumpbin /dependents`. macOS records the `libaws_lc_fips_*_crypto.dylib` named by `otool -L`. Each companion includes the path of that file. Linux links the module into the library, so it has no companion. |
| `toolchain.json` | `rustc -vV`, Cargo, `cargo-cyclonedx`, and `CC`/`CXX` when set. |
| `manifest.json` | `stage` is `gha-candidate` and `covers_releng_published_bytes` is false. |

On macOS and Windows that module is the `aws-lc-fips-sys` build-script output under `target/<triple>/release/build` when the build did not copy it beside the library. `hashes.json` records that path. An older cached copy of the same filename stays out.

`ring`, `openssl`, `openssl-sys`, `openssl-src`, `openssl-macros`, and `native-tls` fail the SBOM check. The packaged status API is withheld, so `self_check.status` is `not-run`.

Outside pull requests, the job also runs `actions/attest@v4` twice per library: SLSA provenance, then an SBOM attestation whose subject is that same candidate file. `actions/attest@v4` has to be on the snowflakedb org allowlist before a public workflow runs it:

https://github.com/snowflakedb/self-serve-snowflakedb/blob/26ecbd5af4827db3e40fe9cdbad9777d7abd3e35/configs/snowflakedb/default/default_actions.yml

## Package bundle

`package_evidence.py` writes a bundle for one finished package file (`.rpm`, `.deb`, `.tar.gz`, `.pkg`, or `.msi`). The subject is that package file. `sbom.cdx.json` lists the files the package installs, and `manifest.json` sets `stage` to `gha-package-candidate` with `covers_releng_published_bytes` false. The package jobs do not call this script yet.

## Other packages

Python, JDBC, and .NET release workflows do not build FIPS packages yet. A FIPS lane calls `.github/actions/fips-release-evidence` with the bridge `Cargo.toml`, the same features and target as the build, and the unstripped library before any byte-changing sign or strip step. npm has no lockfile in this repo, so the Node bundle does not include an npm dependency SBOM.
